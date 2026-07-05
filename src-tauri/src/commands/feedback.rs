use tauri::{AppHandle, Manager};
use std::fs;
use std::io::Write;
use serde::Serialize;

#[derive(Serialize)]
pub struct FeedbackPayload {
    pub markdown: String,
    pub system_info: String,
    pub recent_logs: String,
}

/// 收集系统信息（应用版本、OS、Tauri 版本）
fn collect_system_info(app: &AppHandle) -> String {
    let mut info = String::new();
    info.push_str(&format!("- OS: {}\n", std::env::consts::OS));
    info.push_str(&format!("- Arch: {}\n", std::env::consts::ARCH));
    info.push_str(&format!("- Rust: {}\n", rustc_version_runtime()));
    let pkg_version = app.package_info().version.to_string();
    info.push_str(&format!("- App version: {}\n", pkg_version));
    info.push_str(&format!("- Tauri: 2.x\n"));
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    info.push_str(&format!("- 时间: {}\n", now));
    info
}

fn rustc_version_runtime() -> &'static str {
    // 简单实现，避免额外依赖
    "stable"
}

/// 读取最近 N 行调试日志
fn read_recent_logs(n: usize) -> String {
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("shuati-debug.log")));
    if let Some(path) = log_path {
        if let Ok(content) = fs::read_to_string(&path) {
            let lines: Vec<&str> = content.lines().rev().take(n).collect();
            let mut result = String::new();
            for line in lines.iter().rev() {
                result.push_str(line);
                result.push('\n');
            }
            return result;
        }
    }
    String::from("（无日志文件）")
}

/// 构造反馈 Markdown 文本
#[tauri::command]
pub fn build_feedback(
    app: AppHandle,
    category: String,
    title: String,
    description: String,
    contact: String,
) -> Result<FeedbackPayload, String> {
    let system_info = collect_system_info(&app);
    let recent_logs = read_recent_logs(100);

    let category_label = match category.as_str() {
        "bug" => "🐛 Bug 报告",
        "feature" => "💡 功能建议",
        "question" => "❓ 使用问题",
        "other" => "📝 其他",
        _ => category.as_str(),
    };

    let mut md = String::new();
    md.push_str(&format!("## {}\n\n", category_label));
    md.push_str(&format!("**标题**: {}\n\n", title));
    md.push_str("**详细描述**:\n```\n");
    md.push_str(&description);
    md.push_str("\n```\n\n");
    if !contact.trim().is_empty() {
        md.push_str(&format!("**联系方式**: {}\n\n", contact));
    }
    md.push_str("### 系统信息\n");
    md.push_str("```\n");
    md.push_str(&system_info);
    md.push_str("```\n\n");
    md.push_str("### 最近日志（最近 100 行）\n");
    md.push_str("<details><summary>点击展开</summary>\n\n```\n");
    md.push_str(&recent_logs);
    md.push_str("\n```\n</details>\n");

    Ok(FeedbackPayload {
        markdown: md,
        system_info,
        recent_logs,
    })
}

/// 保存反馈到本地文件（zip-free 简单版：写 .md 到反馈目录）
#[tauri::command]
pub fn save_feedback_local(
    app: AppHandle,
    content: String,
) -> Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let feedback_dir = app_data.join("feedback");
    fs::create_dir_all(&feedback_dir).map_err(|e| e.to_string())?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let file_path = feedback_dir.join(format!("feedback_{}.md", ts));
    let mut f = fs::File::create(&file_path).map_err(|e| e.to_string())?;
    f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(file_path.to_string_lossy().to_string())
}

/// 在文件管理器中打开反馈目录
#[tauri::command]
pub fn open_feedback_folder(app: AppHandle) -> Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let feedback_dir = app_data.join("feedback");
    fs::create_dir_all(&feedback_dir).map_err(|e| e.to_string())?;
    let dir = feedback_dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

/// 获取反馈目录路径
#[tauri::command]
pub fn get_feedback_dir(app: AppHandle) -> Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let feedback_dir = app_data.join("feedback");
    Ok(feedback_dir.to_string_lossy().to_string())
}
