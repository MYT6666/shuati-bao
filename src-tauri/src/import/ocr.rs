use anyhow::Result;
use base64::Engine;
use std::fs;
use std::process::Command;

/// 对 base64 图片做 OCR，返回识别文本。需系统已安装 tesseract 并在 PATH。
pub fn ocr_base64(b64: &str, lang: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let tmp = std::env::temp_dir().join(format!("ocr_{}.png", uuid::Uuid::new_v4()));
    fs::write(&tmp, &bytes)?;
    let result = run_tesseract(&tmp, lang);
    let _ = fs::remove_file(&tmp);
    let output = result?;
    if !output.status.success() {
        anyhow::bail!("tesseract failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // 归一化空白：trim 首尾，内部连续空白/换行归一为单空格，避免破坏下游题型识别正则
    let cleaned: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(cleaned)
}

fn run_tesseract(tmp: &std::path::Path, lang: &str) -> Result<std::process::Output> {
    let output = Command::new("tesseract")
        .arg(tmp)
        .arg("stdout")
        .args(["-l", lang])
        .output()?;
    Ok(output)
}

/// 检测 tesseract 是否可用
pub fn tesseract_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
