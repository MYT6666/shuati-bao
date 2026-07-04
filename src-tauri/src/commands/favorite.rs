use tauri::State;
use crate::db::{DbState, repo};

#[tauri::command]
pub fn toggle_favorite(db: State<'_, DbState>, bank_id: i64, question_id: i64) -> anyhow::Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::toggle_favorite(&conn, bank_id, question_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_favorites(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<Vec<i64>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::list_favorites(&conn, bank_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_favorite(db: State<'_, DbState>, bank_id: i64, question_id: i64) -> anyhow::Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::is_favorite(&conn, bank_id, question_id).map_err(|e| e.to_string())
}

/// P2-8: 批量清空收藏夹（单条 SQL，避免 N 次 IPC）
#[tauri::command]
pub fn clear_favorites(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::clear_favorites(&conn, bank_id).map_err(|e| e.to_string())
}
