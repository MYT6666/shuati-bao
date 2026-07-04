use tauri::State;
use crate::db::{DbState, models::Question, repo};
use crate::import::ai;

#[tauri::command]
pub fn list_questions(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<Vec<Question>, String> {
    crate::dbg_log(format!("list_questions_start bank_id={}", bank_id));
    let conn = db.0.lock().map_err(|e| {
        crate::dbg_log(format!("list_questions_lock_err: {}", e));
        e.to_string()
    })?;
    let r = repo::list_questions(&conn, bank_id).map_err(|e| {
        crate::dbg_log(format!("list_questions_repo_err: {}", e));
        e.to_string()
    });
    crate::dbg_log(format!("list_questions_done count={}", r.as_ref().map(|v| v.len()).unwrap_or(0)));
    r
}

#[tauri::command]
pub fn clear_bank_questions(db: State<'_, DbState>, bankId: i64) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::clear_bank_questions(&conn, bankId).map_err(|e| e.to_string())
}

/// P1-10: 按关键词搜索题目
#[tauri::command]
pub fn search_questions(db: State<'_, DbState>, bank_id: i64, query: String, limit: Option<i64>) -> anyhow::Result<Vec<Question>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(50);
    repo::search_questions(&conn, bank_id, &query, lim).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_question(db: State<'_, DbState>, q: Question) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::update_question(&conn, &q).map_err(|e| e.to_string())
}

/// 调用 AI 解析单题：读取设置中的 AI 配置，调用 AI 返回解析文本
#[tauri::command]
pub async fn analyze_question(db: State<'_, DbState>, q: Question) -> anyhow::Result<String, String> {
    let (api_key, base_url, model) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let key = repo::get_setting(&conn, "ai_api_key")
            .map_err(|e| e.to_string())?
            .ok_or("未配置 AI Key，请先在设置中填写")?;
        let base = repo::get_setting(&conn, "ai_base_url")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "https://api.agnes-ai.com/v1".to_string());
        let model = repo::get_setting(&conn, "ai_model")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "agnes-2.0-flash".to_string());
        (key, base, model)
    };

    // 解析 options JSON 字符串为列表
    let options: Vec<String> = match q.options.as_deref() {
        Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or_default(),
        _ => Vec::new(),
    };

    ai::analyze_question(
        &q.stem,
        &q.q_type,
        &options,
        q.answer.as_deref(),
        &api_key,
        &base_url,
        &model,
    )
    .await
    .map_err(|e| e.to_string())
}
