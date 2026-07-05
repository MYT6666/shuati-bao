//! 灵活解析方案：可配置的解析规则 Profile
//!
//! 通过 `ParseProfile` 封装一类题库排版格式的所有识别规则，
//! 内置多个预设（default / exam_paper），支持自动格式探测与自定义扩展。
//! `parse_questions_with_profile` 据此动态解析，无需为新格式改源码。

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 题型枚举（与 structure.rs QType 保持一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileQType {
    Single,
        Multi,
    Judge,
    Blank,
    Qa,
}

/// 小节标识样式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPattern {
    /// 正则字符串（捕获组 1 为题型关键词）
    pub regex: String,
    /// 是否带 "一、" 数字前缀
    pub with_prefix: bool,
}

/// 答案区策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AnswerSectionStrategy {
    /// 末尾集中答案区（默认）
    Tail,
    /// 题后立即答案
    Inline,
    /// 两者混合
    Hybrid,
    /// 题干括号内答案
    InStem,
}

/// 一套完整的解析规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseProfile {
    pub name: String,
    pub description: String,

    // 1. 题号识别
    /// 题号起始正则（捕获组 1 为题号，组 2 为题干剩余）
    pub num_start: String,
    /// 去题号正则（捕获组 1 为题干）
    pub num_strip: String,

    // 2. 章节 / 导论识别
    pub chapter_patterns: Vec<String>,
    pub intro_patterns: Vec<String>,

    // 3. 小节（题型）标识
    pub section_patterns: Vec<SectionPattern>,
    /// 题型关键词 → section 编号（1=单选 2=多选 3=判断）
    pub section_keywords: Vec<(String, usize)>,

    // 4. 选项格式
    /// 单行选项正则（捕获组 1 为字母，组 2 为选项内容）
    pub option_line: String,
    /// 段内选项标记正则（用于一行多选项拆分）
    pub option_inline: String,
    /// 支持的选项字母（默认 A-D）
    pub option_letters: Vec<char>,

    // 5. 答案识别
    /// 内联答案正则（如 "答案：A"）
    pub answer_inline: String,
    /// "题号.答案" 提取正则
    pub answer_extract: String,
    /// "1-5 ABCDA" range 正则
    pub answer_range: String,
    /// 题干括号内答案正则
    pub answer_paren: String,
    /// 答案区标题关键词
    pub answer_keywords: Vec<String>,
    /// 解析关键词
    pub analysis_keywords: Vec<String>,

    // 6. 答案归一化
    pub judge_true_tokens: Vec<String>,
    pub judge_false_tokens: Vec<String>,

    // 7. 填空标记
    pub blank_pattern: String,

    // 8. 答案区策略
    pub answer_section_strategy: AnswerSectionStrategy,

    // 9. 公式保护段（正则匹配到的内容在选项拆分时整体保留）
    pub formula_protect_patterns: Vec<String>,
}

/// 编译后的 Profile（缓存所有正则，避免重复编译）
pub struct CompiledProfile {
    pub num_start: Regex,
    pub num_strip: Regex,
    pub chapter: Vec<Regex>,
    pub intro: Vec<Regex>,
    pub section: Vec<(Regex, bool)>, // (regex, with_prefix)
    pub section_keywords: Vec<(String, usize)>,
    pub option_line: Regex,
    pub option_inline: Regex,
    pub option_letters: Vec<char>,
    pub answer_inline: Regex,
    pub answer_extract: Regex,
    pub answer_range: Regex,
    pub answer_paren: Regex,
    pub answer_keywords: Vec<String>,
    pub analysis_keywords: Vec<Regex>,
    pub judge_true_tokens: Vec<String>,
    pub judge_false_tokens: Vec<String>,
    pub blank: Regex,
    pub answer_section_strategy: AnswerSectionStrategy,
    pub formula_protect: Vec<Regex>,
}

impl ParseProfile {
    /// 编译所有正则为 CompiledProfile
    pub fn compile(&self) -> Result<CompiledProfile, regex::Error> {
        let compile_all = |pats: &[String]| -> Result<Vec<Regex>, regex::Error> {
            pats.iter().map(|p| Regex::new(p)).collect()
        };
        let compile_sec = |pats: &[SectionPattern]| -> Result<Vec<(Regex, bool)>, regex::Error> {
            pats.iter()
                .map(|p| Ok((Regex::new(&p.regex)?, p.with_prefix)))
                .collect()
        };

        Ok(CompiledProfile {
            num_start: Regex::new(&self.num_start)?,
            num_strip: Regex::new(&self.num_strip)?,
            chapter: compile_all(&self.chapter_patterns)?,
            intro: compile_all(&self.intro_patterns)?,
            section: compile_sec(&self.section_patterns)?,
            section_keywords: self.section_keywords.clone(),
            option_line: Regex::new(&self.option_line)?,
            option_inline: Regex::new(&self.option_inline)?,
            option_letters: self.option_letters.clone(),
            answer_inline: Regex::new(&self.answer_inline)?,
            answer_extract: Regex::new(&self.answer_extract)?,
            answer_range: Regex::new(&self.answer_range)?,
            answer_paren: Regex::new(&self.answer_paren)?,
            answer_keywords: self.answer_keywords.clone(),
            analysis_keywords: self.analysis_keywords.iter()
                .map(|k| Regex::new(&format!(r"(?i)^\s*{}\s*[:：]?\s*(.+)", k)))
                .collect::<Result<_, _>>()?,
            judge_true_tokens: self.judge_true_tokens.clone(),
            judge_false_tokens: self.judge_false_tokens.clone(),
            blank: Regex::new(&self.blank_pattern)?,
            answer_section_strategy: self.answer_section_strategy,
            formula_protect: compile_all(&self.formula_protect_patterns)?,
        })
    }

    /// 评分：与格式指纹的匹配度（0-100）
    pub fn match_score(&self, fp: &FormatFingerprint) -> usize {
        let mut score = 0usize;

        // 题号样式（25 分）
        if let Ok(re) = Regex::new(&self.num_start) {
            if re.is_match(&fp.num_sample) {
                score += 25;
            }
        }
        // 选项样式（25 分）
        if let Ok(re) = Regex::new(&self.option_line) {
            if re.is_match(&fp.opt_sample) {
                score += 25;
            }
        }
        // 答案关键词（每命中 +5，上限 15）
        let mut kw_score = 0;
        for kw in &self.answer_keywords {
            if fp.answer_kw_seen.iter().any(|s| s.contains(kw)) {
                kw_score += 5;
            }
        }
        score += kw_score.min(15);
        // 章节样式（每命中 +5，上限 15）
        let mut ch_score = 0;
        for pat in &self.chapter_patterns {
            if let Ok(re) = Regex::new(pat) {
                if re.is_match(&fp.chapter_sample) {
                    ch_score += 5;
                }
            }
        }
        score += ch_score.min(15);
        // 答案区策略（20 分）
        match self.answer_section_strategy {
            AnswerSectionStrategy::Tail => {
                if fp.has_tail_answer { score += 20; }
            }
            AnswerSectionStrategy::Inline => {
                if fp.has_inline_answer { score += 20; }
            }
            AnswerSectionStrategy::Hybrid => {
                if fp.has_tail_answer || fp.has_inline_answer { score += 15; }
            }
            AnswerSectionStrategy::InStem => {
                if fp.has_paren_answer { score += 20; }
            }
        }
        // 考试卷特征加分（仅 exam_paper profile 享受）
        // 这部分补偿 default profile 也能匹配 (1) / A． 的"通用性"问题
        if self.name == "exam_paper" {
            // 题号用括号包裹：考试卷强特征
            if fp.has_paren_num { score += 15; }
            // 选项用全角点．：考试卷强特征
            if fp.has_fullwidth_dot_opt { score += 10; }
            // 小节标题含分值：考试卷强特征
            if fp.has_score_suffix { score += 15; }
            // "答：" 简写：考试卷特征
            if fp.has_short_answer_kw { score += 5; }
        }
        score
    }
}

/// 格式指纹：扫描文档前 N 段得到的特征样本
#[derive(Debug, Default)]
pub struct FormatFingerprint {
    pub num_sample: String,
    pub opt_sample: String,
    pub chapter_sample: String,
    pub answer_kw_seen: Vec<String>,
    pub has_tail_answer: bool,
    pub has_inline_answer: bool,
    pub has_paren_answer: bool,
    /// 考试卷特征：题号用括号包裹 (1) （1）
    pub has_paren_num: bool,
    /// 考试卷特征：选项用全角点 A．
    pub has_fullwidth_dot_opt: bool,
    /// 考试卷特征：小节标题含分值后缀 "（每题X分）"
    pub has_score_suffix: bool,
    /// 考试卷特征：使用 "答：" 简写答案
    pub has_short_answer_kw: bool,
}

impl FormatFingerprint {
    /// 扫描前 N 段提取指纹
    pub fn scan(paragraphs: &[String], sample_size: usize) -> Self {
        let mut fp = FormatFingerprint::default();
        let take = sample_size.min(paragraphs.len());
        let head = &paragraphs[..take];

        // 考试卷特征检测正则
        let re_paren_num = Regex::new(r"^\s*[\(（]\d+[\)）]").unwrap();
        let re_fullwidth_dot = Regex::new(r"^\s*[A-D]．").unwrap();
        let re_score_suffix = Regex::new(r"(每题|每题\s*\d+\s*分|共\s*\d+\s*分|分值)").unwrap();
        let re_short_ans = Regex::new(r"(?i)^\s*答[:：]").unwrap();

        // 章节样本（找第一个匹配 "章" 字样的行）
        for p in head {
            if p.contains("章") || p.contains("导论") || p.contains("前言") {
                fp.chapter_sample = p.chars().take(80).collect();
                break;
            }
        }

        // 题号样本（找第一个像 "数字.+分隔符" 的行）
        let re_num_probe = Regex::new(r"^\s*[\(（]?\d+[\.、．)）:：]").unwrap();
        for p in head {
            if re_num_probe.is_match(p) {
                fp.num_sample = p.chars().take(80).collect();
                if re_paren_num.is_match(p) {
                    fp.has_paren_num = true;
                }
                break;
            }
        }

        // 选项样本（找第一个像 "字母.+分隔符" 的行）
        let re_opt_probe = Regex::new(r"^\s*[A-D①②③④ⅠⅡⅢⅣ][\.、．)\s]").unwrap();
        for p in head {
            if re_opt_probe.is_match(p) {
                fp.opt_sample = p.chars().take(80).collect();
                if re_fullwidth_dot.is_match(p) {
                    fp.has_fullwidth_dot_opt = true;
                }
                break;
            }
        }

        // 答案关键词扫描 + 考试卷特征
        let answer_kws = ["答案", "正确答案", "参考答案", "标准答案", "Answer", "答："];
        let inline_re = Regex::new(r"(?i)答案|正确答案|参考答案").unwrap();
        for p in head {
            for kw in &answer_kws {
                if p.contains(kw) && !fp.answer_kw_seen.iter().any(|s| s == kw) {
                    fp.answer_kw_seen.push(kw.to_string());
                }
            }
            if inline_re.is_match(p) && p.len() > 6 {
                fp.has_inline_answer = true;
            }
            // 检测题干括号内答案（如 "（ C ）" 或 "(AB)"）
            if Regex::new(r"[\(（]\s*[A-Da-d](?:[、,，][A-Da-d]){0,3}\s*[\)）]").unwrap().is_match(p)
                && !Regex::new(r"[\(（]\s*[\)）]").unwrap().is_match(p) {
                fp.has_paren_answer = true;
            }
            // 考试卷特征：分值后缀
            if re_score_suffix.is_match(p) {
                fp.has_score_suffix = true;
            }
            // 考试卷特征："答：" 简写
            if re_short_ans.is_match(p) {
                fp.has_short_answer_kw = true;
            }
        }

        // 末尾答案区检测：最后 30% 段落中是否集中出现答案关键词
        if paragraphs.len() > 20 {
            let tail_start = paragraphs.len() * 7 / 10;
            let tail = &paragraphs[tail_start..];
            let mut kw_hits = 0;
            for p in tail {
                if p.contains("答案") || p.contains("Answer") || p.contains("参考答案") {
                    kw_hits += 1;
                }
            }
            if kw_hits >= 1 {
                fp.has_tail_answer = true;
            }
        }

        fp
    }
}

// =============== 内置预设 Profile ===============

/// 默认 Profile：当前已支持的教材式题库格式
pub fn default_profile() -> ParseProfile {
    ParseProfile {
        name: "default".into(),
        description: "教材式题库：1.题号 / A.选项 / 一、单选题 / 1-5 ABCDA".into(),

        num_start: r"^\s*[\(（]?(\d+)[\.、．)）]\s*(.+)".into(),
        num_strip: r"^\s*[\(（]?\d+[\.、．)）]\s*(.+)".into(),

        chapter_patterns: vec![r"^\s*第([一二三四五六七八九十百千零\d]+)\s*章".into()],
        intro_patterns: vec![r"^\s*导论\s*$".into(), r"^\s*前言\s*$".into()],

        section_patterns: vec![
            SectionPattern {
                regex: r"^\s*[一二三四五六七八九十]+\s*[、,，]\s*(单项选择|多项选择|单选|多选|判断)".into(),
                with_prefix: true,
            },
            SectionPattern {
                regex: r"^\s*(单项选择题|多项选择题|单选题|多选题|判断题)\s*$".into(),
                with_prefix: false,
            },
        ],
        section_keywords: vec![
            ("单项选择".into(), 1), ("单选".into(), 1), ("单项选择题".into(), 1), ("单选题".into(), 1),
            ("多项选择".into(), 2), ("多选".into(), 2), ("多项选择题".into(), 2), ("多选题".into(), 2),
            ("判断".into(), 3), ("判断题".into(), 3),
        ],

        option_line: r"^\s*([A-D])[\.、．]\s*(.+)".into(),
        option_inline: r"([A-D])[\.、．]\s*".into(),
        option_letters: vec!['A', 'B', 'C', 'D'],

        answer_inline: r"(?i)^\s*(答案|正确答案|【答案】)\s*[:：]?\s*(.+)".into(),
        answer_extract: r"(\d+)[\.、．)]\s*\(?([A-Da-d](?:[、,，]*[A-Da-d]){0,3}|正确|错误|对|错|√|×|true|false)\)?".into(),
        answer_range: r"(\d+)\s*[-—–~～]+\s*(\d+)\s*[.、．:：]?".into(),
        answer_paren: r"[\(（]\s*([A-Da-d](?:[、,，][A-Da-d]){0,3})\s*[\)）]".into(),
        answer_keywords: vec![
            "答案".into(), "正确答案".into(), "参考答案".into(),
            "标准答案".into(), "试题答案".into(), "答案".into(), "answer".into(),
        ],
        analysis_keywords: vec!["解析".into(), "答案解析".into(), "【解析】".into()],

        judge_true_tokens: vec!["正确".into(), "对".into(), "√".into(), "T".into(), "true".into()],
        judge_false_tokens: vec!["错误".into(), "错".into(), "×".into(), "F".into(), "false".into()],

        blank_pattern: r"_{2,}|（\s*）|\(\s*\)|【\s*】".into(),

        answer_section_strategy: AnswerSectionStrategy::Tail,

        // 默认含基础公式保护：$$...$$ / $...$ / 化学式下标 / 上下标
        formula_protect_patterns: vec![
            r"\$\$[^$]+\$\$".into(),         // 块级 LaTeX
            r"\$[^$\n]+\$".into(),            // 行内 LaTeX
            r"[A-Za-z]\d+[A-Za-z]".into(),    // 化学式如 H2O / NaCl（字母数字字母）
            r"[A-Za-z][\^_]\{[^}]*\}".into(), // 上下标 x^{2} / x_{n}
            r"[A-Za-z][\^_][A-Za-z0-9]".into(), // 简单上下标 x^2 / x_n
        ],
    }
}

/// 标准考试卷 Profile
///
/// 兼容格式：
/// - 题号：(1) （1） 1. 1、 1）
/// - 题型标识：一、单项选择题（每题1分，共20分）
/// - 选项：A． B． C． D．（全角点）或 A. B. C. D.
/// - 答案：题后立即 / 末尾答案区
/// - 解析：解析：xxx 或 【解析】xxx
pub fn exam_paper_profile() -> ParseProfile {
    let mut p = default_profile();
    p.name = "exam_paper".into();
    p.description = "标准考试卷：(1)题号 / A．全角点选项 / 一、单项选择题（含分值）".into();

    // 题号：放宽到 (1) （1） 1. 1、 1） 1. 等多种（注意：字符类中 . 不需转义）
    p.num_start = r"^\s*[\(（]?(\d+)[.、．)）:：]\s*(.+)".into();
    p.num_strip = r"^\s*[\(（]?\d+[.、．)）:：]\s*(.+)".into();

    // 小节标题：允许带分值后缀 "一、单项选择题（每题1分，共20分）"
    p.section_patterns = vec![
        SectionPattern {
            regex: r"^\s*[一二三四五六七八九十]+\s*[、,，]\s*(单项选择|多项选择|单选|多选|判断|名词解释|简答|论述|计算|填空)[^）)]*".into(),
            with_prefix: true,
        },
        SectionPattern {
            regex: r"^\s*(单项选择题|多项选择题|单选题|多选题|判断题|填空题|简答题|论述题|名词解释)[^）)]*$".into(),
            with_prefix: false,
        },
    ];
    p.section_keywords = vec![
        ("单项选择".into(), 1), ("单选".into(), 1), ("单项选择题".into(), 1), ("单选题".into(), 1),
        ("多项选择".into(), 2), ("多选".into(), 2), ("多项选择题".into(), 2), ("多选题".into(), 2),
        ("判断".into(), 3), ("判断题".into(), 3),
        ("填空".into(), 4), ("填空题".into(), 4),
        ("简答".into(), 5), ("简答题".into(), 5), ("名词解释".into(), 5),
        ("论述".into(), 5), ("论述题".into(), 5), ("计算".into(), 5), ("计算题".into(), 5),
    ];

    // 选项：兼容全角点．和半角点.
    p.option_line = r"^\s*([A-D])[\.、．）)]\s*(.+)".into();
    p.option_inline = r"([A-D])[\.、．）)]\s*".into();

    // 答案：兼容 "答：A" 简写
    p.answer_inline = r"(?i)^\s*(答案|正确答案|【答案】|答)\s*[:：]?\s*(.+)".into();

    // 考试卷常为题后立即答案 或 末尾答案区
    p.answer_section_strategy = AnswerSectionStrategy::Hybrid;

    p
}

/// 自动探测：根据格式指纹选最佳 Profile
pub fn detect_profile(paragraphs: &[String]) -> ParseProfile {
    let fingerprint = FormatFingerprint::scan(paragraphs, 200);

    let candidates: Vec<ParseProfile> = vec![
        default_profile(),
        exam_paper_profile(),
    ];

    let mut best = candidates[0].clone();
    let mut best_score = 0;
    for p in &candidates {
        let score = p.match_score(&fingerprint);
        crate::dbg_log(&format!(
            "detect_profile: candidate={} score={}",
            p.name, score
        ));
        if score > best_score {
            best_score = score;
            best = p.clone();
        }
    }

    crate::dbg_log(&format!(
        "detect_profile: chosen={} score={} fp={:?}",
        best.name, best_score, fingerprint
    ));

    best
}

/// 全局缓存的默认 CompiledProfile（避免每次重新编译）
static DEFAULT_COMPILED: OnceLock<Result<CompiledProfile, regex::Error>> = OnceLock::new();

/// 获取默认 CompiledProfile（编译失败时返回错误）
pub fn default_compiled() -> &'static Result<CompiledProfile, regex::Error> {
    DEFAULT_COMPILED.get_or_init(|| default_profile().compile())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_compiles() {
        let p = default_profile();
        let compiled = p.compile();
        assert!(compiled.is_ok(), "default profile 编译失败: {:?}", compiled.err());
    }

    #[test]
    fn test_exam_paper_profile_compiles() {
        let p = exam_paper_profile();
        let compiled = p.compile();
        assert!(compiled.is_ok(), "exam_paper profile 编译失败: {:?}", compiled.err());
    }

    #[test]
    fn test_default_profile_matches_current_format() {
        // 当前题库典型格式
        let paras = vec![
            "导论".to_string(),
            "一、单项选择题".to_string(),
            "1.党的十八大以来，（  ）加速演进。".to_string(),
            "A.社会主义革命".to_string(),
            "B.保护主义".to_string(),
            "答案：A".to_string(),
            "第一章 引言".to_string(),
        ];
        let fp = FormatFingerprint::scan(&paras, 200);
        let p = default_profile();
        let score = p.match_score(&fp);
        assert!(score >= 50, "default profile 评分应 >=50, 实际 {}", score);
    }

    #[test]
    fn test_exam_paper_profile_matches_paren_number() {
        let paras = vec![
            "一、单项选择题（每题1分，共20分）".to_string(),
            "(1) 题目内容".to_string(),
            "A．选项A".to_string(),
            "B．选项B".to_string(),
            "答案：A".to_string(),
        ];
        let fp = FormatFingerprint::scan(&paras, 200);
        let p = exam_paper_profile();
        let score = p.match_score(&fp);
        assert!(score >= 50, "exam_paper profile 评分应 >=50, 实际 {}", score);
    }

    #[test]
    fn test_detect_profile_chooses_exam_paper_for_paren_format() {
        let paras = vec![
            "一、单项选择题（每题1分，共20分）".to_string(),
            "(1) 题目内容".to_string(),
            "A．选项A".to_string(),
            "B．选项B".to_string(),
            "答案：A".to_string(),
            "二、多项选择题（每题2分，共10分）".to_string(),
            "(2) 题目内容".to_string(),
            "A．选项A".to_string(),
            "B．选项B".to_string(),
        ];
        let chosen = detect_profile(&paras);
        assert_eq!(chosen.name, "exam_paper",
            "应选 exam_paper，实际选 {}", chosen.name);
    }

    #[test]
    fn test_detect_profile_chooses_default_for_textbook() {
        let paras = vec![
            "导论".to_string(),
            "一、单项选择题".to_string(),
            "1.题目内容".to_string(),
            "A.选项A".to_string(),
            "B.选项B".to_string(),
            "第一章 引言".to_string(),
            "1-5 ABCDA".to_string(),
        ];
        let chosen = detect_profile(&paras);
        assert_eq!(chosen.name, "default",
            "应选 default，实际选 {}", chosen.name);
    }

    #[test]
    fn test_formula_protect_pattern() {
        let p = default_profile();
        let compiled = p.compile().unwrap();
        // LaTeX 块级
        assert!(compiled.formula_protect[0].is_match("$$x^2 + y^2 = r^2$$"));
        // LaTeX 行内
        assert!(compiled.formula_protect[1].is_match("公式 $E=mc^2$ 在此"));
        // 化学式 H2O
        assert!(compiled.formula_protect[2].is_match("H2O 是水"));
        // 上下标 x^{2}
        assert!(compiled.formula_protect[3].is_match("x^{2} + 1"));
        // 简单上下标 x^2
        assert!(compiled.formula_protect[4].is_match("y^2 = 4x"));
    }
}
