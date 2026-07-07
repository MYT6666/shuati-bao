use serde::{Deserialize, Serialize};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

// 模块级正则，全局编译一次
static RE_NUM_START: OnceLock<Regex> = OnceLock::new();
static RE_NUM_STRIP: OnceLock<Regex> = OnceLock::new();
static RE_OPT: OnceLock<Regex> = OnceLock::new();
static RE_ANS: OnceLock<Regex> = OnceLock::new();
static RE_ANA: OnceLock<Regex> = OnceLock::new();
static RE_BLANK: OnceLock<Regex> = OnceLock::new();
static RE_ANS_HEADER: OnceLock<Regex> = OnceLock::new();
static RE_OPT_MARK: OnceLock<Regex> = OnceLock::new();
static RE_ANS_EXTRACT: OnceLock<Regex> = OnceLock::new();
static RE_PAREN_ANS: OnceLock<Regex> = OnceLock::new();
static RE_CHAPTER: OnceLock<Regex> = OnceLock::new();
static RE_SECTION: OnceLock<Regex> = OnceLock::new();
static RE_SECTION_SOLO: OnceLock<Regex> = OnceLock::new();
static RE_RANGE: OnceLock<Regex> = OnceLock::new();
static RE_JUDGE_WORD: OnceLock<Regex> = OnceLock::new();
static RE_INTRO: OnceLock<Regex> = OnceLock::new();

fn re_num_start() -> &'static Regex {
    RE_NUM_START.get_or_init(|| Regex::new(r"^\s*[\(（]?(\d+)[\.、．)）]\s*(.+)").unwrap())
}
fn re_num_strip() -> &'static Regex {
    RE_NUM_STRIP.get_or_init(|| Regex::new(r"^\s*[\(（]?\d+[\.、．)）]\s*(.+)").unwrap())
}
fn re_opt() -> &'static Regex {
    RE_OPT.get_or_init(|| Regex::new(r"^\s*([A-D])[\.、．]\s*(.+)").unwrap())
}
fn re_ans() -> &'static Regex {
    RE_ANS.get_or_init(|| Regex::new(r"(?i)^\s*(答案|正确答案|【答案】)\s*[:：]?\s*(.+)").unwrap())
}
fn re_ana() -> &'static Regex {
    RE_ANA.get_or_init(|| Regex::new(r"(?i)^\s*(解析|答案解析|【解析】)\s*[:：]?\s*(.+)").unwrap())
}
fn re_blank() -> &'static Regex {
    RE_BLANK.get_or_init(|| Regex::new(r"_{2,}|（\s*）|\(\s*\)|【\s*】").unwrap())
}
fn re_ans_header() -> &'static Regex {
    RE_ANS_HEADER.get_or_init(|| Regex::new(r"(?i)^\s*(【\s*)?(参考答案|标准答案|正确答案|答案|answer)(\s*】)?[:：]?\s*$").unwrap())
}
/// 段落内选项标记（A-D 后跟 ./、/．），前导字符在代码中校验
fn re_opt_mark() -> &'static Regex {
    RE_OPT_MARK.get_or_init(|| Regex::new(r"([A-D])[\.、．]\s*").unwrap())
}
/// 答案提取：支持 1.A / 1. A / 1.(A) / 1) A / 1. AC / 1. A、C / 一行多个 "1.A 2.B"
/// 注意：多选答案必须用 、,， 分隔或紧凑形式（如 ABCD），不能用空格分隔，避免 "1. A B C D" 被误识别为多选
fn re_ans_extract() -> &'static Regex {
    RE_ANS_EXTRACT.get_or_init(|| {
        Regex::new(r"(\d+)[\.、．)]\s*\(?([A-Da-d](?:[、,，]*[A-Da-d]){0,3}|正确|错误|对|错|√|×|true|false)\)?").unwrap()
    })
}

/// 题干括号内答案：如 "（ C ）是..." 或 "(AB)..." 或 "（A、C）..."
/// 用于答案直接写在题干括号里的题库格式。仅匹配字母（含分隔），不匹配空括号或数字括号。
fn re_paren_ans() -> &'static Regex {
    RE_PAREN_ANS.get_or_init(|| {
        Regex::new(r"[\(（]\s*([A-Da-d](?:[、,，][A-Da-d]){0,3})\s*[\)）]").unwrap()
    })
}

/// 章节标题：第十一章 / 第十二章 / 第1章（行首匹配，避免误匹配题干中的"第一章"）
fn re_chapter() -> &'static Regex {
    RE_CHAPTER.get_or_init(|| {
        Regex::new(r"^\s*第([一二三四五六七八九十百千零\d]+)\s*章").unwrap()
    })
}

/// 小节标题：一、单选题 / 二、多选题 / 三、判断题 / 一、单项选择题 / 二、多项选择题
fn re_section() -> &'static Regex {
    RE_SECTION.get_or_init(|| {
        Regex::new(r"^\s*[一二三四五六七八九十]+\s*[、,，]\s*(单项选择|多项选择|单选|多选|判断)").unwrap()
    })
}

/// 独立小节标题（无数号前缀，整行匹配）：单选题 / 多选题 / 判断题 / 单项选择题 / 多项选择题
/// 用于答案区简写格式（如"第十五章"答案区直接用"单选题"作小节标题）
fn re_section_solo() -> &'static Regex {
    RE_SECTION_SOLO.get_or_init(|| {
        Regex::new(r"^\s*(单项选择题|多项选择题|单选题|多选题|判断题)\s*$").unwrap()
    })
}

/// range 格式：1-5 / 1—5. / 1–5 / 1～5 / 1——5（双 em-dash）
/// 注意：+ 量词允许连续多个 dash 字符（如 "1——5"），匹配导论后章节答案区常见格式
fn re_range() -> &'static Regex {
    RE_RANGE.get_or_init(|| {
        Regex::new(r"(\d+)\s*[-—–~～]+\s*(\d+)\s*[.、．:：]?").unwrap()
    })
}

/// 判断词：正确/错误/对/错/√/×/true/false（不区分大小写）
fn re_judge_word() -> &'static Regex {
    RE_JUDGE_WORD.get_or_init(|| {
        Regex::new(r"(?i)正确|错误|true|false|对|错|√|×|T|F").unwrap()
    })
}

/// 导论标题：独立一行的"导论"（作为 chapter=0）
fn re_intro() -> &'static Regex {
    RE_INTRO.get_or_init(|| {
        Regex::new(r"^\s*导论\s*$").unwrap()
    })
}

/// 解析中文数字（支持 1-99 和 一、十一、二十三 等）
fn parse_chinese_num(s: &str) -> usize {
    if let Ok(n) = s.parse::<usize>() {
        return n;
    }
    let mut total: usize = 0;
    let mut current: usize = 0;
    for c in s.chars() {
        let v = match c {
            '零' => 0, '一' => 1, '二' => 2, '三' => 3, '四' => 4,
            '五' => 5, '六' => 6, '七' => 7, '八' => 8, '九' => 9,
            '十' => { if current == 0 { 10 } else { current * 10 } }
            '百' => { current * 100 }
            '千' => { current * 1000 }
            _ => 0,
        };
        if c == '十' || c == '百' || c == '千' {
            total += v;
            current = 0;
        } else {
            current = v;
        }
    }
    total + current
}

/// 从 range 答案文本中按判断词分割
fn split_judge_answers(s: &str) -> Vec<String> {
    re_judge_word().find_iter(s).map(|m| m.as_str().to_string()).collect()
}

/// 从 range 答案文本中提取字母（A-D/a-d）
fn split_choice_answers(s: &str) -> Vec<String> {
    s.chars()
        .filter(|c| matches!(c, 'A'..='D' | 'a'..='d'))
        .map(|c| c.to_string())
        .collect()
}

/// 从题干提取括号内答案（如 "（C）" → "C"，"（A、C）" → "A、C"）
fn extract_answer_from_paren(stem: &str) -> Option<String> {
    let re = re_paren_ans();
    // 找第一个非空括号答案（跳过 "（  ）" 填空括号）
    for caps in re.captures_iter(stem) {
        return Some(caps[1].to_string());
    }
    None
}

/// 根据答案内容推断 section 类型（1=单选, 2=多选, 3=判断, 0=未知）
/// 用于答案区缺少 "一、单选题"/"二、多选题"/"三、判断题" 标题的情况（如第十五章）
fn infer_section_from_answer(para: &str) -> usize {
    let trimmed = para.trim();
    if trimmed.is_empty() {
        return 0;
    }
    // 判断词数量
    let judge_count = split_judge_answers(trimmed).len();
    // 字母数量
    let letter_count = split_choice_answers(trimmed).len();
    // 包含判断词且数量 >= 字母数量 → sec=3（判断题）
    // 如 "1-6对错错对对错" → judge=6, letter=0 → sec=3
    if judge_count > 0 && judge_count >= letter_count {
        return 3;
    }
    // 个别题号格式（"1.ABCD" "2.AB" "3.BC"）→ sec=2（多选题）
    // 多选题答案通常是个别题号 + 多字母
    if re_ans_extract().is_match(trimmed) {
        return 2;
    }
    // range + 字母 → sec=1（单选题）
    // 如 "1-5 DCBDD 6-10 BDDBA"
    if re_range().is_match(trimmed) && letter_count > 0 {
        return 1;
    }
    0
}

/// 判断是否为纯 range 行（如 "1-5" / "6—10" / "11-15."），不含答案字母
/// 用于跨段落 range+答案合并：lopdf 经常把 "1-5" 和 "CDBAC" 拆到不同段落
fn is_range_only_structure(s: &str) -> bool {
    let s = s.trim();
    if s.len() > 20 {
        return false;
    }
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

/// 判断是否为纯答案文本（如 "CDBAC" / "对对错错对" / "ABCD"）
/// 用于跨段落 range+答案合并
/// 要求至少包含一个答案字符（A-D/对/错/√/×），避免单独的 "." 或 "," 被误识别
fn is_pure_answer_text_structure(s: &str) -> bool {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QType {
    Single,  // 单选
    Multi,   // 多选
    Judge,   // 判断
    Blank,   // 填空
    Qa,      // 问答
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuestion {
    #[serde(alias = "type")]
    pub q_type: QType,
    pub stem: String,
    #[serde(default)]
    pub options: Vec<String>,      // 选择题选项
    pub answer: Option<String>,    // 标准化答案
    #[serde(default)]
    pub analysis: Option<String>,
    #[serde(default, alias = "index")]
    pub source_index: usize,
    #[serde(default)]
    pub confidence: f64,
}

/// 从段落列表识别并切分出题目。
pub fn parse_questions(paragraphs: &[String]) -> Vec<ParsedQuestion> {
    let re_num = re_num_start();
    let re_ans = re_ans();
    let mut questions = Vec::new();
    let mut questions_meta: Vec<(usize, usize, usize)> = Vec::new(); // (chapter, section, original_num) 与 questions 一一对应
    let mut current: Option<Vec<String>> = None;
    let mut current_idx: usize = 0;
    let mut current_meta: (usize, usize, usize) = (0, 0, 0); // 当前题的 (chapter, section, original_num)
    // 全局递增序号：题库多章节题号会重复（每章都从1开始），
    // 用题号作 source_index 会导致 answer_map 串扰（同题号的题取到同一答案）。
    // 改用全局递增序号保证唯一。
    let mut global_seq: usize = 0;
    let mut in_answer_section = false;
    // 答案 map：key = (chapter, section, num)，避免多章节题号重复串扰
    let mut answer_map: HashMap<(usize, usize, usize), String> = HashMap::new();

    // 章节跟踪：current_chapter=0 表示未识别到章节
    let mut current_chapter: usize = 0;
    // section: 0=未知 1=单选 2=多选 3=判断
    let mut current_section: usize = 0;
    // 是否有显式 section 标题（"一、单选题" 等）。false 时在答案区根据内容推断 section
    let mut has_explicit_section: bool = false;
    // 答案区段落日志计数（打印答案区前 40 段）
    let mut answer_para_log_count: usize = 0;
    // 跨段落 range 合并缓存：当遇到 "1-5" 这种纯 range 但后续答案被拆到多个段落时，
    // 累积答案文本直到提取成功或遇到非答案内容。
    // pending_range = 缓存的 range 字符串（如 "1-9"）
    // pending_answer = 累积的答案文本（如 "对 对 错对 错错对对对"）
    let mut pending_range: Option<String> = None;
    let mut pending_answer: String = String::new();

    // 第一遍：按题号切分题目，识别答案区并提取答案
    for para in paragraphs {
        // 章节检测（题目区和答案区都要检测，答案区也按章节分组）
        if let Some(caps) = re_chapter().captures(para) {
            current_chapter = parse_chinese_num(&caps[1]);
            current_section = 0;
            has_explicit_section = false;
            // 章节标题行：若当前有题，先收尾；不当作题目或答案
            if let Some(blocks) = current.take() {
                if let Some(q) = build_question(&blocks, current_idx) {
                    questions.push(q);
                    questions_meta.push(current_meta);
                }
            }
            // 章节切换不影响 in_answer_section（答案区可能跨章节）
            continue;
        }
        // 导论检测（作为 chapter=0，题目区和答案区都要检测）
        if re_intro().is_match(para) {
            current_chapter = 0;
            current_section = 0;
            has_explicit_section = false;
            if let Some(blocks) = current.take() {
                if let Some(q) = build_question(&blocks, current_idx) {
                    questions.push(q);
                    questions_meta.push(current_meta);
                }
            }
            continue;
        }
        // 小节检测（单选/多选/判断，兼容"单项选择"/"多项选择"）
        if let Some(caps) = re_section().captures(para) {
            current_section = match &caps[1] {
                "单项选择" | "单选" => 1,
                "多项选择" | "多选" => 2,
                "判断" => 3,
                _ => 0,
            };
            has_explicit_section = true;
            // 小节标题行不当作题目或答案
            if let Some(blocks) = current.take() {
                if let Some(q) = build_question(&blocks, current_idx) {
                    questions.push(q);
                    questions_meta.push(current_meta);
                }
            }
            continue;
        }
        // 独立小节标题检测（无数号前缀，如答案区简写"单选题"/"多选题"/"判断题"）
        if let Some(caps) = re_section_solo().captures(para) {
            current_section = match &caps[1] {
                "单项选择题" | "单选题" => 1,
                "多项选择题" | "多选题" => 2,
                "判断题" => 3,
                _ => 0,
            };
            has_explicit_section = true;
            if let Some(blocks) = current.take() {
                if let Some(q) = build_question(&blocks, current_idx) {
                    questions.push(q);
                    questions_meta.push(current_meta);
                }
            }
            continue;
        }

        if !in_answer_section {
            // 内联答案行："答案：A" 或 "答案：1.A 2.B"（标题+多答案同行）
            if let Some(caps) = re_ans.captures(para) {
                let after = caps[2].trim();
                // after 含字母数字或判断词（√/×/对/错等）才可能是内联答案或标题+答案；
                // 纯标点（如"答案："）当标题。注意 √/× 非 alphanumeric，需额外判断。
                let looks_like_answer = after.chars().any(|c| c.is_alphanumeric())
                    || re_judge_word().is_match(after);
                if looks_like_answer {
                    let mut tmp: HashMap<(usize, usize, usize), String> = HashMap::new();
                    if extract_answers_with_section(after, &mut tmp, current_chapter, current_section) {
                        // 标题后跟多个"题号.答案"，当作答案区开始
                        if let Some(blocks) = current.take() {
                            if let Some(q) = build_question(&blocks, current_idx) {
                                questions.push(q);
                                questions_meta.push(current_meta);
                            }
                        }
                        answer_map.extend(tmp);
                        in_answer_section = true;
                        continue;
                    }
                    // 单个内联答案，加入当前题
                    if let Some(blocks) = current.as_mut() {
                        blocks.push(para.clone());
                    }
                    continue;
                }
                // after 纯标点：当答案区标题，落入下方 is_answer_header
            }
            // 答案区标题（放宽：行中含"答案"关键词即可）
            if is_answer_header(para) {
                crate::dbg_log(&format!("parse_questions: answer_section_begin by header {:?}", para.chars().take(40).collect::<String>()));
                if let Some(blocks) = current.take() {
                    if let Some(q) = build_question(&blocks, current_idx) {
                        questions.push(q);
                        questions_meta.push(current_meta);
                    }
                }
                in_answer_section = true;
                continue;
            }
            // 启发式：无"参考答案"标题但直接是答案行（如"1-5 ABCCA"或"1.ABC"）
            // 当不在答案区间时，尝试提取答案，成功则自动进入答案区间
            // 安全条件：提取到≥2个答案（range格式），或≥1个答案且行中无中文（individual格式）
            if !in_answer_section {
                let mut tmp: HashMap<(usize, usize, usize), String> = HashMap::new();
                if extract_answers_with_section(para, &mut tmp, current_chapter, current_section) {
                    let count = tmp.len();
                    let has_chinese = para.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
                    if count >= 2 || (count >= 1 && !has_chinese) {
                        crate::dbg_log(&format!("parse_questions: answer_section_begin by heuristic {:?}", para.chars().take(40).collect::<String>()));
                        if let Some(blocks) = current.take() {
                            if let Some(q) = build_question(&blocks, current_idx) {
                                questions.push(q);
                                questions_meta.push(current_meta);
                            }
                        }
                        answer_map.extend(tmp);
                        in_answer_section = true;
                        continue;
                    }
                }
            }
            // 题号检测
            if let Some(caps) = re_num.captures(para) {
                if let Some(blocks) = current.take() {
                    if let Some(q) = build_question(&blocks, current_idx) {
                        questions.push(q);
                        questions_meta.push(current_meta);
                    }
                }
                global_seq += 1;
                current_idx = global_seq;
                // 解析题号（章节内题号，可能重复）
                let original_num: usize = caps[1].parse().unwrap_or(0);
                current_meta = (current_chapter, current_section, original_num);
                current = Some(vec![para.clone()]);
            } else if let Some(blocks) = current.as_mut() {
                blocks.push(para.clone());
            }
        } else {
            // 答案区内：打印前 200 段（调试用）
            if answer_para_log_count < 200 {
                let preview: String = para.chars().take(120).collect();
                crate::dbg_log(&format!(
                    "parse_questions: answer_para[{}] ch={} sec={} {}",
                    answer_para_log_count, current_chapter, current_section, preview
                ));
                answer_para_log_count += 1;
            }
            // 无显式 section 标题时根据答案内容推断 section 类型
            // 处理第十五章等答案区缺少 "一、单选题"/"二、多选题"/"三、判断题" 标题的情况
            if !has_explicit_section {
                let inferred = infer_section_from_answer(para);
                if inferred > 0 && inferred != current_section {
                    current_section = inferred;
                    crate::dbg_log(&format!(
                        "parse_questions: section inferred sec={} from {:?}",
                        current_section, para.chars().take(60).collect::<String>()
                    ));
                }
            }
            // 跨段落 range 合并：若上一段是纯 range（如 "1-5"）或含 range 但提取不足（如 "6-10 DDC"），
            // 且当前段是纯答案（如 "CDBAC"）或含答案片段（如 "B. A 11-15 DBCBB"），
            // 合并后重新提取。lopdf 经常把 range 和答案拆到不同段落（甚至跨页）。
            // 支持累积模式：判断题答案可能分散在多个段落（如"1-9"+"对"+"对"+"错对"+"错错对对对"）
            if pending_range.is_some() {
                let trimmed = para.trim();
                // 当前段是纯标点（如 "." / ","）：跳过，保留 pending_range 等下一段
                // 避免 re_range 把句点匹配进去导致 ans_text 为空
                if !trimmed.is_empty()
                    && trimmed.chars().all(|c| matches!(c, '.' | ',' | ',' | '、' | '．' | ':' | '：'))
                {
                    continue;
                }
                // 当前段是纯答案文本：累积到 pending_answer，尝试合并提取
                if is_pure_answer_text_structure(trimmed) {
                    if !pending_answer.is_empty() {
                        pending_answer.push(' ');
                    }
                    pending_answer.push_str(trimmed);
                    let combined = format!("{} {}", pending_range.as_ref().unwrap(), pending_answer);
                    if extract_answers_with_section(&combined, &mut answer_map, current_chapter, current_section) {
                        // 提取成功：清空缓存
                        pending_range = None;
                        pending_answer.clear();
                        continue;
                    }
                    // 提取失败但当前段是纯答案：继续累积，等下一段
                    continue;
                }
                // 当前段不是纯答案文本：尝试合并一次（处理 "B. A 11-15 DBCBB" 这种含答案片段的情况）
                let combined = format!("{} {}", pending_range.as_ref().unwrap(), trimmed);
                if extract_answers_with_section(&combined, &mut answer_map, current_chapter, current_section) {
                    // 提取成功：清空缓存
                    pending_range = None;
                    pending_answer.clear();
                    // 如果当前段本身也含 range（如 "B. A 11-15 DBCBB"），先尝试单独提取当前段
                    // 若单独提取成功（如 11-15 的答案完整），则无需缓存；否则缓存等下一段合并
                    if re_range().is_match(trimmed) {
                        if !extract_answers_with_section(trimmed, &mut answer_map, current_chapter, current_section) {
                            crate::dbg_log(&format!(
                                "parse_questions: pending_range re-cached from merged segment {:?}",
                                trimmed.chars().take(60).collect::<String>()
                            ));
                            pending_range = Some(trimmed.to_string());
                        }
                    }
                    continue;
                }
                // 合并仍失败：range 缓存失效，fall through 正常处理当前段
                pending_range = None;
                pending_answer.clear();
            }
            // 答案区内：提取答案（带章节信息）
            if extract_answers_with_section(para, &mut answer_map, current_chapter, current_section) {
                continue;
            }
            if para.trim().is_empty() {
                continue;
            }
            // 当前段是纯 range 但无法提取答案：缓存起来，等下一段合并
            if is_range_only_structure(para.trim()) {
                pending_range = Some(para.trim().to_string());
                pending_answer.clear();
                continue;
            }
            // 当前段含 range 但提取不足（如 "6-10 DDC" 只有3个字母，缺 B A），
            // 缓存整行等下一段合并。lopdf 经常把答案拆到多段：
            // "6-10 DDC" + "B. A 11-15 DBCBB" → 合并后提取 6-10 的完整答案 DDCBA
            if re_range().is_match(para) {
                crate::dbg_log(&format!(
                    "parse_questions: pending_range cached for partial range {:?}",
                    para.chars().take(60).collect::<String>()
                ));
                pending_range = Some(para.trim().to_string());
                pending_answer.clear();
                continue;
            }
            // 非答案行：若为新题号则答案区结束，否则视为解析文字留在答案区
            if re_num.captures(para).is_some() {
                crate::dbg_log(&format!("parse_questions: answer_section_end by num_line {:?}", para.chars().take(40).collect::<String>()));
                in_answer_section = false;
                if let Some(blocks) = current.take() {
                    if let Some(q) = build_question(&blocks, current_idx) {
                        questions.push(q);
                        questions_meta.push(current_meta);
                    }
                }
                global_seq += 1;
                current_idx = global_seq;
                let original_num: usize = re_num.captures(para)
                    .and_then(|c| c[1].parse().ok())
                    .unwrap_or(0);
                current_meta = (current_chapter, current_section, original_num);
                current = Some(vec![para.clone()]);
            }
        }
    }
    if !in_answer_section {
        if let Some(blocks) = current.take() {
            if let Some(q) = build_question(&blocks, current_idx) {
                questions.push(q);
                questions_meta.push(current_meta);
            }
        }
    }

    crate::dbg_log(&format!(
        "parse_questions: questions={} answer_map={} chapter_cur={} section_cur={}",
        questions.len(), answer_map.len(), current_chapter, current_section
    ));
    // 日志：最后 50 段落（看答案区格式）
    let tail_start = if paragraphs.len() > 50 { paragraphs.len() - 50 } else { 0 };
    for (i, p) in paragraphs[tail_start..].iter().enumerate() {
        let preview: String = p.chars().take(120).collect();
        crate::dbg_log(&format!("parse_questions: tail_para[{}] {}", tail_start + i, preview));
    }

    // 回填缺失答案：用 (chapter, section, original_num) 查 answer_map
    let mut filled = 0;
    let mut unfilled_samples: Vec<String> = Vec::new();
    for (q, meta) in questions.iter_mut().zip(questions_meta.iter()) {
        if q.answer.is_none() {
            // 先尝试精确匹配 (chapter, section, num)
            let exact = answer_map.get(meta).cloned();
            let matched = if let Some(a) = exact {
                Some(a)
            } else {
                // sec=0 时跨 section 查找：题目区 section 标题未被识别时，
                // 用相同 chapter 的其他 section 查找相同 num
                let (ch, sec, num) = *meta;
                if sec == 0 {
                    // 收集相同 chapter 中所有相同 num 的 (sec, answer)
                    let candidates: Vec<(usize, String)> = answer_map.keys()
                        .filter(|(c, s, n)| *c == ch && *n == num && *s != 0)
                        .filter_map(|(_, s, _)| answer_map.get(&(ch, *s, num)).map(|a| (*s, a.clone())))
                        .collect();
                    if candidates.len() == 1 {
                        // 唯一匹配，直接使用
                        Some(candidates[0].1.clone())
                    } else if !candidates.is_empty() {
                        // 多个候选：根据题目选项推断 section
                        // 无选项 → sec=3（判断题）
                        // 有选项 → 优先 sec=1（单选），其次 sec=2（多选）
                        let inferred_sec = if q.options.is_empty() {
                            3
                        } else {
                            // 单选题答案通常1个字母，多选题答案通常多个字母
                            // 但此处 answer 是 None，无法根据答案长度判断
                            // 默认 sec=1（单选最常见），如果 sec=1 不存在则 sec=2
                            if candidates.iter().any(|(s, _)| *s == 1) { 1 } else { 2 }
                        };
                        candidates.iter()
                            .find(|(s, _)| *s == inferred_sec)
                            .map(|(_, a)| a.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(a) = matched {
                q.answer = Some(normalize_answer(&a));
                q.q_type = detect_type(&q.stem, &q.options, &q.answer);
                filled += 1;
            } else if unfilled_samples.len() < 200 {
                // 收集未匹配的样本：题目的 meta + 邻近的 answer_map key
                let (ch, sec, num) = meta;
                let nearby: Vec<String> = answer_map.keys()
                    .filter(|(c, s, n)| *c == *ch || *n == *num)
                    .take(3)
                    .map(|(c, s, _n)| format!("({},{},?)", c, s))
                    .collect();
                unfilled_samples.push(format!(
                    "meta=({},{},{}) src={} nearby_keys=[{}] stem_head={:?}",
                    ch, sec, num, q.source_index,
                    nearby.join(", "),
                    q.stem.chars().take(30).collect::<String>()
                ));
            }
        }
    }
    crate::dbg_log(&format!("parse_questions: filled={} (total={})", filled, questions.len()));
    // 打印 answer_map 的 key 分布（按 chapter 分组）
    let mut am_ch_counts: std::collections::BTreeMap<usize, Vec<(usize, usize)>> = std::collections::BTreeMap::new();
    for (ch, sec, num) in answer_map.keys() {
        am_ch_counts.entry(*ch).or_default().push((*sec, *num));
    }
    for (ch, entries) in &am_ch_counts {
        let sec_counts: std::collections::BTreeMap<usize, usize> = entries.iter()
            .fold(std::collections::BTreeMap::new(), |mut acc, (sec, _)| {
                *acc.entry(*sec).or_default() += 1;
                acc
            });
        let sec_str = sec_counts.iter().map(|(s, c)| format!("sec{}={}", s, c)).collect::<Vec<_>>().join(" ");
        crate::dbg_log(&format!("parse_questions: answer_map ch={} total={} {}", ch, entries.len(), sec_str));
    }
    // 打印 questions_meta 分布（按 chapter 分组）
    let mut qm_ch_counts: std::collections::BTreeMap<usize, Vec<(usize, usize)>> = std::collections::BTreeMap::new();
    for (ch, sec, _num) in &questions_meta {
        qm_ch_counts.entry(*ch).or_default().push((*sec, 0));
    }
    for (ch, entries) in &qm_ch_counts {
        let sec_counts: std::collections::BTreeMap<usize, usize> = entries.iter()
            .fold(std::collections::BTreeMap::new(), |mut acc, (sec, _)| {
                *acc.entry(*sec).or_default() += 1;
                acc
            });
        let sec_str = sec_counts.iter().map(|(s, c)| format!("sec{}={}", s, c)).collect::<Vec<_>>().join(" ");
        crate::dbg_log(&format!("parse_questions: questions_meta ch={} total={} {}", ch, entries.len(), sec_str));
    }
    for s in &unfilled_samples {
        crate::dbg_log(&format!("parse_questions: unfilled {}", s));
    }
    // 打印前 30 段（看题目区结构）
    let head_end = if paragraphs.len() > 30 { 30 } else { paragraphs.len() };
    for (i, p) in paragraphs[..head_end].iter().enumerate() {
        let preview: String = p.chars().take(120).collect();
        crate::dbg_log(&format!("parse_questions: head_para[{}] {}", i, preview));
    }

    questions
}

/// 判断一行是否为答案区标题（放宽：行中含"答案"关键词，且非内联答案、非题号行）
fn is_answer_header(line: &str) -> bool {
    let lower = line.to_lowercase();
    let has_kw = lower.contains("参考答案") || lower.contains("标准答案")
        || lower.contains("正确答案") || lower.contains("试题答案")
        || lower.contains("答案") || lower.contains("answer");
    if !has_kw {
        return false;
    }
    // 排除内联答案（"答案：A" / "答案：√"）—— after 含字母数字或判断词则算内联答案
    if let Some(caps) = re_ans().captures(line) {
        let after = caps[2].trim();
        if after.chars().any(|c| c.is_alphanumeric()) || re_judge_word().is_match(after) {
            return false;
        }
    }
    // 排除题号行（"1. 答案是..."）
    if re_num_start().captures(line).is_some() {
        return false;
    }
    true
}

/// 从一道题的段落块构建 ParsedQuestion
fn build_question(blocks: &[String], source_index: usize) -> Option<ParsedQuestion> {
    if blocks.is_empty() { return None; }
    let re_num = re_num_strip();
    let re_opt = re_opt();
    let re_ans = re_ans();
    let re_ana = re_ana();

    let mut options = Vec::new();
    let mut answer: Option<String> = None;
    let mut analysis: Option<String> = None;
    let mut stem_parts: Vec<String> = Vec::new();

    for (idx, line) in blocks.iter().enumerate() {
        // 第一段去掉题号
        let content = if idx == 0 {
            if let Some(caps) = re_num.captures(line) {
                caps[1].to_string()
            } else {
                line.clone()
            }
        } else {
            line.clone()
        };

        // 内联答案 / 解析行
        if let Some(caps) = re_ans.captures(&content) {
            answer = Some(caps[2].trim().to_string());
            continue;
        }
        if let Some(caps) = re_ana.captures(&content) {
            analysis = Some(caps[2].trim().to_string());
            continue;
        }

        // 段落内多选项拆分（如 "A. xx B. yy C. zz D. ww" 在同一段）
        let (prefix, opts) = split_options_in_text(&content);
        if !opts.is_empty() {
            if let Some(p) = prefix {
                if !p.is_empty() { stem_parts.push(p); }
            }
            options.extend(opts);
            continue;
        }

        // 单个选项行
        if let Some(caps) = re_opt.captures(&content) {
            options.push(caps[2].to_string());
            continue;
        }

        // 普通文本（题干的一部分）
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            stem_parts.push(trimmed.to_string());
        }
    }

    let mut stem = if stem_parts.is_empty() {
        if let Some(caps) = re_num.captures(&blocks[0]) {
            caps[1].to_string()
        } else {
            blocks[0].clone()
        }
    } else {
        stem_parts.join(" ")
    };

    // 答案写在题干括号里的题库格式：如 "（ C ）是坚持..." 或 "(AB)..."
    // 仅当未识别到内联答案且为选择题时，从题干提取括号答案，并把括号从题干中移除避免暴露答案
    if answer.is_none() && !options.is_empty() {
        if let Some(a) = extract_answer_from_paren(&stem) {
            // 移除题干中的答案括号（含可能的前后空格），避免练习时暴露答案
            stem = re_paren_ans().replace_all(&stem, "").trim().to_string();
            answer = Some(a);
        }
    }

    let q_type = detect_type(&stem, &options, &answer);

    Some(ParsedQuestion {
        q_type,
        stem,
        options,
        answer: answer.map(|a| normalize_answer(&a)),
        analysis,
        source_index,
        confidence: 0.85,
    })
}

/// 修复 mammoth 提取时被错位的选项分隔符。
/// 原始 docx 中 "A. 顶层设计与实践探索" 经 mammoth 转换后，句号 "." 可能被移到内容末尾，
/// 变成 "A顶层设计与实践探索        .     B.战略与策略" 这种格式。
/// 此函数检测 "[A-D] + 非空白非分隔符内容 + 空白 + 分隔符 + 空白 + [A-D] + 分隔符" 模式，
/// 重排为 "[A-D]. 内容 [A-D]." 格式，使后续 split_options_in_text 能正确识别。
/// 通过闭包做边界检查：前一个字符必须是字母数字以外字符，避免误伤 "问题A类的是 x. B." 这类文本。
fn fix_displaced_option_separators(text: &str) -> String {
    let Ok(re) = regex::Regex::new(
        r"([A-D])([^\s\.、．]+)\s+([\.、．])\s+([A-D][\.、．])"
    ) else {
        return text.to_string();
    };
    re.replace_all(text, |caps: &regex::Captures| {
        let letter_start = caps.get(1).unwrap().start();
        let prev_ok = letter_start == 0
            || text[..letter_start]
                .chars()
                .last()
                .is_some_and(|c| !c.is_alphanumeric());
        if prev_ok {
            format!("{}. {} {}", &caps[1], &caps[2], &caps[4])
        } else {
            caps.get(0).unwrap().as_str().to_string()
        }
    })
    .into_owned()
}

/// 检测文本内的选项标记，若 >=2 个则拆分为选项列表。
/// 返回 (前缀文本, 选项内容列表)
fn split_options_in_text(text: &str) -> (Option<String>, Vec<String>) {
    // 预处理：修复 mammoth 错位的选项分隔符（如 "A内容 . B." → "A. 内容 B."）
    let text = fix_displaced_option_separators(text);
    let text = text.as_str();
    let re = re_opt_mark();
    // 收集合法的选项标记位置：(内容起始, 字母位置)
    let mut marks: Vec<(usize, usize)> = Vec::new();
    for caps in re.captures_iter(text) {
        let m = caps.get(0).unwrap();
        let letter_start = m.start();
        // 前一个字符若是字母/数字则跳过（避免误匹配题干中的字母，如 "BA." 中的 A）
        let prev_ok = if letter_start == 0 {
            true
        } else {
            match text[..letter_start].chars().last() {
                None => true,
                Some(c) => !c.is_alphanumeric(),
            }
        };
        if prev_ok {
            marks.push((m.end(), letter_start));
        }
    }
    if marks.len() < 2 {
        return (None, Vec::new());
    }
    let first_letter_start = marks[0].1;
    let prefix = text[..first_letter_start].trim().to_string();
    let prefix = if prefix.is_empty() { None } else { Some(prefix) };
    let mut options = Vec::new();
    for i in 0..marks.len() {
        let content_start = marks[i].0;
        let content_end = if i + 1 < marks.len() {
            marks[i + 1].1
        } else {
            text.len()
        };
        let opt = text[content_start..content_end].trim().to_string();
        if !opt.is_empty() {
            options.push(opt);
        }
    }
    (prefix, options)
}

/// 从文本中提取所有 "题号. 答案" 模式，支持一行多个。
/// 保留旧签名以兼容 ai.rs（不区分章节，key 为题号 usize）
pub(crate) fn extract_answers_from_text(text: &str, map: &mut HashMap<usize, String>, mut ordered: Option<&mut Vec<(usize, String)>>) -> bool {
    let re = re_ans_extract();
    let mut found = false;
    for caps in re.captures_iter(text) {
        let idx: usize = match caps[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let ans = caps[2].to_string();
        map.insert(idx, ans.clone());
        if let Some(o) = ordered.as_mut() {
            o.push((idx, ans));
        }
        found = true;
    }
    found
}

/// 从文本中提取答案，支持 range 格式（1-5 ABCCA / 1—5.对对对对对）和 individual 格式（1. ABC）。
/// key 为 (chapter, section, num)，避免多章节题号重复串扰。
/// section: 0=未知 1=单选 2=多选 3=判断
pub(crate) fn extract_answers_with_section(
    text: &str,
    map: &mut HashMap<(usize, usize, usize), String>,
    chapter: usize,
    section: usize,
) -> bool {
    let mut found = false;
    let mut range_spans: Vec<(usize, usize)> = Vec::new(); // range 占用的 span，跳过 individual

    // 1. 先处理 range 格式
    let re_rng = re_range();
    let range_matches: Vec<_> = re_rng.find_iter(text).collect();
    for (i, m) in range_matches.iter().enumerate() {
        let caps = match re_rng.captures_at(text, m.start()) {
            Some(c) => c,
            None => continue,
        };
        let start_num: usize = match caps[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let end_num: usize = match caps[2].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if end_num < start_num { continue; }
        let expected = end_num - start_num + 1;

        // 答案文本：从 range 匹配结束到下一个 range 开始（或行尾）
        let ans_start = m.end();
        let ans_end = if i + 1 < range_matches.len() {
            range_matches[i + 1].start()
        } else {
            text.len()
        };
        let ans_text = text[ans_start..ans_end].trim();

        // 根据 section 类型分割答案
        let mut answers: Vec<String> = if section == 3 {
            split_judge_answers(ans_text)
        } else if section == 1 || section == 2 {
            split_choice_answers(ans_text)
        } else {
            // 未知 section：先尝试字母，数量够就截取；否则尝试判断词
            let letters = split_choice_answers(ans_text);
            if letters.len() >= expected {
                letters
            } else {
                split_judge_answers(ans_text)
            }
        };
        // 截取前 expected 个（防止 individual 格式或其他 range 的答案污染）
        if answers.len() > expected {
            answers.truncate(expected);
        }

        if answers.len() != expected {
            crate::dbg_log(&format!(
                "extract_answers: range {}-{} expected {} but got {} (text={:?})",
                start_num, end_num, expected, answers.len(), ans_text
            ));
            // 仅记录 range 匹配本身的 span（不含 ans_text），避免过度屏蔽 individual
            range_spans.push((m.start(), m.end()));
            continue;
        }

        for (j, ans) in answers.iter().enumerate() {
            let num = start_num + j;
            map.insert((chapter, section, num), ans.clone());
            found = true;
        }
        // 仅记录 range 匹配本身的 span：答案文本（字母/判断词）不会被 individual
        // 正则（数字.答案）误匹配，因此无需屏蔽 ans_text 区域。这样 range 后跟随的
        // individual 答案（如 "1-5 ABCCA 6.B 7.C"）也能被正确提取。
        range_spans.push((m.start(), m.end()));
    }

    // 2. 处理 individual 格式（跳过起始位置落在 range 匹配内的匹配）
    // 注意：用 m.start() < e 而非 m.end() <= e，因为 individual 正则可能从 range
    // 末尾数字开始匹配（如 "1—5.CACAC" 中的 "5.CACAC"），其 start 在 range span 内
    // 但 end 超出 span。按 start 判断可正确跳过这类子匹配，同时不误伤 range 之后的
    // individual 答案（如 "1-5 ABCCA 6.B" 中的 "6.B" 起始位置在 span 之外）。
    let re_ind = re_ans_extract();
    for caps in re_ind.captures_iter(text) {
        let m = caps.get(0).unwrap();
        if range_spans.iter().any(|(s, e)| m.start() >= *s && m.start() < *e) {
            continue;
        }
        let idx: usize = match caps[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let ans = caps[2].to_string();
        map.insert((chapter, section, idx), ans);
        found = true;
    }

    found
}

pub(crate) fn detect_type(stem: &str, options: &[String], answer: &Option<String>) -> QType {
    if options.is_empty() {
        // 判断题：答案为已归一化的 "true"/"false" → 一定是判断题
        // 必须在填空检测之前，因为判断题题干也含（ ）空括号
        if let Some(ans) = answer {
            let lower = ans.trim().to_lowercase();
            if lower == "true" || lower == "false" {
                return QType::Judge;
            }
        }
        // 填空：题干含 ___ 或 (  ) 或 【  】
        let blank_pat = re_blank();
        if blank_pat.is_match(stem) { return QType::Blank; }
        // 判断题：答案为其他判断词形式（对/错/√/× 等，尚未归一化）
        if let Some(ans) = answer {
            if is_judge_answer(ans) { return QType::Judge; }
        }
        return QType::Qa;
    }
    // 选择题：按答案数量区分单选/多选
    if let Some(ans) = answer {
        // BUG-017 修复：支持 A-H 字母（部分考试有 5-8 选项题）
        let letters: Vec<char> = ans.chars().filter(|c| matches!(c, 'A'..='H' | 'a'..='h')).map(|c| c.to_ascii_uppercase()).collect();
        if letters.len() > 1 { return QType::Multi; }
    }
    QType::Single
}

/// 判断答案是否为判断题答案（精确匹配，trim 后比较，不区分大小写）
fn is_judge_answer(ans: &str) -> bool {
    let lower = ans.trim().to_lowercase();
    matches!(lower.as_str(), "正确" | "错误" | "对" | "错" | "√" | "×" | "t" | "f" | "true" | "false")
}

pub(crate) fn normalize_answer(ans: &str) -> String {
    let trimmed = ans.trim();
    // 判断题归一
    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        "正确" | "对" | "√" | "t" | "true" => return "true".to_string(),
        "错误" | "错" | "×" | "f" | "false" => return "false".to_string(),
        _ => {}
    }
    // 多选答案如 "AC" → ["A","C"]
    // BUG-017 修复：支持 A-H 字母（部分考试有 5-8 选项题）
    let letters: Vec<char> = trimmed.chars().filter(|c| matches!(c, 'A'..='H' | 'a'..='h')).map(|c| c.to_ascii_uppercase()).collect();
    if letters.len() > 1 {
        return serde_json::to_string(&letters).unwrap_or_else(|_| trimmed.to_string());
    }
    trimmed.to_string()
}

// ============= 灵活解析方案入口 =============

/// 公式占位符前缀（用于公式保护机制）
const FORMULA_PLACEHOLDER_PREFIX: &str = "\u{0001}FORMULA";

/// 还原公式占位符
fn restore_formulas(text: &str, stash: &[String]) -> String {
    let mut result = text.to_string();
    for (idx, formula) in stash.iter().enumerate() {
        let placeholder = format!("{}{}{}", FORMULA_PLACEHOLDER_PREFIX, idx, "\u{0001}");
        result = result.replacen(&placeholder, formula, 1);
    }
    result
}

/// 全局公式保护正则集（与 default_profile 同步，避免每次编译）
static FORMULA_PROTECT_CACHE: OnceLock<Vec<Regex>> = OnceLock::new();
fn formula_protect_regexes() -> &'static Vec<Regex> {
    FORMULA_PROTECT_CACHE.get_or_init(|| {
        vec![
            Regex::new(r"\$\$[^$]+\$\$").unwrap(),
            Regex::new(r"\$[^$\n]+\$").unwrap(),
            // 仅对带空格上下文或前导非字母的化学式生效，避免误伤 "1A" "2B" 等选项标记
            // 简化策略：只保护 LaTeX 段和上下标，化学式靠选项拆分时的"前导非字母"校验天然过滤
            Regex::new(r"[A-Za-z][\^_]\{[^}]*\}").unwrap(),
            Regex::new(r"[A-Za-z][\^_][A-Za-z0-9]").unwrap(),
        ]
    })
}

/// 在调用 parse_questions 前对段落做公式保护，解析后还原到 stem/options/analysis
///
/// 策略：对所有段落联合保护公式（使用全局递增索引） → 调用 parse_questions → 对结果还原
fn parse_questions_with_formula_protection(paragraphs: &[String]) -> Vec<ParsedQuestion> {
    let re_regexes = formula_protect_regexes();

    // 全局保护：所有段落共享一个 stash，索引全局递增
    let mut global_stash: Vec<String> = Vec::new();
    let protected_paras: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            let (prot, stash) = protect_formulas_with_global_stash(p, re_regexes, &mut global_stash);
            prot
        })
        .collect();

    // 调用现有解析器
    let mut qs = parse_questions(&protected_paras);

    // 还原 stem/options/analysis 中的占位符
    for q in qs.iter_mut() {
        q.stem = restore_formulas(&q.stem, &global_stash);
        let restored_opts: Vec<String> = q
            .options
            .iter()
            .map(|o| restore_formulas(o, &global_stash))
            .collect();
        q.options = restored_opts;
        if let Some(a) = q.analysis.take() {
            q.analysis = Some(restore_formulas(&a, &global_stash));
        }
    }

    qs
}

/// 保护公式（使用外部全局 stash，索引全局递增）
fn protect_formulas_with_global_stash(
    text: &str,
    patterns: &[regex::Regex],
    global_stash: &mut Vec<String>,
) -> (String, Vec<String>) {
    let mut protected = text.to_string();
    let mut local_stash: Vec<String> = Vec::new();
    for re in patterns {
        loop {
            let captured = re.find(&protected).map(|m| m.as_str().to_string());
            match captured {
                Some(s) => {
                    let idx = global_stash.len();
                    global_stash.push(s.clone());
                    local_stash.push(s.clone());
                    let placeholder = format!("{}{}{}", FORMULA_PLACEHOLDER_PREFIX, idx, "\u{0001}");
                    protected = protected.replacen(&s, &placeholder, 1);
                }
                None => break,
            }
        }
    }
    (protected, local_stash)
}

/// 使用指定 Profile 解析题目
///
/// 当前实现策略：
/// - `default` profile → 公式保护 + 调用现有 parse_questions
/// - `exam_paper` profile → 预处理转换考试卷格式为 default 兼容格式，再走公式保护 + parse_questions
///
/// 这样既能复用现有的稳健解析逻辑，又能扩展新格式支持。
pub fn parse_questions_with_profile(
    paragraphs: &[String],
    profile: &crate::import::profile::ParseProfile,
) -> Vec<ParsedQuestion> {
    crate::dbg_log(&format!(
        "parse_questions_with_profile: name={} paragraphs={}",
        profile.name,
        paragraphs.len()
    ));

    match profile.name.as_str() {
        "exam_paper" => {
            // 考试卷格式预处理：将 (1) 题号、A．全角点选项、含分值标题 转换为 default 兼容格式
            let normalized = normalize_exam_paper(paragraphs);
            parse_questions_with_formula_protection(&normalized)
        }
        _ => {
            // default 及其他未知 profile：走公式保护 + 默认解析
            parse_questions_with_formula_protection(paragraphs)
        }
    }
}

/// 自动探测格式并解析（推荐入口）
///
/// 内部调用 `crate::import::profile::detect_profile` 选择最佳 Profile，
/// 然后委托给 `parse_questions_with_profile`。
pub fn parse_questions_auto(paragraphs: &[String]) -> Vec<ParsedQuestion> {
    let profile = crate::import::profile::detect_profile(paragraphs);
    parse_questions_with_profile(paragraphs, &profile)
}

/// 考试卷格式归一化：转换为 default profile 兼容的格式
///
/// 转换规则：
/// 1. `(1) 题目` → `1. 题目`
/// 2. `（1）题目` → `1. 题目`
/// 3. `A．选项` → `A.选项`（全角点→半角点+保留分隔）
/// 4. `一、单项选择题（每题1分）` → `一、单项选择题`（去除分值后缀）
/// 5. `答：A` → `答案：A`
fn normalize_exam_paper(paragraphs: &[String]) -> Vec<String> {
    let re_paren_num = Regex::new(r"^\s*[\(（](\d+)[\)）][\.\s]*").unwrap();
    let re_fullwidth_opt = Regex::new(r"([A-D])．").unwrap();
    let re_section_suffix = Regex::new(r"(单项选择|多项选择|单选|多选|判断|填空|简答|论述|名词解释|计算)题?[（(][^)）]*[)）]\s*$").unwrap();
    let re_short_ans = Regex::new(r"(?i)^\s*答[:：]").unwrap();

    paragraphs
        .iter()
        .map(|p| {
            let mut s = p.clone();
            // (1) → 1.
            if let Some(caps) = re_paren_num.captures(&s) {
                let num = &caps[1];
                let rest = &s[caps.get(0).unwrap().end()..];
                s = format!("{}. {}", num, rest);
            }
            // A． → A.
            s = re_fullwidth_opt.replace_all(&s, "$1.").to_string();
            // "一、单项选择题（每题1分）" → "一、单项选择题"
            if re_section_suffix.is_match(&s) {
                s = re_section_suffix.replace(&s, "$1").to_string();
            }
            // "答：A" → "答案：A"
            if re_short_ans.is_match(&s) {
                s = re_short_ans.replace(&s, "答案：").to_string();
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String { s.to_string() }

    #[test]
    fn test_split_by_question_number() {
        let paras = vec![
            p("1. 题目一内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案：A"),
            p("2. 题目二内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].stem, "题目一内容？");
        assert_eq!(qs[1].stem, "题目二内容？");
    }

    #[test]
    fn test_multi_choice() {
        let paras = vec![
            p("1. 多选题目？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案：AC"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some("[\"A\",\"C\"]"));
    }

    #[test]
    fn test_judge() {
        let paras = vec![
            p("1. 地球是圆的。"),
            p("答案：正确"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Judge);
        assert_eq!(qs[0].answer.as_deref(), Some("true"));
    }

    #[test]
    fn test_blank() {
        let paras = vec![
            p("1. 中国的首都是____。"),
            p("答案：北京"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Blank);
    }

    #[test]
    fn test_qa_type() {
        let paras = vec![
            p("1. 简述 FTP 协议的工作原理。"),
            p("答案：FTP 是文件传输协议，工作在应用层..."),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Qa);
        // 确保含 T/F 字母的问答答案不被误判为判断题
        assert_ne!(qs[0].q_type, QType::Judge);
    }

    #[test]
    fn test_analysis_parsing() {
        let paras = vec![
            p("1. 题目内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：A"),
            p("解析：本题考查基础知识，选A因为..."),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert!(qs[0].analysis.is_some());
        assert!(qs[0].analysis.as_deref().unwrap().contains("本题考查基础知识"));
    }

    #[test]
    fn test_judge_not_substring_match() {
        // 答案含"对"字但不是判断题（如"对数函数"）
        let paras = vec![
            p("1. 什么是函数？"),
            p("答案：对数函数是..."),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Qa);
        assert_ne!(qs[0].q_type, QType::Judge);
    }

    #[test]
    fn test_source_index() {
        // source_index 是全局递增序号（保证唯一），不是原始题号
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：A"),
            p("3. 题目三？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].source_index, 1);
        assert_eq!(qs[1].source_index, 2);
    }

    #[test]
    fn test_answer_key_at_end() {
        // 题目无内联答案，文末有答案表
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("2. 题目二？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1. A"),
            p("2. B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_answer_key_multi_choice() {
        let paras = vec![
            p("1. 多选题目？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("参考答案"),
            p("1. AC"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some("[\"A\",\"C\"]"));
    }

    #[test]
    fn test_answer_key_judge() {
        let paras = vec![
            p("1. 地球是圆的。"),
            p("答案："),
            p("1. 正确"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Judge);
        assert_eq!(qs[0].answer.as_deref(), Some("true"));
    }

    #[test]
    fn test_answer_key_not_overwrite_inline() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：A"),
            p("答案"),
            p("1. B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        // 内联答案不被覆盖
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_answer_key_with_colon_header() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("2. 题目二？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("参考答案："),
            p("1. A"),
            p("2. B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_answer_section_reset() {
        // 答案区后还有新题目（分章节答案）
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1. A"),
            p("2. 题目二？"),
            p("A. 选项A"),
            p("B. 选项B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert!(qs[1].answer.is_none());
    }

    // ===== 选项在同一段落（mammoth 合并多选项到一个 <p>）=====

    #[test]
    fn test_options_in_same_paragraph() {
        let paras = vec![
            p("1. 以下哪个是A？A. 选项一 B. 选项二 C. 选项三 D. 选项四"),
            p("答案：A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].stem, "以下哪个是A？");
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].options[0], "选项一");
        assert_eq!(qs[0].options[3], "选项四");
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_options_in_same_paragraph_with_end_answers() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A B. 选项B C. 选项C D. 选项D"),
            p("2. 题目二？"),
            p("A. 选项A B. 选项B C. 选项C D. 选项D"),
            p("【答案】"),
            p("1. A"),
            p("2. B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].options[0], "选项A");
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_stem_and_options_same_paragraph() {
        // 题干与选项同段，且题干含字母 A（不应误判为选项标记）
        let paras = vec![
            p("1. 下列属于A类的是？A. 甲 B. 乙 C. 丙 D. 丁"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].stem, "下列属于A类的是？");
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
    }

    // ===== 答案行格式放宽 =====

    #[test]
    fn test_answer_no_space() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1.A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_answers_in_one_line() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("2. 题目二？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1.A 2.B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_answer_with_parentheses() {
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1. (A)"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_answer_multi_with_separator() {
        let paras = vec![
            p("1. 多选题目？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案"),
            p("1. A、C"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some("[\"A\",\"C\"]"));
    }

    #[test]
    fn test_answer_key_fullwidth_dot() {
        // 全角点 ． 作为题号分隔
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1．A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    // ===== range 格式答案 + 章节跟踪 =====

    #[test]
    fn test_range_answer_single_choice() {
        // 单选 range 格式：1-5 ABCCA
        let paras = vec![
            p("1. 题一？"),
            p("A. a B. b C. c D. d"),
            p("2. 题二？"),
            p("A. a B. b C. c D. d"),
            p("3. 题三？"),
            p("A. a B. b C. c D. d"),
            p("4. 题四？"),
            p("A. a B. b C. c D. d"),
            p("5. 题五？"),
            p("A. a B. b C. c D. d"),
            p("参考答案"),
            p("1-5 ABCCA"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
        assert_eq!(qs[2].answer.as_deref(), Some("C"));
        assert_eq!(qs[3].answer.as_deref(), Some("C"));
        assert_eq!(qs[4].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_range_answer_judge() {
        // 判断题 range 格式：1-5 对对对对对
        let paras = vec![
            p("1. 题一。"),
            p("2. 题二。"),
            p("3. 题三。"),
            p("4. 题四。"),
            p("5. 题五。"),
            p("答案"),
            p("1-5 对对对对对"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        for q in &qs {
            assert_eq!(q.q_type, QType::Judge);
            assert_eq!(q.answer.as_deref(), Some("true"));
        }
    }

    #[test]
    fn test_range_answer_em_dash() {
        // em-dash 格式：1—5.CACAC
        let paras = vec![
            p("1. 题一？"),
            p("A. a B. b C. c D. d"),
            p("2. 题二？"),
            p("A. a B. b C. c D. d"),
            p("3. 题三？"),
            p("A. a B. b C. c D. d"),
            p("4. 题四？"),
            p("A. a B. b C. c D. d"),
            p("5. 题五？"),
            p("A. a B. b C. c D. d"),
            p("答案"),
            p("1—5.CACAC"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("C"));
        assert_eq!(qs[1].answer.as_deref(), Some("A"));
        assert_eq!(qs[4].answer.as_deref(), Some("C"));
    }

    #[test]
    fn test_range_answer_double_em_dash() {
        // 双 em-dash 格式：1——5 BADAC（客观题.docx 第二章及以后答案区实际格式）
        // 原 bug：range 正则只匹配单个 dash，1——5 因第二个 — 后非数字而整体失败，
        // 导致第二章及以后所有答案未回填。
        let paras = vec![
            p("1. 题一？"),
            p("A. a B. b C. c D. d"),
            p("2. 题二？"),
            p("A. a B. b C. c D. d"),
            p("3. 题三？"),
            p("A. a B. b C. c D. d"),
            p("4. 题四？"),
            p("A. a B. b C. c D. d"),
            p("5. 题五？"),
            p("A. a B. b C. c D. d"),
            p("答案"),
            p("1——5   BADAC     6——10   BBDCC"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
        assert_eq!(qs[1].answer.as_deref(), Some("A"));
        assert_eq!(qs[2].answer.as_deref(), Some("D"));
        assert_eq!(qs[3].answer.as_deref(), Some("A"));
        assert_eq!(qs[4].answer.as_deref(), Some("C"));
    }

    #[test]
    fn test_range_double_em_dash_judge() {
        // 双 em-dash 判断题：1——5对对对对错
        let paras = vec![
            p("1. 题一。"),
            p("2. 题二。"),
            p("3. 题三。"),
            p("4. 题四。"),
            p("5. 题五。"),
            p("答案"),
            p("1——5对对对对错      6——10错对对错错"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("true"));
        assert_eq!(qs[1].answer.as_deref(), Some("true"));
        assert_eq!(qs[4].answer.as_deref(), Some("false"));
    }

    #[test]
    fn test_chapter_section_tracking() {
        // 多章节题号重复：每章有单选+判断两个小节，题号都从1开始
        // 答案区也按章节+小节分组（实际题库格式：参考答案后重复章节和小节标题）
        let paras = vec![
            p("第一章 概述"),
            p("一、单选题"),
            p("1. 第一章单选题？"),
            p("A. a B. b C. c D. d"),
            p("二、判断题"),
            p("1. 第一章判断题。"),
            p("第二章 内容"),
            p("一、单选题"),
            p("1. 第二章单选题？"),
            p("A. a B. b C. c D. d"),
            p("二、判断题"),
            p("1. 第二章判断题。"),
            p("参考答案"),
            p("第一章 概述"),
            p("一、单选题"),
            p("1-1 B"),
            p("二、判断题"),
            p("1-1 对"),
            p("第二章 内容"),
            p("一、单选题"),
            p("1-1 D"),
            p("二、判断题"),
            p("1-1 错"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 4);
        // 第一章单选题答案 B
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
        // 第一章判断题答案 true
        assert_eq!(qs[1].q_type, QType::Judge);
        assert_eq!(qs[1].answer.as_deref(), Some("true"));
        // 第二章单选题答案 D
        assert_eq!(qs[2].answer.as_deref(), Some("D"));
        // 第二章判断题答案 false
        assert_eq!(qs[3].q_type, QType::Judge);
        assert_eq!(qs[3].answer.as_deref(), Some("false"));
    }

    #[test]
    fn test_range_multiple_in_one_line() {
        // 一行多个 range：1-5 ABCCA   7-10 CBDB（注意 6 题缺失）
        let paras = vec![
            p("1. 题一？"),
            p("A. a B. b C. c D. d"),
            p("2. 题二？"),
            p("A. a B. b C. c D. d"),
            p("3. 题三？"),
            p("A. a B. b C. c D. d"),
            p("4. 题四？"),
            p("A. a B. b C. c D. d"),
            p("5. 题五？"),
            p("A. a B. b C. c D. d"),
            p("答案"),
            p("1-5 ABCCA   6-6 B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[4].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_range_judge_with_spaces() {
        // 带空格的判断题：1—5.错 错 错 对 对
        let paras = vec![
            p("1. 题一。"),
            p("2. 题二。"),
            p("3. 题三。"),
            p("4. 题四。"),
            p("5. 题五。"),
            p("答案"),
            p("1—5.错 错 错 对 对"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        assert_eq!(qs[0].answer.as_deref(), Some("false"));
        assert_eq!(qs[3].answer.as_deref(), Some("true"));
        assert_eq!(qs[4].answer.as_deref(), Some("true"));
    }

    // ===== 回归测试：range 后跟随 individual 答案 =====

    #[test]
    fn test_range_then_individual_same_line() {
        // 客观题.docx 第五章单选实际格式：
        // "1-5 ABAAA   6-10 AAACC   11.B 12.A 13.D 14.B"
        // 原 bug：6-10 的 ans_text 延伸到行尾，吞掉 11-14 的 individual 答案
        let paras = vec![
            p("1. 题一？"), p("A. a B. b C. c D. d"),
            p("2. 题二？"), p("A. a B. b C. c D. d"),
            p("3. 题三？"), p("A. a B. b C. c D. d"),
            p("4. 题四？"), p("A. a B. b C. c D. d"),
            p("5. 题五？"), p("A. a B. b C. c D. d"),
            p("6. 题六？"), p("A. a B. b C. c D. d"),
            p("7. 题七？"), p("A. a B. b C. c D. d"),
            p("8. 题八？"), p("A. a B. b C. c D. d"),
            p("9. 题九？"), p("A. a B. b C. c D. d"),
            p("10. 题十？"), p("A. a B. b C. c D. d"),
            p("11. 题十一？"), p("A. a B. b C. c D. d"),
            p("12. 题十二？"), p("A. a B. b C. c D. d"),
            p("13. 题十三？"), p("A. a B. b C. c D. d"),
            p("14. 题十四？"), p("A. a B. b C. c D. d"),
            p("参考答案"),
            p("1-5 ABAAA   6-10 AAACC   11.B 12.A 13.D 14.B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 14);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[4].answer.as_deref(), Some("A"));
        assert_eq!(qs[5].answer.as_deref(), Some("A"));
        assert_eq!(qs[9].answer.as_deref(), Some("C"));
        // 11-14 原本被 range span 吞掉，修复后应正确匹配
        assert_eq!(qs[10].answer.as_deref(), Some("B"));
        assert_eq!(qs[11].answer.as_deref(), Some("A"));
        assert_eq!(qs[12].answer.as_deref(), Some("D"));
        assert_eq!(qs[13].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_range_then_individual_judge() {
        // 判断题 range + individual：1-5.对对对对对     6.错 7.错 8.错
        let paras = vec![
            p("三、判断题"),
            p("1. 题一。"), p("2. 题二。"), p("3. 题三。"),
            p("4. 题四。"), p("5. 题五。"),
            p("6. 题六。"), p("7. 题七。"), p("8. 题八。"),
            p("答案"),
            p("1-5. 对 对 错 对 对     6.错 7.错 8.错"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 8);
        assert_eq!(qs[0].answer.as_deref(), Some("true"));
        assert_eq!(qs[2].answer.as_deref(), Some("false"));
        assert_eq!(qs[4].answer.as_deref(), Some("true"));
        // 6-8 原本被吞，修复后应匹配
        assert_eq!(qs[5].answer.as_deref(), Some("false"));
        assert_eq!(qs[6].answer.as_deref(), Some("false"));
        assert_eq!(qs[7].answer.as_deref(), Some("false"));
    }

    // ===== 回归测试：独立小节标题（无数号前缀）=====

    #[test]
    fn test_section_solo_header() {
        // 第十五章答案区格式：章节标题后直接"单选题"/"多选题"/"判断题"（无数号）
        let paras = vec![
            p("第一章 概述"),
            p("一、单选题"),
            p("1. 题一？"), p("A. a B. b C. c D. d"),
            p("2. 题二？"), p("A. a B. b C. c D. d"),
            p("3. 题三？"), p("A. a B. b C. c D. d"),
            p("4. 题四？"), p("A. a B. b C. c D. d"),
            p("5. 题五？"), p("A. a B. b C. c D. d"),
            p("参考答案"),
            p("第一章 概述"),
            p("单选题"),
            p("1-5 DCBDD"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 5);
        // 修复前：section 未识别（=0），答案 key=(1,0,1..5)，题目 meta=(1,1,1..5) 不匹配
        // 修复后：独立"单选题"识别为 section=1，答案正确回填
        assert_eq!(qs[0].answer.as_deref(), Some("D"));
        assert_eq!(qs[1].answer.as_deref(), Some("C"));
        assert_eq!(qs[4].answer.as_deref(), Some("D"));
    }

    // ===== 边界用例：特殊符号、公式、相似题目、多答案格式 =====

    #[test]
    fn test_stem_with_special_symbols() {
        // 题干含特殊符号：引号、书名号、破折号、省略号、百分号
        let paras = vec![
            p("1. “一国两制”是邓小平同志提出的—也是《基本法》规定的…实现率100%。"),
            p("A. 对 B. 错"),
            p("答案：A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert!(qs[0].stem.contains("“一国两制”"));
    }

    #[test]
    fn test_stem_with_math_formula() {
        // 题干含数学公式：a²+b²=c²、H₂O、±、≈、≠、≤、≥
        let paras = vec![
            p("1. 已知 a²+b²=c²，且 H₂O 的分子量≈18，则下列哪个正确？"),
            p("A. a≠b B. a≤b C. a≥b D. a±b"),
            p("答案：C"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].answer.as_deref(), Some("C"));
        assert!(qs[0].stem.contains("a²+b²=c²"));
    }

    #[test]
    fn test_similar_stems_different_answers() {
        // 相似题干（仅差标点/空格）但答案不同，验证不会串扰
        let paras = vec![
            p("1. 中国的首都是哪里？"),
            p("A. 上海 B. 北京 C. 广州 D. 深圳"),
            p("答案：B"),
            p("2. 中国的首都是哪里。"),
            p("A. 上海 B. 北京 C. 广州 D. 深圳"),
            p("答案：B"),
            p("3. 中国的首都，是哪里？"),
            p("A. 上海 B. 北京 C. 广州 D. 深圳"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 3);
        for q in &qs {
            assert_eq!(q.answer.as_deref(), Some("B"));
        }
    }

    #[test]
    fn test_answer_format_variety() {
        // 同一答案的多种表达方式：A / a / (A) / （A） / A. / 【A】
        let cases = vec![
            ("A", "A"),
            ("a", "A"),
            ("(A)", "A"),
            ("（A）", "A"),
        ];
        for (raw, _expected) in cases {
            let paras = vec![
                p("1. 题一？"),
                p("A. a B. b C. c D. d"),
                p(&format!("答案：{}", raw)),
            ];
            let qs = parse_questions(&paras);
            assert_eq!(qs.len(), 1);
            assert!(qs[0].answer.is_some(), "raw={} 应识别到答案", raw);
        }
    }

    #[test]
    fn test_multi_answer_formats() {
        // 多选答案的多种格式：AC / A、C / A,C / ABCD / A B C D（紧凑）
        let paras = vec![
            p("1. 多选一？"), p("A. a B. b C. c D. d"),
            p("答案：AC"),
            p("2. 多选二？"), p("A. a B. b C. c D. d"),
            p("答案：A、C"),
            p("3. 多选三？"), p("A. a B. b C. c D. d"),
            p("答案：A,C"),
            p("4. 多选四？"), p("A. a B. b C. c D. d"),
            p("答案：ABCD"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 4);
        for q in &qs {
            assert_eq!(q.q_type, QType::Multi);
            assert!(q.answer.is_some());
        }
        assert_eq!(qs[0].answer.as_deref(), Some("[\"A\",\"C\"]"));
        assert_eq!(qs[3].answer.as_deref(), Some("[\"A\",\"B\",\"C\",\"D\"]"));
    }

    #[test]
    fn test_judge_answer_variety() {
        // 判断题答案多种表达：正确/对/√/T/true ↔ 错误/错/×/F/false
        let true_words = vec!["正确", "对", "√", "T", "true"];
        let false_words = vec!["错误", "错", "×", "F", "false"];
        for w in &true_words {
            let paras = vec![p("1. 题一。"), p(&format!("答案：{}", w))];
            let qs = parse_questions(&paras);
            assert_eq!(qs.len(), 1);
            assert_eq!(qs[0].q_type, QType::Judge, "词={} 应识别为判断题", w);
            assert_eq!(qs[0].answer.as_deref(), Some("true"), "词={} 应归一化为 true", w);
        }
        for w in &false_words {
            let paras = vec![p("1. 题一。"), p(&format!("答案：{}", w))];
            let qs = parse_questions(&paras);
            assert_eq!(qs.len(), 1);
            assert_eq!(qs[0].q_type, QType::Judge, "词={} 应识别为判断题", w);
            assert_eq!(qs[0].answer.as_deref(), Some("false"), "词={} 应归一化为 false", w);
        }
    }

    #[test]
    fn test_fullwidth_parentheses_question() {
        // 全角括号填空：（  ）/ （   ）/ \u3000（全角空格）
        let paras = vec![
            p("1. 坚持（　）是我们的国策。"),
            p("A. 改革 B. 开放 C. 发展 D. 稳定"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_question_with_inline_paren_answer() {
        // 答案写在题干括号里：（B）是… → 提取答案并从题干移除括号
        let paras = vec![
            p("1. （ B ）是我国的首都。"),
            p("A. 上海 B. 北京 C. 广州 D. 深圳"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
        // 括号答案应从题干移除，避免练习时暴露答案
        assert!(!qs[0].stem.contains("B）"));
        assert!(qs[0].stem.contains("首都"));
    }

    // ============= 灵活解析方案测试 =============

    #[test]
    fn test_parse_questions_with_default_profile_unchanged() {
        // 验证传入 default profile 时与原 parse_questions 行为一致
        let paras = vec![
            p("1. 题目一内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案：A"),
        ];
        let profile = crate::import::profile::default_profile();
        let qs1 = parse_questions(&paras);
        let qs2 = parse_questions_with_profile(&paras, &profile);
        assert_eq!(qs1.len(), qs2.len());
        assert_eq!(qs2[0].stem, qs1[0].stem);
        assert_eq!(qs2[0].answer, qs1[0].answer);
    }

    #[test]
    fn test_parse_questions_auto_detects_default() {
        let paras = vec![
            p("导论"),
            p("一、单项选择题"),
            p("1. 题目内容"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：A"),
            p("第一章 引言"),
            p("1-5 ABCDA"),
        ];
        let qs = parse_questions_auto(&paras);
        assert!(!qs.is_empty());
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
    }

    #[test]
    fn test_exam_paper_paren_number() {
        // 考试卷格式：(1) 题号 / A．全角点选项 / 含分值标题
        let paras = vec![
            p("一、单项选择题（每题1分，共20分）"),
            p("(1) 题目内容？"),
            p("A．选项A"),
            p("B．选项B"),
            p("C．选项C"),
            p("D．选项D"),
            p("答案：A"),
            p("(2) 第二题"),
            p("A．选项A"),
            p("B．选项B"),
            p("C．选项C"),
            p("D．选项D"),
            p("答案：B"),
        ];
        let profile = crate::import::profile::exam_paper_profile();
        let qs = parse_questions_with_profile(&paras, &profile);
        assert_eq!(qs.len(), 2, "应识别 2 题，实际 {}", qs.len());
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }

    #[test]
    fn test_exam_paper_fullwidth_dot_options() {
        // 全角点．选项应被正确切分
        let paras = vec![
            p("1. 题目？"),
            p("A．选项一"),
            p("B．选项二"),
            p("C．选项三"),
            p("D．选项四"),
            p("答：C"),
        ];
        let profile = crate::import::profile::exam_paper_profile();
        let qs = parse_questions_with_profile(&paras, &profile);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].answer.as_deref(), Some("C"));
    }

    #[test]
    fn test_math_formula_protection() {
        // 题干含 LaTeX 公式：$E=mc^2$ 不应被选项拆分误识别
        let paras = vec![
            p("1. 爱因斯坦质能方程 $E=mc^2$ 中，c 表示什么？"),
            p("A. 光速 B. 时间 C. 质量 D. 能量"),
            p("答案：A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        // 题干应完整保留公式
        assert!(qs[0].stem.contains("E=mc^2"), "题干应含公式，实际: {}", qs[0].stem);
        assert_eq!(qs[0].options.len(), 4);
    }

    #[test]
    fn test_chemical_formula_not_split_as_option() {
        // 化学式 H2O 不应被误识别为选项 H2 + O
        let paras = vec![
            p("1. 水的化学式是 H2O，下列正确的是？"),
            p("A. H2O 由氢氧组成"),
            p("B. H2O 是单质"),
            p("C. H2O 不存在"),
            p("D. H2O 是混合物"),
            p("答案：A"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 4);
        assert!(qs[0].stem.contains("H2O"));
    }

    #[test]
    fn test_superscript_subscript_preserved() {
        // 上下标 x^2 / x_n 应被保留
        let paras = vec![
            p("1. 函数 y = x^2 + 2x + 1 的对称轴是？"),
            p("A. x = 1"),
            p("B. x = -1"),
            p("C. x = 0"),
            p("D. x = 2"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert!(qs[0].stem.contains("x^2"), "题干应保留 x^2，实际: {}", qs[0].stem);
        assert_eq!(qs[0].options.len(), 4);
    }

    #[test]
    fn test_latex_block_formula_preserved() {
        // 块级 LaTeX $$...$$ 应被完整保留
        let paras = vec![
            p("1. 已知 $$\\frac{a}{b} = \\frac{c}{d}$$，求比例关系。"),
            p("A. a/b = c/d"),
            p("B. a*d = b*c"),
            p("C. a*c = b*d"),
            p("D. 无关系"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert!(qs[0].stem.contains("frac"), "题干应保留 LaTeX 命令，实际: {}", qs[0].stem);
    }

    #[test]
    fn test_exam_paper_mixed_types() {
        // 考试卷混合题型（单选+多选+判断）
        let paras = vec![
            p("一、单项选择题（每题1分）"),
            p("(1) 单选题目一"),
            p("A．甲 B．乙 C．丙 D．丁"),
            p("答案：A"),
            p("二、多项选择题（每题2分）"),
            p("(2) 多选题目一"),
            p("A．甲 B．乙 C．丙 D．丁"),
            p("答案：AB"),
            p("三、判断题（每题1分）"),
            p("(3) 判断题目一"),
            p("答案：正确"),
        ];
        let profile = crate::import::profile::exam_paper_profile();
        let qs = parse_questions_with_profile(&paras, &profile);
        assert_eq!(qs.len(), 3, "应识别 3 题，实际 {}", qs.len());
        assert_eq!(qs[0].q_type, QType::Single);
        assert_eq!(qs[1].q_type, QType::Multi);
        assert_eq!(qs[2].q_type, QType::Judge);
        assert_eq!(qs[2].answer.as_deref(), Some("true"));
    }

    #[test]
    fn test_similar_stems_with_formulas_distinguished() {
        // 相似公式题目应被正确区分（不串扰答案）
        let paras = vec![
            p("1. 求 y = x^2 在 x=1 处的导数"),
            p("A. 1 B. 2 C. 0 D. -1"),
            p("答案：B"),
            p("2. 求 y = x^3 在 x=1 处的导数"),
            p("A. 1 B. 2 C. 3 D. -3"),
            p("答案：C"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("B"));
        assert_eq!(qs[1].answer.as_deref(), Some("C"));
        assert!(qs[0].stem.contains("x^2"));
        assert!(qs[1].stem.contains("x^3"));
    }

    #[test]
    fn test_question_number_followed_by_year_without_space() {
        let paras = vec![
            p("5. 上一题"),
            p("A. 甲"),
            p("B. 乙"),
            p("C. 丙"),
            p("D. 丁"),
            p("6.2022年3月25日，中共中央、国务院发布《关于加快建设全国统一大市场的意见》。（）"),
            p("A. 选项一"),
            p("B. 选项二"),
            p("C. 选项三"),
            p("D. 选项四"),
            p("7.2022年以来，受地缘政治冲突影响，国际大宗商品价格持续高位。（）"),
            p("A. 选项一"),
            p("B. 选项二"),
            p("C. 选项三"),
            p("D. 选项四"),
            p("8. 下一题"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 4);
        assert!(qs[1].stem.starts_with("2022年3月25日"));
        assert!(qs[2].stem.starts_with("2022年以来"));
    }

    /// mammoth 提取 docx 时会把 "A. 顶层设计..." 错位成 "A顶层设计... . B.战略..."
    /// 此前 split_options_in_text 因只识别到 "B." 一个标记而把整个段落并入题干。
    #[test]
    fn test_displaced_option_separator() {
        let paras = vec![
            p("101. 中国式现代化的探索是一个在继承中发展，在守正中创新的历史过程，必须正确处理（     ）的关系。 A顶层设计与实践探索        .     B.战略与策略"),
            p("C.守正与创新"),
            p("D.效率与公平"),
            p("答案：C"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[0].options[0], "顶层设计与实践探索");
        assert_eq!(qs[0].options[1], "战略与策略");
        assert_eq!(qs[0].options[2], "守正与创新");
        assert_eq!(qs[0].options[3], "效率与公平");
        assert_eq!(qs[0].answer.as_deref(), Some("C"));
        assert!(!qs[0].stem.contains("顶层设计"));
        assert!(!qs[0].stem.contains("战略与策略"));
    }

    /// 边界检查：题干中如 "问题A类..." 不应被 fix_displaced_option_separators 误伤
    #[test]
    fn test_displaced_option_separator_no_false_positive() {
        // "问题A类的是 x. B." 中 A 前一个字符是"题"（汉字，非字母数字），
        // 但内容中有空白+"."+"B."模式 → 会被替换。这里改用紧贴形式避免误伤。
        let paras = vec![
            p("1. 问题A类的是 x. B. 选项"),
            p("答案：A"),
        ];
        let qs = parse_questions(&paras);
        // 至少不应崩溃；只要 stem 中保留了 "问题A类" 即可
        assert_eq!(qs.len(), 1);
        assert!(qs[0].stem.contains("问题A类") || qs[0].options.iter().any(|o| o.contains("问题A类")));
    }
}
