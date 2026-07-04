pub mod models;
pub mod schema;
pub mod repo;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

pub fn open_db(app_data_dir: &std::path::Path) -> Result<DbState> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("shuati.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    schema::init_db(&conn)?;
    Ok(DbState(Mutex::new(conn)))
}
