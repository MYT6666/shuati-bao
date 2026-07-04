use anyhow::Result;

/// 从 mammoth 输出的 HTML 中提取段落文本列表与图片 base64 列表。
/// 图片以 <img src="data:image/png;base64,xxx"> 形式内嵌。
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    /// 段落列表（已去标签），图片位置用 `[IMG:索引]` 占位
    pub paragraphs: Vec<String>,
    /// 图片 base64（不含 data: 前缀）
    pub images: Vec<String>,
}

pub fn parse_html(html: &str) -> Result<ParsedDoc> {
    let mut paragraphs = Vec::new();
    let mut images = Vec::new();

    // 兜底：若无 <p> 标签，按换行切段
    if !html.to_lowercase().contains("<p") {
        let plain = strip_tags(html);
        let paragraphs: Vec<String> = plain
            .split('\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(ParsedDoc { paragraphs, images });
    }

    // 按 <p> 切段（mammoth 默认段落标签）
    let mut current = String::new();
    let mut in_p = false;
    let raw: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < raw.len() {
        // 检测 <p 开标签（避免匹配 <pre> 等：要求下一个字符是 > 或空格）
        if raw[i] == '<' && raw.get(i + 1) == Some(&'p') {
            let next = raw.get(i + 2).copied().unwrap_or('>');
            if next == '>' || next == ' ' {
                in_p = true;
                // 跳过到 >
                while i < raw.len() && raw[i] != '>' { i += 1; }
                i += 1;
                continue;
            }
        }
        // 检测 </p>
        if raw[i] == '<'
            && raw.get(i + 1) == Some(&'/')
            && raw.get(i + 2) == Some(&'p')
            && raw.get(i + 3) == Some(&'>')
        {
            in_p = false;
            let text = strip_tags(&current).trim().to_string();
            if !text.is_empty() {
                paragraphs.push(text);
            }
            current.clear();
            i += "</p>".len();
            continue;
        }
        // 检测 <img
        if raw[i] == '<'
            && raw.get(i + 1) == Some(&'i')
            && raw.get(i + 2) == Some(&'m')
            && raw.get(i + 3) == Some(&'g')
        {
            // 找到 >
            let mut end = i;
            while end < raw.len() && raw[end] != '>' { end += 1; }
            let img_tag: String = raw[i..end].iter().collect();
            if let Some(idx) = img_tag.find("base64,") {
                let after = &img_tag[idx + "base64,".len()..];
                let b64: String = after.chars().take_while(|c| *c != '"' && *c != ' ').collect();
                if !b64.is_empty() {
                    images.push(b64);
                    current.push_str(&format!("[IMG:{}]", images.len() - 1));
                }
            }
            i = end;
            continue;
        }
        if in_p {
            current.push(raw[i]);
        }
        i += 1;
    }
    // 兜底：若无 <p>，按 <br> 或换行切
    if paragraphs.is_empty() && !current.trim().is_empty() {
        for line in strip_tags(&current).split('\n') {
            let t = line.trim();
            if !t.is_empty() { paragraphs.push(t.to_string()); }
        }
    }
    Ok(ParsedDoc { paragraphs, images })
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_paragraphs() {
        let html = "<p>1. 以下哪个是A？</p><p>A. 选项一</p><p>B. 选项二</p>";
        let parsed = parse_html(html).unwrap();
        assert_eq!(parsed.paragraphs.len(), 3);
        assert_eq!(parsed.paragraphs[0], "1. 以下哪个是A？");
        assert!(parsed.images.is_empty());
    }

    #[test]
    fn test_parse_with_image() {
        let html = "<p>题干 <img src=\"data:image/png;base64,abc123==\" /></p>";
        let parsed = parse_html(html).unwrap();
        assert_eq!(parsed.paragraphs.len(), 1);
        assert!(parsed.paragraphs[0].contains("[IMG:0]"));
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0], "abc123==");
    }

    #[test]
    fn test_parse_fallback_no_p_tags() {
        let html = "line1\nline2\nline3";
        let parsed = parse_html(html).unwrap();
        assert_eq!(parsed.paragraphs.len(), 3);
        assert_eq!(parsed.paragraphs[0], "line1");
        assert_eq!(parsed.paragraphs[1], "line2");
        assert_eq!(parsed.paragraphs[2], "line3");
        assert!(parsed.images.is_empty());
    }
}
