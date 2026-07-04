pub mod models;
pub mod schema;
pub mod repo;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

/// 决定数据库文件应位于哪个目录：
/// 1. 若用户通过设置项 `db_path` 自定义了目录，使用该目录
/// 2. 否则使用 Tauri 的 app_data_dir
///
/// 自定义目录会持久化到一个 `db_custom_dir.json` 配置文件（与 db 同目录的 fallback）
pub fn resolve_db_dir(app_data_dir: &std::path::Path) -> std::path::PathBuf {
    // 优先读取 Tauri app_data_dir 下与 db 同级的 config 文件
    let config_file = app_data_dir.join("db_custom_dir.json");
    if let Ok(content) = std::fs::read_to_string(&config_file) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(dir) = parsed.get("dir").and_then(|v| v.as_str()) {
                let p = std::path::PathBuf::from(dir);
                if p.is_dir() {
                    return p;
                }
            }
        }
    }
    app_data_dir.to_path_buf()
}

pub fn open_db(app_data_dir: &std::path::Path) -> Result<DbState> {
    let db_dir = resolve_db_dir(app_data_dir);
    std::fs::create_dir_all(&db_dir)?;
    let db_path = db_dir.join("shuati.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    schema::init_db(&conn)?;
    Ok(DbState(Mutex::new(conn)))
}
