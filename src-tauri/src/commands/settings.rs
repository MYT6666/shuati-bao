use tauri::{State, AppHandle, Manager};
use crate::db::{DbState, repo};

#[tauri::command]
pub fn get_setting(db: State<'_, DbState>, key: String) -> anyhow::Result<Option<String>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::get_setting(&conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(db: State<'_, DbState>, key: String, value: String) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ocr_available() -> bool {
    crate::import::ocr::tesseract_available()
}

/// P2-18/P0 数据安全：手动备份当前数据库到 backups/ 目录
#[tauri::command]
pub fn backup_database(app: AppHandle) -> anyhow::Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let src = app_data.join("shuati.db");
    if !src.exists() {
        return Err("数据库文件不存在".to_string());
    }
    let backups_dir = app_data.join("backups");
    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let dst = backups_dir.join(format!("shuati_{}.db", ts));
    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
    // 记录数据库路径到 settings，供设置页展示
    if let Ok(conn) = rusqlite::Connection::open(&src) {
        let _ = repo::set_setting(&conn, "db_path", src.to_str().unwrap_or(""));
    }
    Ok(dst.to_string_lossy().to_string())
}

/// P0 数据安全：导入前自动备份（保留最近 5 个）
pub fn auto_backup_before_import(app: &AppHandle) -> anyhow::Result<()> {
    let app_data = app.path().app_data_dir()?;
    let src = app_data.join("shuati.db");
    if !src.exists() {
        return Ok(());
    }
    let backups_dir = app_data.join("backups");
    std::fs::create_dir_all(&backups_dir)?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let dst = backups_dir.join(format!("shuati_{}.db", ts));
    std::fs::copy(&src, &dst)?;
    // 清理旧备份：仅保留最近 5 个
    let mut backups: Vec<_> = std::fs::read_dir(&backups_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("shuati_") && e.file_name().to_string_lossy().ends_with(".db"))
        .collect();
    backups.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(std::time::SystemTime::UNIX_EPOCH));
    while backups.len() > 5 {
        if let Some(old) = backups.first() {
            let _ = std::fs::remove_file(old.path());
            backups.remove(0);
        } else {
            break;
        }
    }
    Ok(())
}
