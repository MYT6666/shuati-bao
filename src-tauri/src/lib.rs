pub mod db;
pub mod import;
pub mod commands;

use tauri::Manager;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// 调试日志文件路径（exe 同目录），追加写入
fn debug_log_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("shuati-debug.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("shuati-debug.log"))
}

/// 写一行调试日志到文件 + stderr
pub fn dbg_log(msg: impl AsRef<str>) {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!("[{}] {}", ts, msg.as_ref());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(debug_log_path()) {
        let _ = writeln!(f, "{}", line);
        let _ = f.flush();
    }
    eprintln!("[DBG] {}", line);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 安装全局 panic hook：把 panic 信息写到文件，防止闪退无痕迹
    let log_path = debug_log_path();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let msg = format!("[{}] PANIC: {}\n  location: {}", ts, info, info.location().map(|l| l.to_string()).unwrap_or_default());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = writeln!(f, "{}", msg);
            let _ = f.flush();
        }
        eprintln!("{}", msg);
    }));
    dbg_log("app_run_start");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("no app data dir");
            let db_state = db::open_db(&app_data).expect("failed to open db");
            app.manage(db_state);
            app.manage(commands::import::CancelFlag(Arc::new(AtomicBool::new(false))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bank::list_banks,
            commands::bank::create_bank,
            commands::bank::delete_bank,
            commands::bank::export_bank,
            commands::question::list_questions,
            commands::question::clear_bank_questions,
            commands::question::update_question,
            commands::question::analyze_question,
            commands::question::search_questions,
            commands::practice::record_practice,
            commands::practice::list_wrong,
            commands::practice::bank_stats,
            commands::practice::list_mastered,
            commands::practice::mark_wrong_mastered,
            commands::import::import_from_html,
            commands::import::import_from_pdf,
            commands::import::import_with_ai,
            commands::import::test_ai_connection,
            commands::import::cancel_import,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_db_info,
            commands::settings::open_db_folder,
            commands::settings::pick_database_folder,
            commands::settings::change_db_path,
            commands::settings::restart_app,
            commands::settings::quit_app,
            commands::settings::ocr_available,
            commands::settings::backup_database,
            commands::favorite::toggle_favorite,
            commands::favorite::list_favorites,
            commands::favorite::is_favorite,
            commands::favorite::clear_favorites,
            commands::feedback::build_feedback,
            commands::feedback::save_feedback_local,
            commands::feedback::open_feedback_folder,
            commands::feedback::get_feedback_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
