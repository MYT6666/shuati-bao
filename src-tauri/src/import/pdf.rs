use anyhow::Result;
use lopdf::Document;

/// 把 PDF 每页文本提取出来，按 docx 兼容格式组装成 HTML：
///   每个非空行包成 `<p>...</p>`，与 mammoth 输出结构一致。
/// 这样可以直接复用 html_to_questions 流程，无需改识别逻辑。
pub fn pdf_to_html(path: &str) -> Result<String> {
    let doc = Document::load(path)?;
    let pages = doc.get_pages();
    let mut out = String::new();
    for (i, (page_num, _id)) in pages.iter().enumerate() {
        if i > 0 {
            out.push_str("<p></p>\n"); // 跨页分隔，便于识别 chapter 边界
        }
        let text = doc
            .extract_text(&[*page_num])
            .unwrap_or_default();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // 简单转义：避免用户题里出现 < > 破坏 HTML
            let safe = trimmed
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            out.push_str(&format!("<p>{}</p>\n", safe));
        }
    }
    Ok(out)
}
