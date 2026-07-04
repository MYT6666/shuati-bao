use tauri::State;
use crate::db::{DbState, models::*, repo};
use serde::Serialize;

#[tauri::command]
pub fn list_banks(db: State<'_, DbState>) -> anyhow::Result<Vec<QuizBank>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::list_banks(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_bank(db: State<'_, DbState>, new_bank: NewBank) -> anyhow::Result<QuizBank, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::create_bank(&conn, new_bank).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_bank(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::delete_bank(&conn, bank_id).map_err(|e| e.to_string())
}

/// 导出题库为 JSON 字符串（包含题库信息和所有题目）
#[derive(Serialize)]
pub struct ExportData {
    pub bank: QuizBank,
    pub questions: Vec<Question>,
    pub exported_at: String,
}

#[tauri::command]
pub fn export_bank(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let bank = repo::list_banks(&conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|b| b.id == bank_id)
        .ok_or("题库不存在")?;
    let questions = repo::list_questions(&conn, bank_id).map_err(|e| e.to_string())?;
    let data = ExportData {
        bank,
        questions,
        exported_at: chrono::Local::now().to_rfc3339(),
    };
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}
