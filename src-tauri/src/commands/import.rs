use tauri::{State, AppHandle, Emitter};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crate::db::{DbState, repo};
use crate::import::pipeline::html_to_questions;

/// 取消标志（全局共享，前端点取消时设为 true）
pub struct CancelFlag(pub Arc<AtomicBool>);

#[derive(Serialize)]
pub struct ImportResult {
    pub count: i64,
    pub expected: i64,
}

/// 前端调用 mammoth 转 HTML 后，把 HTML 传入此 command 做结构化并入库
#[tauri::command]
pub fn import_from_html(app: AppHandle, db: State<'_, DbState>, bank_id: i64, html: String) -> anyhow::Result<i64, String> {
    crate::dbg_log(format!("import_from_html_start bank_id={} html_len={}", bank_id, html.len()));
    // BUG-002 修复：空 HTML 校验，避免清空题库
    if html.trim().is_empty() {
        crate::dbg_log("  html_empty_rejected");
        return Err("导入内容为空".to_string());
    }
    // P0 数据安全：导入前自动备份（BUG-009 修复：传入 db state 做 checkpoint）
    let _ = crate::commands::settings::auto_backup_before_import(&app, Some(&db));
    crate::dbg_log("  html_to_questions_begin");
    let questions = html_to_questions(&html, bank_id).map_err(|e| {
        crate::dbg_log(format!("  html_to_questions_error: {}", e));
        e.to_string()
    })?;
    crate::dbg_log(format!("  html_to_questions_done count={}", questions.len()));
    // BUG-002 修复：未识别到题目时拒绝导入，避免清空旧题
    if questions.is_empty() {
        crate::dbg_log("  no_questions_rejected");
        return Err("未识别到任何题目，已取消导入".to_string());
    }
    let count = questions.len() as i64;
    crate::dbg_log("  db_lock_acquire");
    let conn = db.0.lock().map_err(|e| {
        crate::dbg_log(format!("  db_lock_error: {}", e));
        e.to_string()
    })?;
    // BUG-001 修复：使用 replace_bank_questions 单事务原子替换
    crate::dbg_log("  replace_bank_questions_begin");
    repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| {
        crate::dbg_log(format!("  replace_bank_questions_error: {}", e));
        e.to_string()
    })?;
    crate::dbg_log(format!("import_from_html_done count={}", count));
    Ok(count)
}

/// 从 PDF 文件路径提取每页文本并按 docx 兼容 HTML 格式组装
#[tauri::command]
pub fn import_from_pdf(app: AppHandle, db: State<'_, DbState>, bank_id: i64, path: String) -> anyhow::Result<i64, String> {
    crate::dbg_log(format!("import_from_pdf_start bank_id={} path={}", bank_id, path));
    // P0 数据安全：导入前自动备份（BUG-009 修复：传入 db state 做 checkpoint）
    let _ = crate::commands::settings::auto_backup_before_import(&app, Some(&db));

    // 1. 用 lopdf 提取文本
    let html = crate::import::pdf::pdf_to_html(&path).map_err(|e| {
        crate::dbg_log(format!("  pdf_parse_error: {}", e));
        format!("PDF 解析失败：{}", e)
    })?;
    crate::dbg_log(format!("  pdf_to_html_done html_len={}", html.len()));

    // BUG-002 修复：PDF 解析后内容为空校验
    if html.trim().is_empty() {
        return Err("PDF 未提取到任何文本内容".to_string());
    }

    // 2. 复用 import_from_html 走结构化识别入库
    crate::dbg_log("  reuse_import_from_html_begin");
    let questions = crate::import::pipeline::html_to_questions(&html, bank_id).map_err(|e| {
        crate::dbg_log(format!("  html_to_questions_error: {}", e));
        e.to_string()
    })?;
    // BUG-002 修复：未识别到题目时拒绝导入
    if questions.is_empty() {
        return Err("PDF 中未识别到任何题目，已取消导入".to_string());
    }
    let count = questions.len() as i64;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // BUG-001 修复：使用原子替换
    repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
    crate::dbg_log(format!("import_from_pdf_done count={}", count));
    Ok(count)
}

/// 测试 AI 连通性（导入前先测，避免卡死）
#[tauri::command]
pub async fn test_ai_connection(db: State<'_, DbState>) -> anyhow::Result<(), String> {
    let (api_key, base_url, model) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let key = repo::get_setting(&conn, "ai_api_key").map_err(|e| e.to_string())?.ok_or("未配置 AI Key")?;
        let base = repo::get_setting(&conn, "ai_base_url").map_err(|e| e.to_string())?.unwrap_or_else(|| "https://api.agnes-ai.com/v1".to_string());
        let model = repo::get_setting(&conn, "ai_model").map_err(|e| e.to_string())?.unwrap_or_else(|| "agnes-2.0-flash".to_string());
        (key, base, model)
    };
    crate::import::ai::test_connection(&api_key, &base_url, &model)
        .await
        .map_err(|e| e.to_string())
}

/// 取消正在进行的 AI 导入
#[tauri::command]
pub fn cancel_import(flag: State<'_, CancelFlag>) -> anyhow::Result<(), String> {
    flag.0.store(true, Ordering::SeqCst);
    eprintln!("[AI] 收到取消信号");
    Ok(())
}

#[tauri::command]
pub async fn import_with_ai(
    app: AppHandle,
    db: State<'_, DbState>,
    flag: State<'_, CancelFlag>,
    bank_id: i64,
    text: String,
) -> anyhow::Result<ImportResult, String> {
    crate::dbg_log(format!("import_with_ai_start bank_id={} text_len={}", bank_id, text.len()));
    // P0 数据安全：导入前自动备份（BUG-009 修复：传入 db state 做 checkpoint）
    let _ = crate::commands::settings::auto_backup_before_import(&app, Some(&db));
    let (api_key, base_url, model) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let key = repo::get_setting(&conn, "ai_api_key").map_err(|e| e.to_string())?.ok_or("未配置 AI Key")?;
        let base = repo::get_setting(&conn, "ai_base_url").map_err(|e| e.to_string())?.unwrap_or_else(|| "https://api.agnes-ai.com/v1".to_string());
        let model = repo::get_setting(&conn, "ai_model").map_err(|e| e.to_string())?.unwrap_or_else(|| "agnes-2.0-flash".to_string());
        (key, base, model)
    };
    crate::dbg_log(format!("  settings_loaded model={}", model));

    // BUG-014 修复：检查取消标志是否已被用户在 testAiConnection 阶段设置
    // 旧实现无条件重置 flag=false，导致用户在连接测试阶段点取消的意图被丢失
    // 用 swap 原子地"读取并重置"：若用户已取消则返回错误，否则重置为 false 供本次 AI 流程使用
    let was_cancelled = flag.0.swap(false, Ordering::SeqCst);
    if was_cancelled {
        crate::dbg_log("  cancelled_before_ai_start");
        return Err("用户已取消导入".to_string());
    }
    let cancel = flag.0.clone();

    // 预估题数
    let expected = crate::import::ai::count_question_boundaries(&text) as i64;
    crate::dbg_log(format!("  expected={}", expected));

    // ===== 混合引擎：本地优先，AI 补漏 =====
    // 1. 先用本地正则引擎解析（毫秒级）
    crate::dbg_log("  local_parse_begin");
    let paragraphs: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let local_qs = crate::import::structure::parse_questions(&paragraphs);
    crate::dbg_log(format!("  local_parse_done local_count={}", local_qs.len()));

    // 2. 本地结果够多就直接用（秒级完成）
    //    条件：找到 >50 题 且（expected=0 或 识别率 >= 50%）
    let local_good = local_qs.len() > 50 && (expected == 0 || local_qs.len() as i64 >= expected * 50 / 100);
    crate::dbg_log(format!("  local_good={}", local_good));
    if local_good {
        crate::dbg_log("  branch_local_direct_insert");
        let _ = app.emit("ai_progress", serde_json::json!({ "done": 1, "total": 1, "engine": "local" }));
        let questions: Vec<crate::db::models::Question> = local_qs.into_iter().map(|pq| {
            crate::import::pipeline::to_question(pq, bank_id)
        }).collect();
        // BUG-002 修复：本地结果为空时拒绝导入
        if questions.is_empty() {
            return Err("本地引擎未识别到任何题目".to_string());
        }
        let count = questions.len() as i64;
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        // BUG-001 修复：使用原子替换
        repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
        crate::dbg_log("  local_replace_done");
        crate::dbg_log(format!("import_with_ai_done(local) count={}", count));
        return Ok(ImportResult { count, expected });
    }

    // 3. 本地结果不够，回退到 AI（加 5 分钟硬超时，超时后用本地结果）
    crate::dbg_log("  branch_ai_fallback");
    let _ = app.emit("ai_progress", serde_json::json!({ "done": 0, "total": 0, "engine": "ai" }));

    // 进度通道
    let (tx, mut rx) = tokio::sync::watch::channel((0usize, 0usize));
    let app_clone = app.clone();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let (done, total) = *rx.borrow();
            let _ = app_clone.emit("ai_progress", serde_json::json!({ "done": done, "total": total }));
            if total > 0 && done >= total {
                break;
            }
        }
    });

    // AI 处理，5 分钟硬超时
    crate::dbg_log("  ai_structurize_begin");
    let ai_result = tokio::time::timeout(
        Duration::from_secs(300),
        crate::import::ai::ai_structurize(&text, &api_key, &base_url, &model, Some(tx), cancel.clone())
    ).await;
    crate::dbg_log("  ai_structurize_returned");

    // BUG-015 修复：超时分支必须设置 cancel 标志，让仍在后台运行的 spawn 任务感知到取消
    // tokio::time::timeout 只让外层 await 返回 Err(Elapsed)，不会取消 spawn 出去的子任务
    // 若不设置 cancel，后台任务会继续请求 AI API 消耗配额，结果被丢弃
    if ai_result.is_err() {
        cancel.store(true, Ordering::SeqCst);
        crate::dbg_log("  timeout_set_cancel_flag");
    }

    // BUG-018 修复：进度事件保留 total 而非重置 0/0，避免 UI 闪烁
    let _ = app.emit("ai_progress", serde_json::json!({ "done": 1, "total": 1 }));

    let qs = match ai_result {
        Ok(Ok(qs)) => {
            crate::dbg_log(format!("  ai_ok count={}", qs.len()));
            qs
        }
        Ok(Err(e)) => {
            crate::dbg_log(format!("  ai_err={} fallback_local={}", e, local_qs.len()));
            // AI 出错，用本地结果兜底
            let questions: Vec<crate::db::models::Question> = local_qs.into_iter().map(|pq| {
                crate::import::pipeline::to_question(pq, bank_id)
            }).collect();
            // BUG-002 修复：本地兜底也为空时拒绝导入
            if questions.is_empty() {
                return Err(format!("AI 解析失败且本地引擎未识别到题目：{}", e));
            }
            let count = questions.len() as i64;
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            // BUG-001 修复：使用原子替换
            repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
            crate::dbg_log(format!("import_with_ai_done(ai_err_fallback) count={}", count));
            return Ok(ImportResult { count, expected });
        }
        Err(_) => {
            crate::dbg_log(format!("  ai_timeout fallback_local={}", local_qs.len()));
            // AI 超时，用本地结果兜底
            let questions: Vec<crate::db::models::Question> = local_qs.into_iter().map(|pq| {
                crate::import::pipeline::to_question(pq, bank_id)
            }).collect();
            // BUG-002 修复：超时兜底也为空时拒绝导入
            if questions.is_empty() {
                return Err("AI 解析超时且本地引擎未识别到题目".to_string());
            }
            let count = questions.len() as i64;
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            // BUG-001 修复：使用原子替换
            repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
            crate::dbg_log(format!("import_with_ai_done(ai_timeout_fallback) count={}", count));
            return Ok(ImportResult { count, expected });
        }
    };

    let questions: Vec<crate::db::models::Question> = qs.into_iter().map(|pq| {
        crate::import::pipeline::to_question(pq, bank_id)
    }).collect();
    // BUG-002 修复：AI 结果为空时拒绝导入
    if questions.is_empty() {
        return Err("AI 未识别到任何题目".to_string());
    }
    let count = questions.len() as i64;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // BUG-001 修复：使用原子替换
    repo::replace_bank_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
    Ok(ImportResult { count, expected })
}
