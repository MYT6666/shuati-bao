use crate::db::models::Question;
use crate::import::docx::parse_html;
use crate::import::structure::{parse_questions, ParsedQuestion, QType};
use anyhow::Result;

/// 把 mammoth 输出的 HTML 转成可入库的 Question 列表
/// 注意：不再做 OCR（太慢且可能崩溃），如需识别图片内容请用 AI 引擎
pub fn html_to_questions(html: &str, bank_id: i64) -> Result<Vec<Question>> {
    crate::dbg_log(format!("    parse_html_begin html_len={}", html.len()));
    let parsed = parse_html(html)?;
    crate::dbg_log(format!("    parse_html_done paragraphs={} images={}", parsed.paragraphs.len(), parsed.images.len()));
    crate::dbg_log("    parse_questions_begin");
    let parsed_qs = parse_questions(&parsed.paragraphs);
    crate::dbg_log(format!("    parse_questions_done count={}", parsed_qs.len()));
    let qs: Vec<Question> = parsed_qs.into_iter().map(|pq| to_question(pq, bank_id)).collect();
    crate::dbg_log(format!("    to_question_done count={}", qs.len()));
    Ok(qs)
}

pub fn to_question(pq: ParsedQuestion, bank_id: i64) -> Question {
    let q_type = match pq.q_type {
        QType::Single => "single",
        QType::Multi => "multi",
        QType::Judge => "judge",
        QType::Blank => "blank",
        QType::Qa => "qa",
    }.to_string();
    let options = if pq.options.is_empty() { None } else { serde_json::to_string(&pq.options).ok() };
    Question {
        id: 0, bank_id, q_type, stem: pq.stem, options, answer: pq.answer,
        analysis: pq.analysis, source_index: Some(pq.source_index as i64), confidence: pq.confidence,
    }
}
