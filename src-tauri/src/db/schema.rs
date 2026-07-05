pub const CREATE_TABLES_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS quiz_banks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    question_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    stem TEXT NOT NULL,
    options TEXT,
    answer TEXT,
    analysis TEXT,
    source_index INTEGER,
    confidence REAL DEFAULT 1.0
);
CREATE TABLE IF NOT EXISTS practice_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
    question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    user_answer TEXT,
    is_correct INTEGER NOT NULL,
    duration_ms INTEGER,
    practiced_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS wrong_questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
    question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    wrong_count INTEGER DEFAULT 1,
    last_wrong_at TEXT NOT NULL,
    status TEXT DEFAULT 'pending',
    UNIQUE(bank_id, question_id)
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
CREATE TABLE IF NOT EXISTS favorites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
    question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE(bank_id, question_id)
);
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);
CREATE INDEX IF NOT EXISTS idx_questions_bank ON questions(bank_id);
CREATE INDEX IF NOT EXISTS idx_records_question ON practice_records(question_id);
-- P0-6: 新增 bank_id 索引，避免 bank_stats 三次全表扫描
CREATE INDEX IF NOT EXISTS idx_records_bank ON practice_records(bank_id, is_correct);
CREATE INDEX IF NOT EXISTS idx_records_bank_time ON practice_records(bank_id, practiced_at);
CREATE INDEX IF NOT EXISTS idx_wrong_bank ON wrong_questions(bank_id);
CREATE INDEX IF NOT EXISTS idx_favorites_bank ON favorites(bank_id);
CREATE INDEX IF NOT EXISTS idx_wrong_bank_status ON wrong_questions(bank_id, status);
"#;

/// P1-14: 数据库迁移机制
/// 每个迁移函数接收 Connection，执行该版本需要的 DDL/DML。
/// 迁移按顺序执行，从当前版本+1 升到目标版本。
fn migrations() -> Vec<(&'static str, &'static str)> {
    // (版本说明, SQL)
    vec![
        // v1 → v2: 补索引（已通过 CREATE INDEX IF NOT EXISTS 在 CREATE_TABLES_SQL 完成）
        ("v2 add indexes", "CREATE INDEX IF NOT EXISTS idx_records_bank ON practice_records(bank_id, is_correct); CREATE INDEX IF NOT EXISTS idx_records_bank_time ON practice_records(bank_id, practiced_at); CREATE INDEX IF NOT EXISTS idx_wrong_bank_status ON wrong_questions(bank_id, status);"),
        // 后续迁移示例：
        // ("v3 add tags column", "ALTER TABLE questions ADD COLUMN tags TEXT;"),
    ]
}

pub fn init_db(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    // BUG-003 修复：用事务包裹建表语句，DDL 部分失败时整体回滚
    // 旧实现 execute_batch 隐式提交前 N 张表，第 N+1 张失败时数据库处于半迁移状态
    {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(CREATE_TABLES_SQL)?;
        tx.commit()?;
    }
    run_migrations(conn)?;
    Ok(())
}

/// 执行所有未应用的迁移
fn run_migrations(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    // 确保 schema_version 表存在
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);")?;

    // 读取当前版本（MAX 可能返回 NULL，用 Option<i64> 接收）
    let current: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get::<_, Option<i64>>(0))
        .unwrap_or(None)
        .unwrap_or(0);

    let migs = migrations();
    let target = migs.len() as i64;

    if current >= target {
        return Ok(());
    }

    // 执行 current+1 .. target 的迁移
    for (i, (desc, sql)) in migs.iter().enumerate() {
        let v = (i + 1) as i64;
        if v <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", rusqlite::params![v])?;
        tx.commit()?;
        eprintln!("[DB] migration {} applied: {}", v, desc);
    }

    Ok(())
}
