use anyhow::Result;
use lopdf::{content::Content, Document, Encoding, Object, ObjectId};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// PDF 最大页数限制（防止超大 PDF 卡死）
const MAX_PAGES: usize = 500;
/// 整体超时（秒）—— 5 分钟
const TOTAL_TIMEOUT_SECS: u64 = 300;

/// 全局取消标志（供 cancel_pdf_import 命令触发）
static ACTIVE_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// 注册当前活跃的取消标志
pub fn set_active_cancel(flag: Arc<AtomicBool>) {
    if let Ok(mut guard) = ACTIVE_CANCEL.lock() {
        *guard = Some(flag);
    }
}

/// 清除活跃的取消标志
pub fn clear_active_cancel() {
    if let Ok(mut guard) = ACTIVE_CANCEL.lock() {
        *guard = None;
    }
}

/// 触发取消（供 cancel_pdf_import 命令调用）
pub fn trigger_cancel() {
    if let Ok(mut guard) = ACTIVE_CANCEL.lock() {
        if let Some(flag) = guard.take() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

/// PDF 解析进度回调
/// 参数：(已处理页数, 总页数, 当前页号, 是否成功)
pub type ProgressCb = Box<dyn Fn(usize, usize, u32, bool) + Send + Sync>;

/// 把 PDF 每页文本提取出来，按 docx 兼容格式组装成 HTML：
///   每个非空行包成 `<p>...</p>`，与 mammoth 输出结构一致。
///
/// PDF 性能与稳定性修复（v0.1.7）：
/// - 缓存页表并按 page_id 提取，避免每页重复扫描整棵 page tree
/// - 不再为每页创建后台线程，避免复杂 PDF 超时后堆积不可停止任务
/// - cancel_flag 支持外部取消
/// - 逐页进度回调
/// - 最大页数限制 MAX_PAGES
pub fn pdf_to_html(
    path: &str,
    cancel: Arc<AtomicBool>,
    progress: Option<ProgressCb>,
) -> Result<String> {
    let doc = Document::load(path)?;
    if doc.is_encrypted() {
        return Err(anyhow::anyhow!(
            "PDF 已加密，无法直接导入。请先解除密码或另存为未加密 PDF"
        ));
    }
    let pages = doc.get_pages();
    let total = pages.len().min(MAX_PAGES);
    let mut out = String::new();
    let start = Instant::now();

    for (i, (page_num, page_id)) in pages.iter().take(MAX_PAGES).enumerate() {
        // 外部取消检查
        if cancel.load(Ordering::Relaxed) {
            eprintln!("[PDF] 收到取消信号，已处理 {}/{} 页", i, total);
            break;
        }
        // 整体超时检查
        if start.elapsed() > Duration::from_secs(TOTAL_TIMEOUT_SECS) {
            eprintln!(
                "[PDF] 整体超时 {} 秒，已处理 {}/{} 页",
                TOTAL_TIMEOUT_SECS, i, total
            );
            break;
        }

        if i > 0 {
            out.push_str("<p></p>\n");
        }

        let page_text = match extract_page_text_by_id(&doc, *page_id) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("[PDF] 第 {} 页提取错误: {}", page_num, e);
                None
            }
        };

        let success = page_text.is_some();
        if let Some(text) = page_text {
            for paragraph in pdf_text_to_paragraphs(&text) {
                let safe = html_escape(&paragraph);
                out.push_str(&format!("<p>{}</p>\n", safe));
            }
        }

        // 进度回调
        if let Some(cb) = &progress {
            cb(i + 1, total, *page_num, success);
        }
    }

    if out.is_empty() {
        return Err(anyhow::anyhow!(
            "PDF 所有页提取均失败，可能是扫描件、加密 PDF 或文件过大。建议：1) 用 Word/WPS 打开 PDF 另存为 .docx；2) 或复制内容到 .txt 文件后导入"
        ));
    }
    Ok(out)
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn pdf_text_to_paragraphs(text: &str) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum ParagraphKind {
        Stem,
        Option,
        Answer,
    }

    fn flush(out: &mut Vec<String>, current: &mut String) {
        let text = current.trim();
        if !text.is_empty() {
            out.push(text.to_string());
        }
        current.clear();
    }

    // 预处理：在章节标题、小节标题、答案标题前插入换行
    // lopdf 经常把多个段落合并到一行，导致结构识别失败
    let text = split_by_structural_keywords(text);
    let lines = normalize_pdf_lines(&text);
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut current_kind: Option<ParagraphKind> = None;

    for line in lines {
        if let Some((num, rest)) = parse_question_marker(&line) {
            flush(&mut paragraphs, &mut current);
            current.push_str(&format!("{num}. {rest}"));
            current_kind = Some(ParagraphKind::Stem);
            continue;
        }

        if let Some((label, rest)) = parse_option_marker(&line) {
            flush(&mut paragraphs, &mut current);
            current.push_str(&format!("{label}. {rest}"));
            current_kind = Some(ParagraphKind::Option);
            continue;
        }

        if is_answer_line(&line) {
            flush(&mut paragraphs, &mut current);
            current.push_str(&line);
            current_kind = Some(ParagraphKind::Answer);
            continue;
        }

        // 结构化标题（章节/小节标题）：强制断行，避免被合并到答案或题目段落中
        // 这是 PDF 导入的关键修复：lopdf 常把 "三、判断 1-5 对对对 第一章... 一、单选 1-5 DDBAC"
        // 这种混合内容塞到一行，split_by_structural_keywords 已在文本层面拆分，
        // 这里确保拆分后的标题行不会被段落合并逻辑重新吞回
        if is_structural_header(&line) {
            flush(&mut paragraphs, &mut current);
            paragraphs.push(line);
            // 在答案区间内，保持 Answer 状态，让后续答案行能正确合并到新段落
            // 否则 "1-5" "CDBAC" 等答案行会变成独立段落，无法被 extract_answers_with_section 处理
            if current_kind != Some(ParagraphKind::Answer) {
                current_kind = None;
            }
            continue;
        }

        match current_kind {
            Some(ParagraphKind::Stem) => append_pdf_fragment(&mut current, &line, true),
            Some(ParagraphKind::Option) => append_pdf_fragment(
                &mut current,
                line.trim_start_matches(|c| is_option_separator(c)).trim(),
                true,
            ),
            Some(ParagraphKind::Answer) => append_pdf_fragment(&mut current, &line, false),
            None => paragraphs.push(line),
        }
    }

    flush(&mut paragraphs, &mut current);
    paragraphs
}

/// 预处理：在章节标题、小节标题、答案标题前插入换行
/// lopdf 经常把多个段落合并到一行（如 "11.AC 12.AB 三、判断 1-5 对对对 第一章... 一、单选 1-5 DDBAC"）
/// 这会导致 structure.rs 的章节/小节跟踪失败，答案区间识别混乱
fn split_by_structural_keywords(text: &str) -> String {
    let mut result = text.to_string();

    // 在章节标题前插入换行（支持"第一章"到"第二十章"+"导论"+"绪论"）
    const CHAPTERS: &[&str] = &[
        "第一章", "第二章", "第三章", "第四章", "第五章",
        "第六章", "第七章", "第八章", "第九章", "第十章",
        "第十一章", "第十二章", "第十三章", "第十四章", "第十五章",
        "第十六章", "第十七章", "第十八章", "第十九章", "第二十章",
        "导论", "绪论",
    ];
    for ch in CHAPTERS {
        result = result.replace(ch, &format!("\n{}", ch));
    }

    // 在小节标题前插入换行
    const SECTIONS: &[&str] = &[
        "一、单选", "二、多选", "三、判断",
        "一、单项选择", "二、多项选择", "三、判断题",
        "一、单选题", "二、多选题", "三、判断题",
    ];
    for sec in SECTIONS {
        result = result.replace(sec, &format!("\n{}", sec));
    }
    // 用正则处理带空格的小节标题（lopdf 可能在 "二、" 和 "多选题" 之间插入空格）
    // 如 "二、 多选题" → "\n二、多选题"
    // 同时在标题后插入换行，避免答案内容被合并到标题行
    if let Ok(re_section_spaced) = regex::Regex::new(r"([一二三])\s*、\s*(单选|多选|判断)(题|项选择题)?") {
        result = re_section_spaced.replace_all(&result, "\n$1、$2$3\n").to_string();
    }

    // 在答案标题前插入换行
    const ANSWER_HEADERS: &[&str] = &["参考答案", "标准答案", "正确答案"];
    for hdr in ANSWER_HEADERS {
        result = result.replace(hdr, &format!("\n{}", hdr));
    }

    // 合并被 lopdf 拆开的 range end："11 - 1 5 DBCBB" → "11-15 DBCBB"
    // lopdf 有时把 "11-15" 拆成 "11 - 1 5"（在 end 的数字间插入空格）
    // 仅在 "数字 dash 数字 空格 数字" 后面跟着答案字母/判断词时合并，
    // 避免误伤 "1-5 6-10"（range+range）或 "1-5 6.B"（range+individual）格式
    // 循环替换处理 end 是多位数的情况（如 "1 - 1 2 3 D" → "1-123 D"）
    if let Ok(re_split_range_end) = regex::Regex::new(r"(\d+)\s*[-—–~～]+\s*(\d)\s+(\d)(\s*[A-Da-d对错√×])") {
        loop {
            let new_result = re_split_range_end.replace_all(&result, "$1-$2$3$4").to_string();
            if new_result == result {
                break;
            }
            result = new_result;
        }
    }

    // 合并被 lopdf 拆到多行的 range："1\n——\n5 BADAC" → "1——5 BADAC"
    // lopdf 经常把 "1——5" 拆成三行："1" / "——" / "5 BADAC 6——10 BBDCC"
    // 此处把跨行的 "数字\n dash \n 数字" 合并为 "数字dash数字"
    // 循环替换处理一行内有多个 range 被拆开的情况
    if let Ok(re_cross_line_range) = regex::Regex::new(r"(\d+)\s*\n\s*([-—–~～]+)\s*\n\s*(\d+)") {
        loop {
            let new_result = re_cross_line_range.replace_all(&result, "$1$2$3").to_string();
            if new_result == result {
                break;
            }
            result = new_result;
        }
    }

    result
}

fn normalize_pdf_lines(text: &str) -> Vec<String> {
    let raw: Vec<String> = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    let mut lines = Vec::with_capacity(raw.len());
    let mut i = 0;

    while i < raw.len() {
        let line = raw[i].trim();

        if is_digits(line)
            && raw
                .get(i + 1)
                .is_some_and(|next| starts_with_marker_separator(next.trim()))
        {
            lines.push(format!("{}{}", line, raw[i + 1].trim()));
            i += 2;
            continue;
        }

        if is_digits(line) && raw.get(i + 1).is_some_and(|next| is_dash(next.trim())) {
            if let Some(end) = raw.get(i + 2) {
                lines.push(format!("{}-{}", line, end.trim()));
                i += 3;
                continue;
            }
        }

        if is_option_label_only(line)
            && raw
                .get(i + 1)
                .is_some_and(|next| is_marker_separator(next.trim()))
        {
            lines.push(format!("{}.", line));
            i += 2;
            continue;
        }

        // 合并 range 行 + 纯答案行：lopdf 有时把 "1-5" 和 "CDBAC" 拆成两行
        if is_range_only(line)
            && raw
                .get(i + 1)
                .is_some_and(|next| is_pure_answer_text(next.trim()))
        {
            lines.push(format!("{} {}", line, raw[i + 1].trim()));
            i += 2;
            continue;
        }

        lines.push(line.to_string());
        i += 1;
    }

    lines
        .into_iter()
        .flat_map(|line| split_embedded_question_markers(&line))
        .collect()
}

fn split_embedded_question_markers(line: &str) -> Vec<String> {
    let mut starts = vec![0usize];
    let mut iter = line.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        if idx == 0 || !ch.is_ascii_digit() {
            continue;
        }

        let prefix = line[..idx].trim_end();
        if !ends_like_question_boundary(prefix) {
            continue;
        }

        let mut after_digits = idx + ch.len_utf8();
        while let Some((next_idx, next_ch)) = iter.peek().copied() {
            if next_ch.is_ascii_digit() {
                iter.next();
                after_digits = next_idx + next_ch.len_utf8();
            } else {
                break;
            }
        }

        let rest = &line[after_digits..];
        let rest_trimmed = rest.trim_start();
        let Some(marker) = rest_trimmed.chars().next() else {
            continue;
        };
        if is_question_separator(marker) {
            starts.push(idx);
        }
    }

    starts.dedup();
    let mut parts = Vec::with_capacity(starts.len());
    for (pos, start) in starts.iter().enumerate() {
        let end = starts.get(pos + 1).copied().unwrap_or(line.len());
        let part = line[*start..end].trim();
        if !part.is_empty() {
            parts.push(part.to_string());
        }
    }
    parts
}

fn ends_like_question_boundary(s: &str) -> bool {
    s.chars()
        .rev()
        .find(|c| !c.is_whitespace())
        .is_some_and(|c| matches!(c, ')' | '）' | '。' | '？' | '?' | '！' | '!'))
}

fn parse_question_marker(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('(').unwrap_or(trimmed);
    let trimmed = trimmed.strip_prefix('（').unwrap_or(trimmed);
    let digit_len = trimmed
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(idx, c)| idx + c.len_utf8())
        .last()?;
    let (num, rest) = trimmed.split_at(digit_len);
    let rest = rest.trim_start();
    let mut chars = rest.chars();
    let marker = chars.next()?;
    if !is_question_separator(marker) {
        return None;
    }
    if matches!(marker, '.' | '．') && starts_with_short_decimal(chars.as_str()) {
        return None;
    }
    let rest = chars.as_str();
    let rest = if matches!(marker, ')' | '）') {
        rest
    } else {
        rest.trim_start_matches(|c| matches!(c, ')' | '）'))
    };
    Some((num.to_string(), rest.trim().to_string()))
}

fn parse_option_marker(line: &str) -> Option<(char, String)> {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let label = chars.next()?;
    if !matches!(label, 'A'..='D') {
        return None;
    }
    let rest = chars.as_str().trim_start();
    if rest.is_empty() {
        return Some((label, String::new()));
    }
    let mut rest_chars = rest.chars();
    let marker = rest_chars.next()?;
    if !is_option_separator(marker) {
        return None;
    }
    Some((label, rest_chars.as_str().trim().to_string()))
}

fn append_pdf_fragment(current: &mut String, fragment: &str, compact_cjk: bool) {
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return;
    }
    if current.is_empty() {
        current.push_str(fragment);
        return;
    }
    if should_join_without_space(current, fragment, compact_cjk) {
        current.push_str(fragment);
    } else {
        current.push(' ');
        current.push_str(fragment);
    }
}

fn should_join_without_space(current: &str, fragment: &str, compact_cjk: bool) -> bool {
    let first = fragment.chars().next();
    let last = current.chars().rev().find(|c| !c.is_whitespace());

    if matches!(first, Some(',' | '.' | '?' | '!' | ':' | ';' | ')' | '）' | '，' | '。' | '？' | '！' | '：' | '；' | '、')) {
        return true;
    }
    if matches!(last, Some('(' | '（')) {
        return true;
    }
    compact_cjk && first.is_some_and(is_cjk_char) && last.is_some_and(is_cjk_char)
}

fn is_answer_line(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    lower.starts_with("answer")
        || line.starts_with("答案")
        || line.starts_with("正确答案")
        || line.starts_with("参考答案")
        || line.starts_with("标准答案")
        || line.starts_with("【答案")
}

/// 判断是否为结构化标题（章节标题或小节标题）
/// 用于强制断行，避免标题被合并到答案或题目段落中
fn is_structural_header(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }
    // 章节标题："第一章" 到 "第二十章" + "导论" + "绪论"
    if line.starts_with("第") && line.chars().take(10).any(|c| c == '章') {
        return true;
    }
    if line.starts_with("导论") || line.starts_with("绪论") {
        return true;
    }
    // 小节标题："一、单选" / "二、多选" / "三、判断" 及其变体
    if (line.starts_with("一、") || line.starts_with("二、") || line.starts_with("三、"))
        && (line.contains("选") || line.contains("判断"))
    {
        return true;
    }
    false
}

fn is_question_separator(c: char) -> bool {
    matches!(c, '.' | '、' | '．' | ')' | '）')
}

fn is_option_separator(c: char) -> bool {
    matches!(c, '.' | '、' | '．')
}

fn is_marker_separator(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if is_question_separator(c) || is_option_separator(c))
        && chars.next().is_none()
}

fn starts_with_marker_separator(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(marker) = chars.next() else {
        return false;
    };
    if !is_question_separator(marker) && !is_option_separator(marker) {
        return false;
    }
    if matches!(marker, '.' | '．') && chars.next().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }
    true
}

fn starts_with_short_decimal(s: &str) -> bool {
    let digit_count = s
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .take(3)
        .count();
    (1..=2).contains(&digit_count)
}

fn is_dash(s: &str) -> bool {
    // 支持一个或多个连续 dash 字符（lopdf 常把 "——" 作为 range 分隔符输出）
    // 也支持 "至" 字作为区间分隔符
    if s == "至" {
        return true;
    }
    !s.is_empty()
        && s.chars().all(|c| matches!(c, '-' | '—' | '–' | '－' | '~' | '～'))
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn is_option_label_only(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('A'..='D')) && chars.next().is_none()
}

/// 判断是否为纯 range 行（如 "1-5" / "6—10" / "11-15."），不含答案字母
fn is_range_only(s: &str) -> bool {
    let s = s.trim();
    if s.len() > 20 {
        return false;
    }
    // 匹配 "数字 dash 数字" 后跟可选标点
    let mut seen_digit = false;
    let mut seen_dash = false;
    let mut seen_digit_after_dash = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if seen_dash {
                seen_digit_after_dash = true;
            } else {
                seen_digit = true;
            }
        } else if matches!(c, '-' | '—' | '–' | '～' | '~') {
            if !seen_digit {
                return false;
            }
            seen_dash = true;
        } else if matches!(c, '.' | '、' | '．' | ':' | '：') {
            // 允许尾部标点
        } else {
            return false;
        }
    }
    seen_digit && seen_dash && seen_digit_after_dash
}

/// 判断是否为纯答案文本（字母 A-D / 判断词 / 空格 / 分隔符）
/// 要求至少包含一个答案字符（A-D/对/错/√/×），避免单独的 "." 或 "," 被误识别
fn is_pure_answer_text(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    let has_answer_char = s.chars().any(|c| {
        matches!(c, 'A'..='D' | 'a'..='d' | '对' | '错' | '√' | '×')
    });
    if !has_answer_char {
        return false;
    }
    s.chars().all(|c| {
        matches!(c, 'A'..='D' | 'a'..='d' | ' ')
            || c == ',' || c == ','
            || c == '、' || c == '.'
            || c == '对' || c == '错'
            || c == '√' || c == '×'
    })
}

fn is_cjk_char(c: char) -> bool {
    matches!(
        c,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
    )
}

fn extract_page_text_by_id(doc: &Document, page_id: ObjectId) -> Result<String> {
    fn collect_text(text: &mut String, encoding: &Encoding, operands: &[Object]) -> Result<()> {
        for operand in operands {
            match operand {
                Object::String(bytes, _) => {
                    text.push_str(&Document::decode_text(encoding, bytes)?);
                }
                Object::Array(items) => {
                    collect_text(text, encoding, items)?;
                    text.push(' ');
                }
                Object::Integer(i) if *i < -100 => {
                    text.push(' ');
                }
                _ => {}
            }
        }
        Ok(())
    }

    let fonts = doc.get_page_fonts(page_id)?;
    let encodings: BTreeMap<Vec<u8>, Encoding> = fonts
        .into_iter()
        .map(|(name, font)| font.get_font_encoding(doc).map(|encoding| (name, encoding)))
        .collect::<lopdf::Result<BTreeMap<Vec<u8>, Encoding>>>()?;
    let content_data = doc.get_page_content(page_id)?;
    let content = Content::decode(&content_data)?;
    let mut text = String::new();
    let mut current_encoding = None;

    for operation in &content.operations {
        match operation.operator.as_ref() {
            "Tf" => {
                let current_font = operation
                    .operands
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("PDF 字体操作缺少字体参数"))?
                    .as_name()?;
                current_encoding = encodings.get(current_font);
            }
            "Tj" | "TJ" => {
                if let Some(encoding) = current_encoding {
                    collect_text(&mut text, encoding, &operation.operands)?;
                }
            }
            "ET" => {
                if !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            _ => {}
        }
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Object, Stream};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;

    fn sample_pdf_path(page_count: usize) -> PathBuf {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Courier",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });

        let mut kids = Vec::new();
        for i in 1..=page_count {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 12.into()]),
                    Operation::new("Td", vec![40.into(), 780.into()]),
                    Operation::new(
                        "Tj",
                        vec![Object::string_literal(format!("Page {i} question"))],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
            });
            kids.push(page_id.into());
        }

        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => page_count as i64,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let path = std::env::temp_dir().join(format!(
            "shuati-pdf-test-{}-{}.pdf",
            std::process::id(),
            page_count
        ));
        doc.save(&path).unwrap();
        path
    }

    #[test]
    fn extracts_text_from_cached_page_id() {
        let path = sample_pdf_path(2);
        let doc = Document::load(&path).unwrap();
        let pages = doc.get_pages();
        let page_id = *pages.get(&2).unwrap();

        let text = extract_page_text_by_id(&doc, page_id).unwrap();

        assert!(text.contains("Page 2 question"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn pdf_to_html_reports_progress_for_generated_pages() {
        let path = sample_pdf_path(3);
        let progress_seen = Arc::new(Mutex::new(Vec::new()));
        let progress_for_cb = Arc::clone(&progress_seen);

        let html = pdf_to_html(
            path.to_str().unwrap(),
            Arc::new(AtomicBool::new(false)),
            Some(Box::new(move |done, total, page, success| {
                progress_for_cb
                    .lock()
                    .unwrap()
                    .push((done, total, page, success));
            })),
        )
        .unwrap();

        assert!(html.contains("<p>Page 1 question</p>"));
        assert!(html.contains("<p>Page 3 question</p>"));
        assert_eq!(progress_seen.lock().unwrap().len(), 3);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn fragmented_pdf_lines_are_reflowed_into_questions() {
        let text = [
            "一、单项选择题",
            "1.",
            "党的十八大以来，",
            "（",
            "）",
            "加速演进。",
            "A",
            "、社会主义革命",
            "B.",
            "保护主义",
            "C.",
            "百年未有之大变局",
            "D.",
            "普世价值",
            "答案：C",
            "2.",
            "将习近平新时代中国特色社会主义思想载入宪法的会议是（",
            "）。",
            "A.",
            "党的十八大",
            "B.",
            "党的十九大",
            "C.",
            "十八届三中全会",
            "D.",
            "第十三届全国人民代表大会",
            "答案：D",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].options.len(), 4);
        assert_eq!(questions[0].answer.as_deref(), Some("C"));
        assert_eq!(questions[1].answer.as_deref(), Some("D"));
    }

    #[test]
    fn embedded_pdf_question_markers_start_new_questions() {
        let text = [
            "三、判断题",
            "1. 改革是一场深刻革命。（） 2 ．全面深化改革是“四个全面”战略布局中具有突破性和先导性的关键环节。（）",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 2);
        assert!(questions[0].stem.contains("改革是一场深刻革命"));
        assert!(questions[1].stem.contains("全面深化改革"));
    }

    #[test]
    fn split_digit_line_followed_by_marker_and_question_text() {
        let text = [
            "三、判断题",
            "1. 改革是一场深刻革命。（）",
            "2",
            "．全面深化改革是“四个全面”战略布局中具有突破性和先导性的关键环节。（）",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 2);
        assert!(questions[1].stem.contains("全面深化改革"));
    }

    #[test]
    fn split_digit_line_does_not_turn_decimal_into_question() {
        let text = [
            "1. 2021年中国实现了9899万农村贫困人口全部脱贫，832个贫困县全部摘帽，12",
            ".8万个贫困村全部出列，区域性整体贫困得到解决。",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 1);
        assert!(questions[0].stem.contains("12 .8万个") || questions[0].stem.contains("12.8万个"));
    }

    #[test]
    fn decimal_at_line_start_is_not_a_question_number() {
        let text = [
            "1. 加快发展数字经济是我国建设现代化产业体系的重要内容。十年间，中国数字经济规模占国内生产总值比重达到45",
            ".5%。数字经济的快速发展充分表明（）。",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 1);
        assert!(questions[0].stem.contains("45.5%"));
    }

    #[test]
    fn question_stem_starting_with_year_is_still_a_question() {
        let text = [
            "1.2024年，我国取得了很不平凡的发展成绩，经济实力、科技实力、综合国力持续增强。（）",
            "2.2018年11月5日在上海举行中国国际进口博览会。（）",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 2);
        assert!(questions[0].stem.starts_with("2024年"));
        assert!(questions[1].stem.starts_with("2018年"));
    }

    #[test]
    fn question_number_six_followed_by_year_is_normalized() {
        let text = [
            "5. 上一题。",
            "A. 甲",
            "B. 乙",
            "C. 丙",
            "D. 丁",
            "6.2022年3月25日，中共中央、国务院发布《关于加快建设全国统一大市场的意见》。（）",
            "A. 选项一",
            "B. 选项二",
            "C. 选项三",
            "D. 选项四",
            "7.2022年以来，受地缘政治冲突影响，国际大宗商品价格持续高位。（）",
            "A. 选项一",
            "B. 选项二",
            "C. 选项三",
            "D. 选项四",
            "8. 下一题。",
        ]
        .join("\n");

        let paragraphs = pdf_text_to_paragraphs(&text);
        let questions = crate::import::structure::parse_questions(&paragraphs);

        assert_eq!(questions.len(), 4);
        assert!(questions[1].stem.starts_with("2022年3月25日"));
        assert!(questions[2].stem.starts_with("2022年以来"));
        assert_eq!(questions[1].options.len(), 4);
        assert_eq!(questions[2].options.len(), 4);
    }
}
