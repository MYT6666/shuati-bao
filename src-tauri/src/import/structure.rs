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

/// range 格式：1-5 / 1—5. / 1–5 / 1～5
fn re_range() -> &'static Regex {
    RE_RANGE.get_or_init(|| {
        Regex::new(r"(\d+)\s*[-—–~～]\s*(\d+)\s*[.、．:：]?").unwrap()
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
    // 答案区段落日志计数（打印答案区前 40 段）
    let mut answer_para_log_count: usize = 0;

    // 第一遍：按题号切分题目，识别答案区并提取答案
    for para in paragraphs {
        // 章节检测（题目区和答案区都要检测，答案区也按章节分组）
        if let Some(caps) = re_chapter().captures(para) {
            current_chapter = parse_chinese_num(&caps[1]);
            current_section = 0;
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
            // 小节标题行不当作题目或答案
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
                // after 含字母数字才可能是内联答案或标题+答案；纯标点（如"答案："）当标题
                if after.chars().any(|c| c.is_alphanumeric()) {
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
            // 答案区内：打印前 40 段（调试用）
            if answer_para_log_count < 40 {
                let preview: String = para.chars().take(120).collect();
                crate::dbg_log(&format!(
                    "parse_questions: answer_para[{}] ch={} sec={} {}",
                    answer_para_log_count, current_chapter, current_section, preview
                ));
                answer_para_log_count += 1;
            }
            // 答案区内：提取答案（带章节信息）
            if extract_answers_with_section(para, &mut answer_map, current_chapter, current_section) {
                continue;
            }
            if para.trim().is_empty() {
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
            if let Some(a) = answer_map.get(meta) {
                q.answer = Some(normalize_answer(a));
                q.q_type = detect_type(&q.stem, &q.options, &q.answer);
                filled += 1;
            } else if unfilled_samples.len() < 10 {
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
    // 排除内联答案（"答案：A"）—— 仅当 after 含字母数字才算内联答案
    if let Some(caps) = re_ans().captures(line) {
        let after = caps[2].trim();
        if after.chars().any(|c| c.is_alphanumeric()) {
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

/// 检测文本内的选项标记，若 >=2 个则拆分为选项列表。
/// 返回 (前缀文本, 选项内容列表)
fn split_options_in_text(text: &str) -> (Option<String>, Vec<String>) {
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
            // 仍记录 span，避免 individual 误匹配 range 内的数字
            range_spans.push((m.start(), ans_end));
            continue;
        }

        for (j, ans) in answers.iter().enumerate() {
            let num = start_num + j;
            map.insert((chapter, section, num), ans.clone());
            found = true;
        }
        range_spans.push((m.start(), ans_end));
    }

    // 2. 处理 individual 格式（跳过 range span 内的匹配）
    let re_ind = re_ans_extract();
    for caps in re_ind.captures_iter(text) {
        let m = caps.get(0).unwrap();
        if range_spans.iter().any(|(s, e)| m.start() >= *s && m.end() <= *e) {
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
        let letters: Vec<char> = ans.chars().filter(|c| matches!(c, 'A'..='D' | 'a'..='d')).map(|c| c.to_ascii_uppercase()).collect();
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
    let letters: Vec<char> = trimmed.chars().filter(|c| matches!(c, 'A'..='D' | 'a'..='d')).map(|c| c.to_ascii_uppercase()).collect();
    if letters.len() > 1 {
        return serde_json::to_string(&letters).unwrap_or_else(|_| trimmed.to_string());
    }
    trimmed.to_string()
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
}
