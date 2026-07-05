use tauri::State;
use crate::db::{DbState, models::PracticeRecord, repo};

#[tauri::command]
pub fn record_practice(db: State<'_, DbState>, record: PracticeRecord) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::record_practice(&conn, &record).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_wrong(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<Vec<i64>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let list = repo::list_wrong(&conn, bank_id).map_err(|e| e.to_string())?;
    Ok(list.into_iter().map(|w| w.question_id).collect())
}

#[tauri::command]
pub fn bank_stats(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<crate::db::models::BankStats, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::bank_stats(&conn, bank_id).map_err(|e| e.to_string())
}

/// P1-7: 列出已掌握错题
#[tauri::command]
pub fn list_mastered(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<Vec<i64>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let list = repo::list_mastered(&conn, bank_id).map_err(|e| e.to_string())?;
    Ok(list.into_iter().map(|w| w.question_id).collect())
}

/// P1-7: 标记错题为已掌握
#[tauri::command]
pub fn mark_wrong_mastered(db: State<'_, DbState>, bank_id: i64, question_id: i64) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::mark_wrong_mastered(&conn, bank_id, question_id).map_err(|e| e.to_string())
}

/// BUG-011 修复：把已掌握的错题放回 pending（不污染 practice_records 统计）
#[tauri::command]
pub fn restore_wrong_to_pending(db: State<'_, DbState>, bank_id: i64, question_id: i64) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::restore_wrong_to_pending(&conn, bank_id, question_id).map_err(|e| e.to_string())
}
