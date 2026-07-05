use rusqlite::{params, Connection};
use anyhow::Result;
use chrono::Utc;
use crate::db::models::*;

pub fn create_bank(conn: &Connection, new_bank: NewBank) -> Result<QuizBank> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO quiz_banks (name, description, question_count, created_at, updated_at) VALUES (?1, ?2, 0, ?3, ?3)",
        params![new_bank.name, new_bank.description, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(QuizBank {
        id, name: new_bank.name, description: new_bank.description,
        question_count: 0, created_at: now.clone(), updated_at: now,
    })
}

pub fn list_banks(conn: &Connection) -> Result<Vec<QuizBank>> {
    let mut stmt = conn.prepare("SELECT id, name, description, question_count, created_at, updated_at FROM quiz_banks ORDER BY updated_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(QuizBank {
            id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
            question_count: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn delete_bank(conn: &Connection, bank_id: i64) -> Result<()> {
    conn.execute("DELETE FROM quiz_banks WHERE id = ?1", params![bank_id])?;
    Ok(())
}

pub fn insert_questions(conn: &Connection, bank_id: i64, questions: &[Question]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for q in questions {
        tx.execute(
            "INSERT INTO questions (bank_id, type, stem, options, answer, analysis, source_index, confidence) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![bank_id, q.q_type, q.stem, q.options, q.answer, q.analysis, q.source_index, q.confidence],
        )?;
    }
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM questions WHERE bank_id=?1", params![bank_id], |r| r.get(0))?;
    let now = Utc::now().to_rfc3339();
    tx.execute("UPDATE quiz_banks SET question_count=?1, updated_at=?2 WHERE id=?3", params![count, now, bank_id])?;
    tx.commit()?;
    Ok(())
}

pub fn list_questions(conn: &Connection, bank_id: i64) -> Result<Vec<Question>> {
    let mut stmt = conn.prepare("SELECT id, bank_id, type, stem, options, answer, analysis, source_index, confidence FROM questions WHERE bank_id=?1 ORDER BY id")?;
    let rows = stmt.query_map(params![bank_id], |row| {
        Ok(Question {
            id: row.get(0)?, bank_id: row.get(1)?, q_type: row.get(2)?, stem: row.get(3)?,
            options: row.get(4)?, answer: row.get(5)?, analysis: row.get(6)?,
            source_index: row.get(7)?, confidence: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// P1-10: 按关键词搜索题目（题干包含查询）
pub fn search_questions(conn: &Connection, bank_id: i64, query: &str, limit: i64) -> Result<Vec<Question>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, bank_id, type, stem, options, answer, analysis, source_index, confidence
         FROM questions
         WHERE bank_id=?1 AND stem LIKE ?2
         ORDER BY id
         LIMIT ?3"
    )?;
    let rows = stmt.query_map(params![bank_id, pattern, limit], |row| {
        Ok(Question {
            id: row.get(0)?, bank_id: row.get(1)?, q_type: row.get(2)?, stem: row.get(3)?,
            options: row.get(4)?, answer: row.get(5)?, analysis: row.get(6)?,
            source_index: row.get(7)?, confidence: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn record_practice(conn: &Connection, r: &PracticeRecord) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO practice_records (bank_id, question_id, user_answer, is_correct, duration_ms, practiced_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![r.bank_id, r.question_id, r.user_answer, r.is_correct as i32, r.duration_ms, r.practiced_at],
    )?;
    if !r.is_correct {
        // upsert 错题本
        tx.execute(
            "INSERT INTO wrong_questions (bank_id, question_id, wrong_count, last_wrong_at, status) VALUES (?1,?2,1,?3,'pending')
             ON CONFLICT(bank_id, question_id) DO UPDATE SET wrong_count=wrong_count+1, last_wrong_at=?3, status='pending'",
            params![r.bank_id, r.question_id, r.practiced_at],
        )?;
    } else {
        // BUG-010 修复：答对则将错题状态置为 mastered（旧值 resolved 在前端不可见，造成"题消失"）
        // list_mastered 仅返回 status='mastered'，resolved 状态的题前后端均无法查看
        tx.execute(
            "UPDATE wrong_questions SET status = 'mastered' WHERE bank_id = ?1 AND question_id = ?2 AND status = 'pending'",
            params![r.bank_id, r.question_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list_wrong(conn: &Connection, bank_id: i64) -> Result<Vec<WrongQuestion>> {
    let mut stmt = conn.prepare("SELECT id, bank_id, question_id, wrong_count, last_wrong_at, status FROM wrong_questions WHERE bank_id=?1 AND status='pending'")?;
    let rows = stmt.query_map(params![bank_id], |row| {
        Ok(WrongQuestion {
            id: row.get(0)?, bank_id: row.get(1)?, question_id: row.get(2)?,
            wrong_count: row.get(3)?, last_wrong_at: row.get(4)?, status: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// P1-7: 列出已掌握错题（status='mastered'）
pub fn list_mastered(conn: &Connection, bank_id: i64) -> Result<Vec<WrongQuestion>> {
    let mut stmt = conn.prepare("SELECT id, bank_id, question_id, wrong_count, last_wrong_at, status FROM wrong_questions WHERE bank_id=?1 AND status='mastered'")?;
    let rows = stmt.query_map(params![bank_id], |row| {
        Ok(WrongQuestion {
            id: row.get(0)?, bank_id: row.get(1)?, question_id: row.get(2)?,
            wrong_count: row.get(3)?, last_wrong_at: row.get(4)?, status: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

/// P1-7: 标记错题为已掌握（status pending -> mastered）
/// BUG-016 修复：限定 status='pending'，避免将 resolved/mastered 状态的题误改为 mastered
pub fn mark_wrong_mastered(conn: &Connection, bank_id: i64, question_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE wrong_questions SET status='mastered' WHERE bank_id=?1 AND question_id=?2 AND status='pending'",
        params![bank_id, question_id],
    )?;
    Ok(())
}

/// BUG-011 修复：把已掌握的错题放回 pending 状态
/// 旧实现通过 recordPractice(is_correct=false) 实现，会写入 practice_records 表，
/// 污染 bank_stats 的 practiced/correct 统计。此函数仅更新 wrong_questions 状态，
/// 不写入练习记录。
pub fn restore_wrong_to_pending(conn: &Connection, bank_id: i64, question_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE wrong_questions SET status='pending', last_wrong_at=?3 WHERE bank_id=?1 AND question_id=?2 AND status='mastered'",
        params![bank_id, question_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// P2-8: 批量清空收藏夹（避免 N 次 IPC）
pub fn clear_favorites(conn: &Connection, bank_id: i64) -> Result<()> {
    conn.execute("DELETE FROM favorites WHERE bank_id=?1", params![bank_id])?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    match conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute("INSERT INTO settings (key, value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", params![key, value])?;
    Ok(())
}

pub fn clear_bank_questions(conn: &Connection, bank_id: i64) -> anyhow::Result<()> {
    // BUG-012 修复：用事务包裹 DELETE + UPDATE，避免部分失败导致数据不一致
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM questions WHERE bank_id = ?1", params![bank_id])?;
    tx.execute("UPDATE quiz_banks SET question_count = 0 WHERE id = ?1", params![bank_id])?;
    tx.commit()?;
    Ok(())
}

/// BUG-001 修复：原子替换题库题目（单事务包裹 DELETE + INSERT + UPDATE）
///
/// 旧实现先 clear_bank_questions 再 insert_questions，两步不在同一事务中，
/// 若 insert 失败则旧题已删除、新题未插入，数据永久丢失。
/// 此函数用单事务包裹，任一步失败整体回滚，保证数据一致性。
pub fn replace_bank_questions(conn: &Connection, bank_id: i64, questions: &[Question]) -> anyhow::Result<()> {
    let tx = conn.unchecked_transaction()?;
    // 1. 删除旧题
    tx.execute("DELETE FROM questions WHERE bank_id = ?1", params![bank_id])?;
    // 2. 插入新题
    for q in questions {
        tx.execute(
            "INSERT INTO questions (bank_id, type, stem, options, answer, analysis, source_index, confidence) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![bank_id, q.q_type, q.stem, q.options, q.answer, q.analysis, q.source_index, q.confidence],
        )?;
    }
    // 3. 更新题库计数
    let count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM questions WHERE bank_id=?1",
        params![bank_id],
        |r| r.get(0),
    )?;
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "UPDATE quiz_banks SET question_count=?1, updated_at=?2 WHERE id=?3",
        params![count, now, bank_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn update_question(conn: &Connection, q: &Question) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE questions SET type=?1, stem=?2, options=?3, answer=?4, analysis=?5 WHERE id=?6",
        params![q.q_type, q.stem, q.options, q.answer, q.analysis, q.id],
    )?;
    Ok(())
}

pub fn bank_stats(conn: &Connection, bank_id: i64) -> Result<BankStats> {
    // P1-5: 三次 COUNT 合并为一次查询，配合 idx_records_bank 索引
    let (total, practiced, correct): (i64, i64, i64) = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM questions WHERE bank_id=?1) AS total,
            (SELECT COUNT(*) FROM practice_records WHERE bank_id=?1) AS practiced,
            (SELECT COUNT(*) FROM practice_records WHERE bank_id=?1 AND is_correct=1) AS correct",
        params![bank_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(BankStats { total, practiced, correct })
}

/// 切换收藏状态：已收藏则取消，未收藏则添加。返回 true 表示当前已收藏。
pub fn toggle_favorite(conn: &Connection, bank_id: i64, question_id: i64) -> Result<bool> {
    let existed: Option<i64> = conn.query_row(
        "SELECT id FROM favorites WHERE bank_id=?1 AND question_id=?2",
        params![bank_id, question_id],
        |r| r.get(0),
    ).ok();
    if let Some(id) = existed {
        conn.execute("DELETE FROM favorites WHERE id=?1", params![id])?;
        Ok(false)
    } else {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO favorites (bank_id, question_id, created_at) VALUES (?1,?2,?3)",
            params![bank_id, question_id, now],
        )?;
        Ok(true)
    }
}

pub fn list_favorites(conn: &Connection, bank_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT question_id FROM favorites WHERE bank_id=?1 ORDER BY id")?;
    let rows = stmt.query_map(params![bank_id], |row| row.get::<_, i64>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn is_favorite(conn: &Connection, bank_id: i64, question_id: i64) -> Result<bool> {
    let exists: Option<i64> = conn.query_row(
        "SELECT 1 FROM favorites WHERE bank_id=?1 AND question_id=?2",
        params![bank_id, question_id],
        |r| r.get(0),
    ).ok();
    Ok(exists.is_some())
}
