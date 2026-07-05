use tauri::{State, AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use crate::db::{DbState, repo};
use serde::Serialize;
#[derive(Serialize)]
pub struct DbInfo {
    pub path: String,
    pub size_bytes: u64,
    pub backups_dir: String,
    pub backup_count: usize,
}

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

/// 返回真实数据库路径/大小/备份统计（自动识别用户自定义目录）
#[tauri::command]
pub fn get_db_info(app: AppHandle) -> anyhow::Result<DbInfo, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 关键：必须用 resolve_db_dir，否则用户改过位置后这里永远显示旧路径
    let db_dir = crate::db::resolve_db_dir(&app_data);
    let db_file = db_dir.join("shuati.db");
    let backups_dir = db_dir.join("backups");

    let size_bytes = std::fs::metadata(&db_file)
        .map(|m| m.len())
        .unwrap_or(0);
    let backup_count = std::fs::read_dir(&backups_dir)
        .map(|d| d.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    Ok(DbInfo {
        path: db_file.to_string_lossy().to_string(),
        size_bytes,
        backups_dir: backups_dir.to_string_lossy().to_string(),
        backup_count,
    })
}

/// 在文件管理器中打开数据库所在目录（自动跟随用户自定义）
#[tauri::command]
pub fn open_db_folder(app: AppHandle) -> anyhow::Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_dir = crate::db::resolve_db_dir(&app_data);
    let dir = db_dir.to_string_lossy().to_string();
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

/// 在 Rust 端弹出系统文件夹选择对话框（避免前端 dialog 插件返回值类型差异问题）
/// 阻塞当前线程直到用户选择完毕；返回选中的目录绝对路径，None 表示用户取消
#[tauri::command]
pub fn pick_database_folder(app: AppHandle) -> anyhow::Result<Option<String>, String> {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<Option<String>>();
    app.dialog().file().pick_folder(move |folder_path| {
        let result = folder_path.and_then(|fp| {
            // FilePath 转字符串
            fp.into_path().ok().map(|p| p.to_string_lossy().to_string())
        });
        let _ = tx.send(result);
    });
    // 阻塞等待用户操作（pick_folder 是异步回调）
    let result = rx.recv().map_err(|e| e.to_string())?;
    Ok(result)
}

/// 修改数据库目录。流程：
/// 1. 校验新目录是已存在的文件夹
/// 2. 把当前 db_custom_dir.json 写入 app_data_dir（启动时优先读取）
/// 3. 把当前 shuati.db 复制到新目录（如果新目录里没有）
/// 4. 返回新数据库完整路径
/// 下次启动时，open_db 会优先使用新目录
#[tauri::command]
pub fn change_db_path(db: State<'_, crate::db::DbState>, app: AppHandle, new_dir: String) -> anyhow::Result<String, String> {
    let new_path = std::path::PathBuf::from(new_dir.trim());
    if !new_path.is_dir() {
        return Err(format!("目录不存在或不可访问：{}", new_path.display()));
    }
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let current_db = app_data.join("shuati.db");
    let target_db = new_path.join("shuati.db");

    crate::dbg_log(&format!("[change_db_path] app_data={} new_path={}", app_data.display(), new_path.display()));
    crate::dbg_log(&format!("[change_db_path] current_db={} exists={}", current_db.display(), current_db.exists()));
    crate::dbg_log(&format!("[change_db_path] target_db={} exists={}", target_db.display(), target_db.exists()));

    // 0. 确保 app_data 目录存在
    std::fs::create_dir_all(&app_data).map_err(|e| e.to_string())?;

    // 0.1 关键：WAL 模式下必须先 checkpoint，否则复制出来的 db 可能不完整（丢失 wal/shm 还没刷盘的变更）
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        // PRAGMA wal_checkpoint(TRUNCATE) 强制把所有 wal 内容写回主 db
        if let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
            crate::dbg_log(&format!("[change_db_path] checkpoint warn: {}", e));
        }
    }

    // 1. 写 db_custom_dir.json（让下次启动生效）
    let config_file = app_data.join("db_custom_dir.json");
    let config_content = serde_json::json!({ "dir": new_path.to_string_lossy() });
    std::fs::write(&config_file, serde_json::to_string_pretty(&config_content).unwrap())
        .map_err(|e| format!("写入配置文件失败：{}", e))?;
    crate::dbg_log(&format!("[change_db_path] wrote config to {}", config_file.display()));

    // 2. 把当前数据库复制到新位置（若新位置尚无 db）
    if !target_db.exists() {
        if current_db.exists() {
            std::fs::copy(&current_db, &target_db)
                .map_err(|e| format!("复制数据库失败：{}", e))?;
            crate::dbg_log(&format!("[change_db_path] copied db -> {}", target_db.display()));
        } else {
            crate::dbg_log("[change_db_path] WARN: current_db does not exist, nothing to copy");
        }
    } else {
        crate::dbg_log("[change_db_path] target_db already exists, skip copy");
    }

    // 3. 同步迁移 backups 目录（如果原来在 app_data/backups/）
    let current_backups = app_data.join("backups");
    if current_backups.is_dir() {
        let target_backups = new_path.join("backups");
        if !target_backups.is_dir() {
            let _ = std::fs::create_dir_all(&target_backups);
            // 复制备份文件（仅顶层）
            if let Ok(entries) = std::fs::read_dir(&current_backups) {
                for e in entries.flatten() {
                    if let Ok(name) = e.file_name().into_string() {
                        let from = current_backups.join(&name);
                        let to = target_backups.join(&name);
                        if from.is_file() && !to.exists() {
                            let _ = std::fs::copy(&from, &to);
                        }
                    }
                }
            }
            crate::dbg_log(&format!("[change_db_path] migrated backups -> {}", target_backups.display()));
        }
    }

    Ok(target_db.to_string_lossy().to_string())
}

/// 重启应用：先退出当前进程，由前端/启动器拉起新进程
#[tauri::command]
pub fn restart_app(app: AppHandle) -> anyhow::Result<(), String> {
    // 拿到当前 exe 路径
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();

    // 异步启动新进程（在新进程中启动自己）
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW flag (0x08000000) 避免弹出黑窗
        std::process::Command::new(&exe)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("启动新进程失败：{}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(&exe)
            .spawn()
            .map_err(|e| format!("启动新进程失败：{}", e))?;
    }

    // 关闭当前进程
    app.exit(0);
    Ok(())
}

/// 关闭应用
#[tauri::command]
pub fn quit_app(app: AppHandle) -> anyhow::Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn ocr_available() -> bool {
    crate::import::ocr::tesseract_available()
}

/// P2-18/P0 数据安全：手动备份当前数据库到 backups/ 目录
#[tauri::command]
pub fn backup_database(app: AppHandle, db: State<'_, DbState>) -> anyhow::Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let src = app_data.join("shuati.db");
    if !src.exists() {
        return Err("数据库文件不存在".to_string());
    }
    // BUG-009 修复：复制前强制 WAL checkpoint，确保最新数据已刷盘到主 db 文件
    // 旧实现直接 std::fs::copy 只复制主 db，不复制 wal/shm，备份是旧数据
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").map_err(|e| e.to_string())?;
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
/// BUG-009 修复：复制前强制 WAL checkpoint
pub fn auto_backup_before_import(app: &AppHandle, db: Option<&crate::db::DbState>) -> anyhow::Result<()> {
    let app_data = app.path().app_data_dir()?;
    let src = app_data.join("shuati.db");
    if !src.exists() {
        return Ok(());
    }
    // BUG-009 修复：复制前强制 WAL checkpoint
    if let Some(db_state) = db {
        // 注意：MutexGuard<Connection> 不是 Send，不能用 ? 转换到 anyhow::Error
        // 使用 map_err 转字符串避免 Send/Sync 约束问题
        if let Ok(conn) = db_state.0.lock() {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    } else {
        // 无 DbState 时尝试临时连接做 checkpoint
        if let Ok(conn) = rusqlite::Connection::open(&src) {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
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
