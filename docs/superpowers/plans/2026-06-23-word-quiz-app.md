# 刷题宝 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建 Tauri 桌面应用，把 Word 题库一键识别（本地 OCR + AI 增强）转换成可点击选项刷题，含多题库管理、答题记录、错题本。

**Architecture:** Tauri 2.x（Rust 后端 + Vue 3 前端）。Rust 负责 Word 解析、OCR、AI 调用、SQLite 存储；Vue 负责导入向导、刷题交互、错题本 UI。前后端通过 Tauri command 通信。

**Tech Stack:** Tauri 2.x, Rust, Vue 3 + TypeScript + Vite + Pinia + Vue Router, SQLite (rusqlite), mammoth.js, Tesseract OCR, AI 大模型 API (reqwest)

**Spec:** `docs/superpowers/specs/2026-06-23-word-quiz-app-design.md`

---

## 文件结构总览

```
src-tauri/
  Cargo.toml
  tauri.conf.json
  src/
    main.rs                      # Tauri 入口，注册 commands
    lib.rs                       # 模块声明
    db/
      mod.rs                     # 模块声明
      schema.rs                  # 建表 SQL + 迁移
      models.rs                  # 数据结构（Question/Bank/Record...）
      repo.rs                    # CRUD 操作
    import/
      mod.rs
      docx.rs                    # mammoth.js 调用桥接（.docx → HTML+图片）
      structure.rs               # 题型识别 + 答案关联（核心，TDD）
      pipeline.rs                # 导入流水线整合
      ocr.rs                     # Tesseract OCR
      ai.rs                      # AI 大模型识别
    commands/
      mod.rs
      bank.rs                    # 题库 command
      question.rs                # 题目 command
      import.rs                  # 导入 command
      practice.rs                # 刷题/记录 command
src/
  main.ts
  App.vue
  router/index.ts
  stores/
    bank.ts                      # 题库列表状态
    practice.ts                  # 刷题会话状态
    settings.ts                  # 设置（AI Key、引擎选择）
  views/
    HomeView.vue                 # 题库列表
    ImportView.vue               # 导入向导
    PracticeView.vue             # 刷题页
    WrongView.vue                # 错题本
    StatsView.vue                # 统计
    SettingsView.vue             # 设置
  components/
    QuestionCard.vue             # 题目展示+选项
    OptionButton.vue             # 单个选项按钮
    ImportReviewTable.vue        # 导入校验表格
    ProgressBar.vue
  utils/
    api.ts                       # 封装 Tauri invoke 调用
```

---

## 阶段 0：环境搭建

### Task 1: 安装 Rust 工具链与 Tauri 依赖

**Files:** 无（环境配置）

- [ ] **Step 1: 安装 Rust**

下载并运行 `https://win.rustup.rs/x86_64`（rustup-init.exe），按默认选项安装。安装完成后重启终端。

Run: `rustc --version`
Expected: 输出 `rustc 1.xx.x` 版本号

- [ ] **Step 2: 安装 Microsoft C++ Build Tools**

Tauri Windows 依赖 MSVC。访问 `https://visualstudio.microsoft.com/visual-cpp-build-tools/` 下载 Build Tools，安装时勾选「使用 C++ 的桌面开发」工作负载。需重启。

- [ ] **Step 3: 安装 Tauri CLI**

Run: `npm install -D @tauri-apps/cli@latest`
（后续在项目内安装，此步可省略，Task 2 会随项目创建带入）

- [ ] **Step 4: 验证 WebView2**

Windows 10/11 通常预装 WebView2。如未安装，从 `https://developer.microsoft.com/microsoft-edge/webview2/` 下载 Evergreen Bootstrapper。

---

## 阶段 1：项目脚手架与数据层

### Task 2: 创建 Tauri + Vue 项目

**Files:**
- Create: 整个项目骨架（由 create-tauri-app 生成）
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`

- [ ] **Step 1: 用 create-tauri-app 初始化项目**

在 `d:\桌面\刷题宝` 目录内初始化（目录已有 .git 和 docs，用 `.` 当前目录）：

Run: `npm create tauri-app@latest . -- --template vue-ts --manager npm --name 刷题宝`
（如交互式提示，选 Vue + TypeScript）

若因目录非空报错，先临时移走 docs 和 .git 到上级，初始化后再移回；或手动 `npm create tauri-app@latest shuati-bao-tmp` 后把内容拷入。

- [ ] **Step 2: 安装前端依赖**

Run: `npm install`

安装业务依赖：
Run: `npm install vue-router pinia mammoth`
Run: `npm install -D @types/mammoth`

- [ ] **Step 3: 添加 Rust 依赖**

编辑 `src-tauri/Cargo.toml`，在 `[dependencies]` 加入：

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
reqwest = { version = "0.12", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
base64 = "0.22"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
```

- [ ] **Step 4: 验证项目能启动**

Run: `npm run tauri dev`
Expected: 弹出 Tauri 窗口，显示默认 Vue 欢迎页。首次编译 Rust 较慢（数分钟）。

- [ ] **Step 5: 配置 tauri.conf.json**

修改 `src-tauri/tauri.conf.json`，设置 `productName` 为 `刷题宝`，`identifier` 为 `com.shuati-bao.app`，允许 fs/dialog 插件权限。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: 初始化 Tauri+Vue 项目骨架"
```

### Task 3: 数据库 Schema 与迁移

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `schema.rs`, `models.rs`, `repo.rs`
- Modify: `src-tauri/src/lib.rs`, `main.rs`

- [ ] **Step 1: 编写数据模型 models.rs**

Create `src-tauri/src/db/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizBank {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub question_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub bank_id: i64,
    #[serde(rename = "type")]
    pub q_type: String, // single | multi | judge | blank
    pub stem: String,
    pub options: Option<String>, // JSON 数组
    pub answer: Option<String>,  // 单选"A"；多选["A","C"]；判断"true"；填空["x"]
    pub analysis: Option<String>,
    pub source_index: Option<i64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeRecord {
    pub id: i64,
    pub bank_id: i64,
    pub question_id: i64,
    pub user_answer: Option<String>,
    pub is_correct: bool,
    pub duration_ms: Option<i64>,
    pub practiced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrongQuestion {
    pub id: i64,
    pub bank_id: i64,
    pub question_id: i64,
    pub wrong_count: i64,
    pub last_wrong_at: String,
    pub status: String, // pending | mastered
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBank {
    pub name: String,
    pub description: Option<String>,
}
```

- [ ] **Step 2: 编写 schema.rs**

Create `src-tauri/src/db/schema.rs`:

```rust
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
    status TEXT DEFAULT 'pending'
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
CREATE INDEX IF NOT EXISTS idx_questions_bank ON questions(bank_id);
CREATE INDEX IF NOT EXISTS idx_records_question ON practice_records(question_id);
CREATE INDEX IF NOT EXISTS idx_wrong_bank ON wrong_questions(bank_id);
"#;

pub fn init_db(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    conn.execute_batch(CREATE_TABLES_SQL)?;
    Ok(())
}
```

- [ ] **Step 3: 编写 repo.rs（CRUD）**

Create `src-tauri/src/db/repo.rs`:

```rust
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

pub fn record_practice(conn: &Connection, r: &PracticeRecord) -> Result<()> {
    conn.execute(
        "INSERT INTO practice_records (bank_id, question_id, user_answer, is_correct, duration_ms, practiced_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![r.bank_id, r.question_id, r.user_answer, r.is_correct as i32, r.duration_ms, r.practiced_at],
    )?;
    if !r.is_correct {
        // upsert 错题本
        conn.execute(
            "INSERT INTO wrong_questions (bank_id, question_id, wrong_count, last_wrong_at, status) VALUES (?1,?2,1,?3,'pending')
             ON CONFLICT(bank_id, question_id) DO UPDATE SET wrong_count=wrong_count+1, last_wrong_at=?3, status='pending'",
            params![r.bank_id, r.question_id, r.practiced_at],
        )?;
    }
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

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let res: Option<String> = conn.query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| r.get(0)).ok();
    Ok(res)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute("INSERT INTO settings (key, value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", params![key, value])?;
    Ok(())
}
```

- [ ] **Step 4: 编写 db/mod.rs 与数据库连接管理**

Create `src-tauri/src/db/mod.rs`:

```rust
pub mod models;
pub mod schema;
pub mod repo;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct DbState(pub Mutex<Connection>);

pub fn open_db(app_data_dir: &std::path::Path) -> Result<DbState> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("shuati.db");
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    schema::init_db(&conn)?;
    Ok(DbState(Mutex::new(conn)))
}
```

- [ ] **Step 5: 在 lib.rs 注册数据库状态**

Create/Modify `src-tauri/src/lib.rs`:

```rust
pub mod db;
pub mod import;
pub mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("no app data dir");
            let db_state = db::open_db(&app_data).expect("failed to open db");
            app.manage(db_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bank::list_banks,
            commands::bank::create_bank,
            commands::bank::delete_bank,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: 编写 commands/bank.rs**

Create `src-tauri/src/commands/mod.rs`:
```rust
pub mod bank;
pub mod question;
pub mod import;
pub mod practice;
```

Create `src-tauri/src/commands/bank.rs`:
```rust
use tauri::State;
use crate::db::{DbState, models::*, repo};

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
```

- [ ] **Step 7: 编译验证**

Run: `npm run tauri dev`
Expected: 编译通过，窗口启动（commands/question 等模块先建空壳避免编译错误）。

为占位模块创建空文件：`src-tauri/src/commands/question.rs`, `import.rs`, `practice.rs` 各写 `// placeholder`，`src-tauri/src/import/mod.rs` 写 `pub mod docx; pub mod structure; pub mod pipeline; pub mod ocr; pub mod ai;` 并建对应空文件。

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: 数据库层与题库 CRUD"
```

---

## 阶段 2：Word 导入核心（.docx 文本路径）

### Task 4: .docx 解析模块

**Files:**
- Create: `src-tauri/src/import/docx.rs`
- Test: `src-tauri/src/import/docx.rs` (内联 #[cfg(test)])

说明：mammoth.js 是 JS 库，在 Tauri 中通过前端调用 mammoth 转 HTML，再把 HTML+图片传给 Rust 做结构化。因此 docx.rs 提供「接收 HTML 文本，提取纯文本+图片占位」的工具。

- [ ] **Step 1: 编写 HTML 拆解工具**

Create `src-tauri/src/import/docx.rs`:

```rust
use anyhow::Result;

/// 从 mammoth 输出的 HTML 中提取段落文本列表与图片 base64 列表。
/// 图片以 <img src="data:image/png;base64,xxx"> 形式内嵌。
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    /// 段落列表（已去标签），图片位置用 `[IMG:索引]` 占位
    pub paragraphs: Vec<String>,
    /// 图片 base64（不含 data: 前缀）
    pub images: Vec<String>,
}

pub fn parse_html(html: &str) -> Result<ParsedDoc> {
    let mut paragraphs = Vec::new();
    let mut images = Vec::new();

    // 按 <p> 切段（mammoth 默认段落标签）
    let mut current = String::new();
    let mut in_p = false;
    let mut chars = html.chars().peekable();
    let raw: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < raw.len() {
        let rest: String = raw[i..].iter().collect();
        if rest.starts_with("<p") {
            in_p = true;
            // 跳过到 >
            while i < raw.len() && raw[i] != '>' { i += 1; }
            i += 1;
            continue;
        }
        if rest.starts_with("</p>") {
            in_p = false;
            let text = strip_tags(&current).trim().to_string();
            if !text.is_empty() {
                paragraphs.push(text);
            }
            current.clear();
            i += 4;
            continue;
        }
        // 图片处理
        if rest.starts_with("<img") {
            // 找到 src="data:image/...;base64,XXX"
            let end = rest.find('>').unwrap_or(rest.len());
            let img_tag = &rest[..end];
            if let Some(idx) = img_tag.find("base64,") {
                let after = &img_tag[idx + 7..];
                let b64: String = after.chars().take_while(|c| *c != '"' && *c != ' ').collect();
                if !b64.is_empty() {
                    images.push(b64);
                    current.push_str(&format!("[IMG:{}]", images.len() - 1));
                }
            }
            i += end;
            continue;
        }
        if in_p {
            current.push(raw[i]);
        }
        i += 1;
    }
    // 兜底：若无 <p>，按 <br> 或换行切
    if paragraphs.is_empty() && !current.trim().is_empty() {
        for line in strip_tags(&current).split('\n') {
            let t = line.trim();
            if !t.is_empty() { paragraphs.push(t.to_string()); }
        }
    }
    Ok(ParsedDoc { paragraphs, images })
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_paragraphs() {
        let html = "<p>1. 以下哪个是A？</p><p>A. 选项一</p><p>B. 选项二</p>";
        let parsed = parse_html(html).unwrap();
        assert_eq!(parsed.paragraphs.len(), 3);
        assert_eq!(parsed.paragraphs[0], "1. 以下哪个是A？");
        assert!(parsed.images.is_empty());
    }

    #[test]
    fn test_parse_with_image() {
        let html = "<p>题干 <img src=\"data:image/png;base64,abc123==\" /></p>";
        let parsed = parse_html(html).unwrap();
        assert_eq!(parsed.paragraphs.len(), 1);
        assert!(parsed.paragraphs[0].contains("[IMG:0]"));
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0], "abc123==");
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cd src-tauri && cargo test docx -- --nocapture`
Expected: 2 个测试通过。

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: .docx HTML 解析（段落+图片提取）"
```

### Task 5: 结构化解析 - 题型识别（TDD 核心）

**Files:**
- Create: `src-tauri/src/import/structure.rs`

这是项目核心逻辑，用 TDD 严格覆盖。

- [ ] **Step 1: 定义题目结构与识别接口**

Create `src-tauri/src/import/structure.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QType {
    Single,  // 单选
    Multi,   // 多选
    Judge,   // 判断
    Blank,   // 填空
    Qa,      // 问答
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuestion {
    pub q_type: QType,
    pub stem: String,
    pub options: Vec<String>,      // 选择题选项
    pub answer: Option<String>,    // 标准化答案
    pub analysis: Option<String>,
    pub source_index: usize,
    pub confidence: f64,
}

/// 从段落列表识别并切分出题目。
pub fn parse_questions(paragraphs: &[String]) -> Vec<ParsedQuestion> {
    // Step 2 起逐步实现
    Vec::new()
}
```

- [ ] **Step 2: 写失败测试 - 识别题号切分题目**

在 `structure.rs` 末尾加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String { s.to_string() }

    #[test]
    fn test_split_by_question_number() {
        let paras = vec![
            p("1. 题目一内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案：A"),
            p("2. 题目二内容？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案：B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].stem, "题目一内容？");
        assert_eq!(qs[1].stem, "题目二内容？");
    }
}
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cd src-tauri && cargo test structure::tests::test_split_by_question_number`
Expected: FAIL（返回空 Vec，长度 0 != 2）

- [ ] **Step 4: 实现题号切分**

替换 `parse_questions`：

```rust
use regex::Regex;

pub fn parse_questions(paragraphs: &[String]) -> Vec<ParsedQuestion> {
    let re_num = Regex::new(r"^\s*(\d+)[\.、．]\s*(.+)").unwrap();
    let mut questions = Vec::new();
    let mut current: Option<Vec<String>> = None;
    let mut current_idx: usize = 0;

    for para in paragraphs {
        if let Some(caps) = re_num.captures(para) {
            // 新题开始：先保存上一题
            if let Some(blocks) = current.take() {
                if let Some(q) = build_question(&blocks, current_idx) {
                    questions.push(q);
                }
            }
            let num: usize = caps[1].parse().unwrap_or(0);
            current_idx = num;
            current = Some(vec![para.clone()]);
        } else if let Some(blocks) = current.as_mut() {
            blocks.push(para.clone());
        }
    }
    if let Some(blocks) = current.take() {
        if let Some(q) = build_question(&blocks, current_idx) {
            questions.push(q);
        }
    }
    questions
}

/// 从一道题的段落块构建 ParsedQuestion
fn build_question(blocks: &[String], source_index: usize) -> Option<ParsedQuestion> {
    if blocks.is_empty() { return None; }
    // 第一段是题号+题干
    let re_num = Regex::new(r"^\s*\d+[\.、．]\s*(.+)").unwrap();
    let stem = if let Some(caps) = re_num.captures(&blocks[0]) {
        caps[1].to_string()
    } else {
        blocks[0].clone()
    };

    let re_opt = Regex::new(r"^\s*([A-D])[\.、．]\s*(.+)").unwrap();
    let re_ans = Regex::new(r"(?i)^\s*(答案|正确答案|【答案】)\s*[:：]?\s*(.+)").unwrap();
    let re_ana = Regex::new(r"(?i)^\s*(解析|答案解析|【解析】)\s*[:：]?\s*(.+)").unwrap();

    let mut options = Vec::new();
    let mut answer: Option<String> = None;
    let mut analysis: Option<String> = None;

    for line in &blocks[1..] {
        if let Some(caps) = re_opt.captures(line) {
            options.push(caps[2].to_string());
        } else if let Some(caps) = re_ans.captures(line) {
            answer = Some(caps[2].trim().to_string());
        } else if let Some(caps) = re_ana.captures(line) {
            analysis = Some(caps[2].trim().to_string());
        }
    }

    // 题型判定
    let q_type = detect_type(&stem, &options, &answer);

    Some(ParsedQuestion {
        q_type,
        stem,
        options,
        answer: answer.map(|a| normalize_answer(&a)),
        analysis,
        source_index,
        confidence: 0.85,
    })
}

fn detect_type(stem: &str, options: &[String], answer: &Option<String>) -> QType {
    // 判断题：无选项，或答案为 正确/错误/对/错/√/×/T/F
    let judge_pat = Regex::new(r"(?i)(正确|错误|对|错|√|×|T|F|true|false)").unwrap();
    if options.is_empty() {
        if let Some(ans) = answer {
            if judge_pat.is_match(ans) { return QType::Judge; }
        }
        // 填空：题干含 ___ 或 (  ) 或 【  】
        let blank_pat = Regex::new(r"_{2,}|（\s*）|\(\s*\)|【\s*】").unwrap();
        if blank_pat.is_match(stem) { return QType::Blank; }
        return QType::Qa;
    }
    // 选择题：按答案数量区分单选/多选
    if let Some(ans) = answer {
        let letters: Vec<char> = ans.chars().filter(|c| matches!(c, 'A'..='D' | 'a'..='d')).map(|c| c.to_ascii_uppercase()).collect();
        if letters.len() > 1 { return QType::Multi; }
    }
    QType::Single
}

fn normalize_answer(ans: &str) -> String {
    let trimmed = ans.trim();
    // 判断题归一
    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        "正确" | "对" | "√" | "t" | "true" => return "true".to_string(),
        "错误" | "错" | "×" | "f" | "false" => return "false".to_string(),
        _ => {}
    }
    // 多选答案如 "AC" → ["A","C"]
    let letters: Vec<char> = trimmed.chars().filter(|c| matches!(c, 'A'..='D' | 'a'..='d')).map(|c| c.to_ascii_uppercase()).collect();
    if letters.len() > 1 {
        return serde_json::to_string(&letters).unwrap_or_else(|_| trimmed.to_string());
    }
    trimmed.to_string()
}
```

需在 `Cargo.toml` 加 `regex = "1"`。

- [ ] **Step 5: 运行测试确认通过**

Run: `cd src-tauri && cargo test structure`
Expected: 通过。

- [ ] **Step 6: 补充测试 - 多选、判断、填空**

追加测试：

```rust
    #[test]
    fn test_multi_choice() {
        let paras = vec![
            p("1. 多选题目？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("C. 选项C"),
            p("D. 选项D"),
            p("答案：AC"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Multi);
        assert_eq!(qs[0].answer.as_deref(), Some("[\"A\",\"C\"]"));
    }

    #[test]
    fn test_judge() {
        let paras = vec![
            p("1. 地球是圆的。"),
            p("答案：正确"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Judge);
        assert_eq!(qs[0].answer.as_deref(), Some("true"));
    }

    #[test]
    fn test_blank() {
        let paras = vec![
            p("1. 中国的首都是____。"),
            p("答案：北京"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].q_type, QType::Blank);
    }
```

- [ ] **Step 7: 运行全部测试**

Run: `cd src-tauri && cargo test structure`
Expected: 4 个测试全通过。

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: 题型识别与答案归一（单选/多选/判断/填空）"
```

### Task 6: 答案关联（文末答案表模式）

**Files:**
- Modify: `src-tauri/src/import/structure.rs`

- [ ] **Step 1: 写失败测试 - 文末答案表**

追加测试：

```rust
    #[test]
    fn test_answer_key_at_end() {
        // 题目无内联答案，文末有答案表
        let paras = vec![
            p("1. 题目一？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("2. 题目二？"),
            p("A. 选项A"),
            p("B. 选项B"),
            p("答案"),
            p("1. A"),
            p("2. B"),
        ];
        let qs = parse_questions(&paras);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].answer.as_deref(), Some("A"));
        assert_eq!(qs[1].answer.as_deref(), Some("B"));
    }
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test test_answer_key_at_end`
Expected: FAIL（答案为 None）

- [ ] **Step 3: 实现文末答案表关联**

修改 `parse_questions`，在收集完所有题目后，扫描「答案区」并回填。在 `parse_questions` 函数体最后、返回前插入：

```rust
    // 文末答案表关联：扫描未被 build_question 消化的「答案区」段落
    // 重新遍历，找出以 "答案"/"参考答案"/"Answer" 开头之后的段落
    let ans_header = Regex::new(r"(?i)^\s*(参考答案|答案|answer)\s*$").unwrap();
    let ans_line = Regex::new(r"^\s*(\d+)[\.、．]\s*([A-Da-d]+|正确|错误|对|错|√|×)").unwrap();
    let mut answer_map: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    let mut in_answer_section = false;
    for para in paragraphs {
        if ans_header.is_match(para) { in_answer_section = true; continue; }
        if in_answer_section {
            if let Some(caps) = ans_line.captures(para) {
                let idx: usize = caps[1].parse().unwrap_or(0);
                answer_map.insert(idx, caps[2].to_string());
            }
        }
    }
    // 回填
    for q in questions.iter_mut() {
        if q.answer.is_none() {
            if let Some(a) = answer_map.get(&q.source_index) {
                q.answer = Some(normalize_answer(a));
                // 重新判定题型（可能原本因无答案判为 Qa）
                q.q_type = detect_type(&q.stem, &q.options, &q.answer);
            }
        }
    }
```

- [ ] **Step 4: 运行测试**

Run: `cd src-tauri && cargo test structure`
Expected: 全部通过（含文末答案表）。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: 文末答案表关联"
```

### Task 7: 导入流水线整合（Tauri command）

**Files:**
- Create: `src-tauri/src/import/pipeline.rs`
- Create: `src-tauri/src/commands/import.rs`
- Modify: `src-tauri/src/lib.rs`（注册 command）

- [ ] **Step 1: 编写 pipeline.rs**

Create `src-tauri/src/import/pipeline.rs`:

```rust
use crate::db::models::Question;
use crate::import::docx::parse_html;
use crate::import::structure::{parse_questions, ParsedQuestion, QType};
use anyhow::Result;

/// 把 mammoth 输出的 HTML 转成可入库的 Question 列表
pub fn html_to_questions(html: &str, bank_id: i64) -> Result<Vec<Question>> {
    let parsed = parse_html(html)?;
    let parsed_qs = parse_questions(&parsed.paragraphs);
    let qs = parsed_qs.into_iter().map(|pq| to_question(pq, bank_id)).collect();
    Ok(qs)
}

fn to_question(pq: ParsedQuestion, bank_id: i64) -> Question {
    let q_type = match pq.q_type {
        QType::Single => "single",
        QType::Multi => "multi",
        QType::Judge => "judge",
        QType::Blank => "blank",
        QType::Qa => "qa",
    }.to_string();
    let options = if pq.options.is_empty() { None } else { serde_json::to_string(&pq.options).ok() };
    Question {
        id: 0, bank_id, q_type, stem: pq.stem, options, answer: pq.answer,
        analysis: pq.analysis, source_index: Some(pq.source_index as i64), confidence: pq.confidence,
    }
}
```

- [ ] **Step 2: 编写 commands/import.rs**

Create `src-tauri/src/commands/import.rs`:

```rust
use tauri::State;
use crate::db::{DbState, repo};
use crate::import::pipeline::html_to_questions;

/// 前端调用 mammoth 转 HTML 后，把 HTML 传入此 command 做结构化并入库
#[tauri::command]
pub fn import_from_html(db: State<'_, DbState>, bank_id: i64, html: String) -> anyhow::Result<i64, String> {
    let questions = html_to_questions(&html, bank_id).map_err(|e| e.to_string())?;
    let count = questions.len() as i64;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::insert_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
    Ok(count)
}
```

- [ ] **Step 3: 注册 command**

修改 `src-tauri/src/lib.rs` 的 `invoke_handler`，加入 `commands::import::import_from_html`。

- [ ] **Step 4: 编译验证**

Run: `npm run tauri dev`
Expected: 编译通过。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: 导入流水线 command（HTML→结构化→入库）"
```

---

## 阶段 3：刷题主流程 UI

### Task 8: 前端路由与状态管理骨架

**Files:**
- Create: `src/router/index.ts`, `src/stores/bank.ts`, `src/utils/api.ts`
- Modify: `src/main.ts`, `src/App.vue`

- [ ] **Step 1: 配置 main.ts**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './style.css'

createApp(App).use(createPinia()).use(router).mount('#app')
```

- [ ] **Step 2: 编写 utils/api.ts（封装 invoke）**

```typescript
import { invoke } from '@tauri-apps/api/core'

export interface QuizBank {
  id: number; name: string; description: string | null;
  question_count: number; created_at: string; updated_at: string;
}
export interface Question {
  id: number; bank_id: number; type: string; stem: string;
  options: string | null; answer: string | null; analysis: string | null;
  source_index: number | null; confidence: number;
}
export interface NewBank { name: string; description: string | null }

export const api = {
  listBanks: () => invoke<QuizBank[]>('list_banks'),
  createBank: (b: NewBank) => invoke<QuizBank>('create_bank', { newBank: b }),
  deleteBank: (id: number) => invoke<void>('delete_bank', { bankId: id }),
  listQuestions: (bankId: number) => invoke<Question[]>('list_questions', { bankId }),
  importFromHtml: (bankId: number, html: string) => invoke<number>('import_from_html', { bankId, html }),
  recordPractice: (r: { bank_id: number; question_id: number; user_answer: string | null; is_correct: boolean; duration_ms: number | null }) =>
    invoke<void>('record_practice', { record: { ...r, practiced_at: new Date().toISOString() } }),
  listWrong: (bankId: number) => invoke<{ question_id: number }[]>('list_wrong', { bankId }),
}
```

需在 `commands/question.rs` 和 `practice.rs` 补充对应 command（见 Task 11）。

- [ ] **Step 3: 编写 router/index.ts**

```typescript
import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'home', component: () => import('../views/HomeView.vue') },
  { path: '/import/:bankId?', name: 'import', component: () => import('../views/ImportView.vue') },
  { path: '/practice/:bankId', name: 'practice', component: () => import('../views/PracticeView.vue') },
  { path: '/wrong/:bankId', name: 'wrong', component: () => import('../views/WrongView.vue') },
  { path: '/stats/:bankId', name: 'stats', component: () => import('../views/StatsView.vue') },
  { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue') },
]

export default createRouter({ history: createWebHistory(), routes })
```

- [ ] **Step 4: 编写 stores/bank.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, QuizBank } from '../utils/api'

export const useBankStore = defineStore('bank', () => {
  const banks = ref<QuizBank[]>([])
  const loading = ref(false)

  async function load() {
    loading.value = true
    banks.value = await api.listBanks()
    loading.value = false
  }
  async function create(name: string, description: string | null) {
    const b = await api.createBank({ name, description })
    banks.value.unshift(b)
    return b
  }
  async function remove(id: number) {
    await api.deleteBank(id)
    banks.value = banks.value.filter(b => b.id !== id)
  }
  return { banks, loading, load, create, remove }
})
```

- [ ] **Step 5: 编写 App.vue（侧边栏导航）**

```vue
<template>
  <div class="app">
    <aside class="sidebar">
      <h1>刷题宝</h1>
      <nav>
        <RouterLink to="/">题库</RouterLink>
        <RouterLink to="/settings">设置</RouterLink>
      </nav>
    </aside>
    <main class="content"><RouterView /></main>
  </div>
</template>

<script setup lang="ts">
import { RouterView, RouterLink } from 'vue-router'
</script>

<style>
.app { display: flex; height: 100vh; }
.sidebar { width: 200px; background: #f5f5f5; padding: 16px; border-right: 1px solid #ddd; }
.sidebar nav { display: flex; flex-direction: column; gap: 8px; margin-top: 16px; }
.sidebar nav a { text-decoration: none; color: #333; padding: 8px; border-radius: 6px; }
.sidebar nav a.router-link-active { background: #42b883; color: #fff; }
.content { flex: 1; overflow: auto; padding: 24px; }
</style>
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: 前端路由与状态骨架"
```

### Task 9: 题库列表页 HomeView

**Files:**
- Create: `src/views/HomeView.vue`

- [ ] **Step 1: 编写 HomeView.vue**

```vue
<template>
  <div class="home">
    <div class="header">
      <h2>我的题库</h2>
      <button @click="showNew = true">+ 新建题库</button>
    </div>
    <div v-if="bankStore.loading">加载中...</div>
    <div v-else class="grid">
      <div v-for="b in bankStore.banks" :key="b.id" class="card">
        <h3>{{ b.name }}</h3>
        <p>{{ b.question_count }} 题</p>
        <div class="actions">
          <button @click="$router.push(`/practice/${b.id}`)">开始刷题</button>
          <button @click="$router.push(`/import/${b.id}`)">导入</button>
          <button @click="$router.push(`/wrong/${b.id}`)">错题本</button>
          <button @click="del(b.id)">删除</button>
        </div>
      </div>
    </div>
    <div v-if="showNew" class="modal">
      <div class="modal-body">
        <h3>新建题库</h3>
        <input v-model="newName" placeholder="题库名称" />
        <textarea v-model="newDesc" placeholder="描述（可选）"></textarea>
        <div class="modal-actions">
          <button @click="create">确定</button>
          <button @click="showNew = false">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useBankStore } from '../stores/bank'

const bankStore = useBankStore()
const showNew = ref(false)
const newName = ref('')
const newDesc = ref('')

onMounted(() => bankStore.load())

async function create() {
  if (!newName.value.trim()) return
  await bankStore.create(newName.value.trim(), newDesc.value || null)
  showNew.value = false
  newName.value = ''
  newDesc.value = ''
}
async function del(id: number) {
  if (confirm('确认删除该题库？')) await bankStore.remove(id)
}
</script>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 16px; }
.card { border: 1px solid #ddd; border-radius: 8px; padding: 16px; background: #fff; }
.card .actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; }
button { padding: 6px 12px; border: 1px solid #ddd; border-radius: 6px; background: #fff; cursor: pointer; }
button:hover { background: #f0f0f0; }
.modal { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
.modal-body { background: #fff; padding: 24px; border-radius: 8px; min-width: 320px; display: flex; flex-direction: column; gap: 8px; }
input, textarea { padding: 8px; border: 1px solid #ddd; border-radius: 6px; }
</style>
```

- [ ] **Step 2: 验证**

Run: `npm run tauri dev`
Expected: 首页显示题库列表，可新建/删除题库。

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: 题库列表页"
```

### Task 10: 导入向导 ImportView

**Files:**
- Create: `src/views/ImportView.vue`

- [ ] **Step 1: 编写 ImportView.vue**

```vue
<template>
  <div class="import">
    <h2>导入题库</h2>
    <div v-if="bankId" class="target">导入到：{{ bankName }}</div>

    <div class="step" v-if="step === 1">
      <h3>步骤1：选择 Word 文件</h3>
      <button @click="pickFile">选择 .docx 文件</button>
      <p v-if="fileName">{{ fileName }}</p>
    </div>

    <div class="step" v-if="step === 2">
      <h3>步骤2：识别中...</h3>
      <p>{{ status }}</p>
    </div>

    <div class="step" v-if="step === 3">
      <h3>步骤3：校验识别结果（{{ reviewList.length }} 题）</h3>
      <ImportReviewTable :list="reviewList" @update="onUpdate" />
      <div class="actions">
        <button @click="confirmImport">确认导入</button>
        <button @click="step = 1">重新选择</button>
      </div>
    </div>

    <div class="step" v-if="step === 4">
      <h3>导入成功！共 {{ importedCount }} 题</h3>
      <button @click="$router.push(`/practice/${bankId}`)">开始刷题</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import mammoth from 'mammoth'
import { api, Question } from '../utils/api'
import { useBankStore } from '../stores/bank'
import ImportReviewTable from '../components/ImportReviewTable.vue'

const route = useRoute()
const router = useRouter()
const bankStore = useBankStore()
const bankId = ref(Number(route.params.bankId) || 0)
const bankName = ref('')
const step = ref(1)
const fileName = ref('')
const status = ref('')
const reviewList = ref<Question[]>([])
const importedCount = ref(0)

if (!bankId.value) {
  // 未指定题库，先创建一个
  const name = prompt('请输入题库名称')
  if (name) {
    const b = await bankStore.create(name, null)
    bankId.value = b.id
    bankName.value = b.name
  } else {
    router.push('/')
  }
} else {
  bankName.value = bankStore.banks.find(b => b.id === bankId.value)?.name || ''
}

async function pickFile() {
  const selected = await open({ filters: [{ name: 'Word', extensions: ['docx'] }] })
  if (!selected || Array.isArray(selected)) return
  fileName.value = selected as string
  step.value = 2
  status.value = '读取文件中...'
  try {
    const arrayBuffer = await readFileAsArray(selected as string)
    status.value = '解析文档中...'
    const result = await mammoth.convertToHtml({ arrayBuffer })
    status.value = '结构化识别中...'
    // 调用后端结构化（不入库，先返回校验）
    // 为简化，这里直接调 import_from_html 入库后再 list 出来校验编辑
    const count = await api.importFromHtml(bankId.value, result.value)
    reviewList.value = await api.listQuestions(bankId.value)
    step.value = 3
  } catch (e) {
    status.value = '失败：' + e
    step.value = 1
  }
}

async function readFileAsArray(path: string): Promise<ArrayBuffer> {
  const { readFile } = await import('@tauri-apps/plugin-fs')
  const bytes = await readFile(path)
  return bytes.buffer
}

function onUpdate(q: Question) {
  const i = reviewList.value.findIndex(x => x.id === q.id)
  if (i >= 0) reviewList.value[i] = q
}

async function confirmImport() {
  importedCount.value = reviewList.value.length
  step.value = 4
}
</script>

<style scoped>
.step { background: #fff; border: 1px solid #ddd; border-radius: 8px; padding: 24px; margin-top: 16px; }
.actions { margin-top: 16px; display: flex; gap: 8px; }
button { padding: 8px 16px; border: 1px solid #ddd; border-radius: 6px; cursor: pointer; }
</style>
```

注：校验页编辑后回写数据库的 command（update_question）在 Task 11 补充。MVP 阶段可先只读校验。

- [ ] **Step 2: 编写 ImportReviewTable.vue**

Create `src/components/ImportReviewTable.vue`:

```vue
<template>
  <div class="table-wrap">
    <table>
      <thead>
        <tr><th>题号</th><th>题型</th><th>题干</th><th>选项</th><th>答案</th><th>置信度</th></tr>
      </thead>
      <tbody>
        <tr v-for="(q, i) in list" :key="q.id" :class="{ low: q.confidence < 0.6 }">
          <td>{{ q.source_index ?? i + 1 }}</td>
          <td>{{ typeLabel(q.type) }}</td>
          <td><textarea v-model="q.stem" @change="$emit('update', q)"></textarea></td>
          <td>{{ q.options }}</td>
          <td><input v-model="q.answer" @change="$emit('update', q)" /></td>
          <td>{{ (q.confidence * 100).toFixed(0) }}%</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { Question } from '../utils/api'
defineProps<{ list: Question[] }>()
defineEmits<{ (e: 'update', q: Question): void }>()
function typeLabel(t: string) {
  return { single: '单选', multi: '多选', judge: '判断', blank: '填空', qa: '问答' }[t] || t
}
</script>

<style scoped>
.table-wrap { overflow: auto; max-height: 60vh; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ddd; padding: 6px; text-align: left; font-size: 13px; }
tr.low { background: #fff3f3; }
textarea, input { width: 100%; border: 1px solid #eee; padding: 4px; }
</style>
```

- [ ] **Step 3: 验证**

Run: `npm run tauri dev`
Expected: 能选 .docx 文件，识别后显示校验表格，确认后入库。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: 导入向导与校验页"
```

### Task 11: 题目与刷题 command + 刷题页

**Files:**
- Create: `src-tauri/src/commands/question.rs`, `practice.rs`
- Create: `src/views/PracticeView.vue`, `src/components/QuestionCard.vue`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 编写 question.rs command**

```rust
use tauri::State;
use crate::db::{DbState, models::Question, repo};

#[tauri::command]
pub fn list_questions(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<Vec<Question>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::list_questions(&conn, bank_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_question(db: State<'_, DbState>, q: Question) -> anyhow::Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE questions SET type=?1, stem=?2, options=?3, answer=?4, analysis=?5 WHERE id=?6",
        rusqlite::params![q.q_type, q.stem, q.options, q.answer, q.analysis, q.id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
```

在 `repo.rs` 加 `list_wrong` 已存在；`practice.rs`：

```rust
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
```

- [ ] **Step 2: 注册 command**

修改 `lib.rs` 的 `invoke_handler`，加入：
`commands::question::list_questions`, `commands::question::update_question`, `commands::practice::record_practice`, `commands::practice::list_wrong`, `commands::import::import_from_html`。

- [ ] **Step 3: 编写 QuestionCard.vue**

```vue
<template>
  <div class="qcard">
    <div class="stem">
      <span class="idx">{{ index + 1 }}.</span>
      <span class="type-tag">{{ typeLabel }}</span>
      {{ question.stem }}
    </div>

    <!-- 选择题 -->
    <div v-if="isChoice" class="options">
      <button
        v-for="(opt, i) in options"
        :key="i"
        class="option"
        :class="optionClass(i)"
        :disabled="submitted"
        @click="toggle(i)"
      >
        <span class="letter">{{ letter(i) }}</span> {{ opt }}
      </button>
    </div>

    <!-- 判断题 -->
    <div v-else-if="question.type === 'judge'" class="options">
      <button class="option" :class="judgeClass(true)" :disabled="submitted" @click="answerJudge(true)">√ 正确</button>
      <button class="option" :class="judgeClass(false)" :disabled="submitted" @click="answerJudge(false)">× 错误</button>
    </div>

    <!-- 填空/问答 -->
    <div v-else class="blank">
      <textarea v-model="blankAnswer" :disabled="submitted" placeholder="输入你的答案"></textarea>
    </div>

    <div class="actions">
      <button v-if="!submitted" @click="submit">确认</button>
      <button v-if="submitted" @click="$emit('next')">下一题</button>
      <button v-if="submitted && !isChoice && question.type !== 'judge'" @click="selfEval(true)">答对</button>
      <button v-if="submitted && !isChoice && question.type !== 'judge'" @click="selfEval(false)">答错</button>
    </div>

    <div v-if="submitted" class="feedback" :class="{ correct: isCorrect }">
      <p>{{ isCorrect ? '✓ 回答正确' : '✗ 回答错误' }}</p>
      <p>正确答案：{{ displayAnswer }}</p>
      <div v-if="question.analysis" class="analysis">
        <strong>解析：</strong>{{ question.analysis }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Question } from '../utils/api'

const props = defineProps<{ question: Question; index: number }>()
const emit = defineEmits<{
  (e: 'answered', payload: { correct: boolean; answer: string }): void
  (e: 'next'): void
}>()

const selected = ref<number[]>([])
const blankAnswer = ref('')
const submitted = ref(false)
const isCorrect = ref(false)
const selfCorrect = ref(false)

const options = computed<string[]>(() => props.question.options ? JSON.parse(props.question.options) : [])
const isChoice = computed(() => ['single', 'multi'].includes(props.question.type))
const typeLabel = computed(() => ({ single: '单选', multi: '多选', judge: '判断', blank: '填空', qa: '问答' }[props.question.type] || props.question.type))
const displayAnswer = computed(() => {
  if (!props.question.answer) return '（未识别到答案）'
  if (props.question.type === 'judge') return props.question.answer === 'true' ? '正确' : '错误'
  return props.question.answer
})

function letter(i: number) { return String.fromCharCode(65 + i) }
function toggle(i: number) {
  if (props.question.type === 'single') { selected.value = [i] }
  else {
    const idx = selected.value.indexOf(i)
    if (idx >= 0) selected.value.splice(idx, 1)
    else selected.value.push(i)
  }
}
function optionClass(i: number) {
  if (!submitted.value) return { selected: selected.value.includes(i) }
  const correctLetters = parseAnswerLetters()
  const isAns = correctLetters.includes(i)
  const isPicked = selected.value.includes(i)
  return { correct: isAns, wrong: isPicked && !isAns }
}
function judgeClass(val: boolean) {
  if (!submitted.value) return {}
  const ans = props.question.answer === 'true'
  return { correct: val === ans, wrong: val !== ans }
}
function parseAnswerLetters(): number[] {
  if (!props.question.answer) return []
  try {
    const arr = JSON.parse(props.question.answer) as string[]
    return arr.map(s => s.charCodeAt(0) - 65)
  } catch {
    return props.question.answer.split('').filter(c => /[A-D]/i.test(c)).map(c => c.toUpperCase().charCodeAt(0) - 65)
  }
}
function answerJudge(val: boolean) {
  submitted.value = true
  isCorrect.value = (val === (props.question.answer === 'true'))
  emit('answered', { correct: isCorrect.value, answer: String(val) })
}
function submit() {
  submitted.value = true
  if (isChoice.value) {
    const picked = [...selected.value].sort().map(i => String.fromCharCode(65 + i))
    const correct = parseAnswerLetters().sort().map(i => String.fromCharCode(65 + i))
    isCorrect.value = JSON.stringify(picked) === JSON.stringify(correct)
    emit('answered', { correct: isCorrect.value, answer: JSON.stringify(picked) })
  } else {
    // 填空/问答：展示参考答案，等自评
    isCorrect.value = false
  }
}
function selfEval(correct: boolean) {
  isCorrect.value = correct
  emit('answered', { correct, answer: blankAnswer.value })
}
</script>

<style scoped>
.qcard { background: #fff; border-radius: 8px; padding: 24px; border: 1px solid #eee; }
.stem { font-size: 16px; line-height: 1.6; margin-bottom: 16px; }
.idx { font-weight: bold; margin-right: 8px; }
.type-tag { background: #eee; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-right: 8px; }
.options { display: flex; flex-direction: column; gap: 8px; }
.option { text-align: left; padding: 12px; border: 1px solid #ddd; border-radius: 6px; background: #fff; cursor: pointer; }
.option.selected { border-color: #42b883; background: #f0fff5; }
.option.correct { border-color: #42b883; background: #d4edda; }
.option.wrong { border-color: #f56c6c; background: #fde2e2; }
.letter { font-weight: bold; margin-right: 8px; }
.actions { margin-top: 16px; display: flex; gap: 8px; }
.feedback { margin-top: 16px; padding: 12px; border-radius: 6px; background: #fde2e2; }
.feedback.correct { background: #d4edda; }
.analysis { margin-top: 8px; color: #555; }
textarea { width: 100%; min-height: 80px; padding: 8px; border: 1px solid #ddd; border-radius: 6px; }
</style>
```

- [ ] **Step 4: 编写 PracticeView.vue**

```vue
<template>
  <div class="practice">
    <div class="topbar">
      <span>{{ bankName }}</span>
      <span>{{ current + 1 }} / {{ questions.length }}</span>
      <select v-model="mode">
        <option value="order">顺序练习</option>
        <option value="random">随机练习</option>
      </select>
    </div>

    <div v-if="questions.length && current >= 0">
      <QuestionCard
        :key="currentQuestion.id"
        :question="currentQuestion"
        :index="current"
        @answered="onAnswered"
        @next="next"
      />
    </div>
    <div v-else>暂无题目，请先导入。</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api, Question } from '../utils/api'
import { useBankStore } from '../stores/bank'
import QuestionCard from '../components/QuestionCard.vue'

const route = useRoute()
const bankStore = useBankStore()
const bankId = Number(route.params.bankId)
const bankName = computed(() => bankStore.banks.find(b => b.id === bankId)?.name || '')
const questions = ref<Question[]>([])
const order = ref<number[]>([])
const current = ref(0)
const mode = ref('order')

const currentQuestion = computed(() => questions.value[order.value[current.value]] || questions.value[0])

onMounted(async () => {
  questions.value = await api.listQuestions(bankId)
  order.value = questions.value.map((_, i) => i)
})

function shuffle<T>(arr: T[]): T[] {
  const a = [...arr]
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]]
  }
  return a
}

async function onAnswered(payload: { correct: boolean; answer: string }) {
  const q = currentQuestion.value
  await api.recordPractice({ bank_id: bankId, question_id: q.id, user_answer: payload.answer, is_correct: payload.correct, duration_ms: null })
}

function next() {
  if (current.value < questions.value.length - 1) current.value++
}
</script>

<style scoped>
.topbar { display: flex; gap: 16px; align-items: center; margin-bottom: 16px; }
select { padding: 4px 8px; }
</style>
```

- [ ] **Step 5: 验证**

Run: `npm run tauri dev`
Expected: 选 .docx 导入题目后，进入刷题页，可点击选项、确认判分、看解析、下一题。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: 刷题页核心交互（单选/多选/判断/填空）"
```

---

## 阶段 4：增强功能

### Task 12: 错题本 WrongView

**Files:**
- Create: `src/views/WrongView.vue`

- [ ] **Step 1: 编写 WrongView.vue**

```vue
<template>
  <div class="wrong">
    <h2>错题本（{{ wrongIds.length }} 题）</h2>
    <button v-if="!practicing && wrongIds.length" @click="start">错题重练</button>
    <QuestionCard v-if="practicing && current" :question="current" :index="idx" @answered="onAnswered" @next="next" />
    <div v-else-if="!wrongIds.length">暂无错题，继续加油！</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api, Question } from '../utils/api'
import QuestionCard from '../components/QuestionCard.vue'

const route = useRoute()
const bankId = Number(route.params.bankId)
const allQuestions = ref<Question[]>([])
const wrongIds = ref<number[]>([])
const practicing = ref(false)
const queue = ref<number[]>([])
const idx = ref(0)
const current = ref<Question | null>(null)

onMounted(async () => {
  allQuestions.value = await api.listQuestions(bankId)
  const ids = await api.listWrong(bankId)
  wrongIds.value = ids
})

function start() {
  queue.value = [...wrongIds.value]
  idx.value = 0
  practicing.value = true
  loadCurrent()
}
function loadCurrent() {
  const id = queue.value[idx.value]
  current.value = allQuestions.value.find(q => q.id === id) || null
}
function next() {
  if (idx.value < queue.value.length - 1) { idx.value++; loadCurrent() }
  else { practicing.value = false }
}
async function onAnswered(payload: { correct: boolean; answer: string }) {
  if (current.value) {
    await api.recordPractice({ bank_id: bankId, question_id: current.value.id, user_answer: payload.answer, is_correct: payload.correct, duration_ms: null })
  }
}
</script>
```

- [ ] **Step 2: 验证并 Commit**

```bash
git add -A
git commit -m "feat: 错题本与错题重练"
```

### Task 13: 统计页 StatsView（基础）

**Files:**
- Create: `src/views/StatsView.vue`
- Create: `src-tauri/src/commands/practice.rs`（补充 stats command）

- [ ] **Step 1: 后端 stats command**

在 `repo.rs` 加：

```rust
pub struct BankStats {
    pub total: i64,
    pub practiced: i64,
    pub correct: i64,
}

pub fn bank_stats(conn: &Connection, bank_id: i64) -> Result<BankStats> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM questions WHERE bank_id=?1", params![bank_id], |r| r.get(0))?;
    let practiced: i64 = conn.query_row("SELECT COUNT(*) FROM practice_records WHERE bank_id=?1", params![bank_id], |r| r.get(0))?;
    let correct: i64 = conn.query_row("SELECT COUNT(*) FROM practice_records WHERE bank_id=?1 AND is_correct=1", params![bank_id], |r| r.get(0))?;
    Ok(BankStats { total, practiced, correct })
}
```

在 `commands/practice.rs` 加：

```rust
#[tauri::command]
pub fn bank_stats(db: State<'_, DbState>, bank_id: i64) -> anyhow::Result<serde_json::Value, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let s = repo::bank_stats(&conn, bank_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "total": s.total, "practiced": s.practiced, "correct": s.correct }))
}
```

注册到 `lib.rs`。

- [ ] **Step 2: 前端 StatsView.vue**

```vue
<template>
  <div class="stats">
    <h2>学习统计</h2>
    <div class="cards">
      <div class="stat-card"><span>{{ stats.total }}</span><label>总题数</label></div>
      <div class="stat-card"><span>{{ stats.practiced }}</span><label>已练习</label></div>
      <div class="stat-card"><span>{{ accuracy }}%</span><label>正确率</label></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()
const bankId = Number(route.params.bankId)
const stats = ref({ total: 0, practiced: 0, correct: 0 })
const accuracy = computed(() => stats.value.practiced ? Math.round(stats.value.correct / stats.value.practiced * 100) : 0)

onMounted(async () => {
  stats.value = await invoke('bank_stats', { bankId })
})
</script>

<style scoped>
.cards { display: flex; gap: 16px; }
.stat-card { background: #fff; border: 1px solid #eee; border-radius: 8px; padding: 24px; text-align: center; }
.stat-card span { display: block; font-size: 32px; font-weight: bold; color: #42b883; }
</style>
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: 基础统计页"
```

### Task 14: OCR 引擎集成（Tesseract）

**Files:**
- Create: `src-tauri/src/import/ocr.rs`
- Modify: `src-tauri/src/import/pipeline.rs`

- [ ] **Step 1: 添加 tesseract 依赖**

`Cargo.toml` 加：
```toml
tesseract-rs = "0.3"
```
（或改用调用系统 `tesseract.exe`，通过 `std::process::Command`，避免编译依赖问题。推荐后者更稳妥。）

- [ ] **Step 2: 编写 ocr.rs（调用系统 tesseract）**

```rust
use anyhow::Result;
use std::process::Command;
use std::fs;
use std::path::Path;

/// 对 base64 图片做 OCR，返回识别文本。需系统已安装 tesseract 并在 PATH。
pub fn ocr_base64(b64: &str, lang: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let tmp = std::env::temp_dir().join(format!("ocr_{}.png", uuid::Uuid::new_v4()));
    fs::write(&tmp, &bytes)?;
    let output = Command::new("tesseract")
        .arg(&tmp)
        .arg("stdout")
        .args(["-l", lang])
        .output()?;
    let _ = fs::remove_file(&tmp);
    if !output.status.success() {
        anyhow::bail!("tesseract failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 检测 tesseract 是否可用
pub fn tesseract_available() -> bool {
    Command::new("tesseract").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}
```

需 `use base64::Engine;`。

- [ ] **Step 3: 在 pipeline 中对图片做 OCR 回填**

修改 `pipeline.rs`，在 `parse_html` 后，若 `parsed.images` 非空且 tesseract 可用，对每个 `[IMG:n]` 占位替换为 OCR 文本：

```rust
pub fn html_to_questions(html: &str, bank_id: i64) -> Result<Vec<Question>> {
    let mut parsed = parse_html(html)?;
    if crate::import::ocr::tesseract_available() {
        for (i, img) in parsed.images.iter().enumerate() {
            if let Ok(text) = crate::import::ocr::ocr_base64(img, "chi_sim+eng") {
                let placeholder = format!("[IMG:{}]", i);
                for p in parsed.paragraphs.iter_mut() {
                    *p = p.replace(&placeholder, &text);
                }
            }
        }
    }
    let parsed_qs = parse_questions(&parsed.paragraphs);
    Ok(parsed_qs.into_iter().map(|pq| to_question(pq, bank_id)).collect())
}
```

- [ ] **Step 4: 验证并 Commit**

```bash
git add -A
git commit -m "feat: Tesseract OCR 集成（图片识别）"
```

### Task 15: AI 增强引擎

**Files:**
- Create: `src-tauri/src/import/ai.rs`
- Modify: `src-tauri/src/commands/import.rs`（加 AI 导入 command）
- Modify: `src-tauri/src/db/repo.rs`（get_setting 读 Key）

- [ ] **Step 1: 编写 ai.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Serialize)]
struct AiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AiRequest<'a> {
    model: &'a str,
    messages: Vec<AiMessage<'a>>,
    temperature: f32,
}

#[derive(Deserialize)]
struct AiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: RespMessage,
}

#[derive(Deserialize)]
struct RespMessage {
    content: String,
}

const PROMPT: &str = r#"你是题库结构化助手。下面是 Word 文档提取的内容（可能含图片OCR文本）。
请识别所有题目，输出 JSON 数组，每个元素：
{"type":"single|multi|judge|blank","stem":"题干","options":["A选项",...],"answer":"答案","analysis":"解析"}
- 单选 answer 为单个字母如 "A"；多选为字母串如 "AC"；判断为 "true"/"false"；填空为答案文本
- 只输出 JSON，不要解释
"#;

pub async fn ai_structurize(text: &str, api_key: &str, base_url: &str, model: &str) -> Result<Vec<crate::import::structure::ParsedQuestion>> {
    let client = Client::new();
    let req = AiRequest {
        model,
        messages: vec![
            AiMessage { role: "system", content: PROMPT },
            AiMessage { role: "user", content: text },
        ],
        temperature: 0.1,
    };
    let resp = client.post(format!("{}/chat/completions", base_url))
        .bearer_auth(api_key)
        .json(&req)
        .send().await?
        .json::<AiResponse>().await?;
    let content = resp.choices.into_iter().next().ok_or_else(|| anyhow::anyhow!("无 AI 响应"))?.message.content;
    // 提取 JSON 数组（容错：去掉 markdown 代码块）
    let json = extract_json(&content);
    let parsed: Vec<crate::import::structure::ParsedQuestion> = serde_json::from_str(&json)?;
    Ok(parsed)
}

fn extract_json(s: &str) -> String {
    let s = s.trim();
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            return s[start..=end].to_string();
        }
    }
    s.to_string()
}
```

注：`ParsedQuestion` 需 derive `Deserialize`（已在 Task 5 加）。

- [ ] **Step 2: 加 AI 导入 command**

在 `commands/import.rs` 加：

```rust
use crate::db::repo;

#[tauri::command]
pub async fn import_with_ai(db: tauri::State<'_, DbState>, bank_id: i64, text: String) -> anyhow::Result<i64, String> {
    let (api_key, base_url, model) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let key = repo::get_setting(&conn, "ai_api_key").map_err(|e| e.to_string())?.ok_or("未配置 AI Key")?;
        let base = repo::get_setting(&conn, "ai_base_url").map_err(|e| e.to_string())?.unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4".to_string());
        let model = repo::get_setting(&conn, "ai_model").map_err(|e| e.to_string())?.unwrap_or_else(|| "glm-4v".to_string());
        (key, base, model)
    };
    let qs = crate::import::ai::ai_structurize(&text, &api_key, &base_url, &model).await.map_err(|e| e.to_string())?;
    let questions: Vec<crate::db::models::Question> = qs.into_iter().map(|pq| {
        crate::import::pipeline::to_question_public(pq, bank_id)
    }).collect();
    let count = questions.len() as i64;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    repo::insert_questions(&conn, bank_id, &questions).map_err(|e| e.to_string())?;
    Ok(count)
}
```

（需把 `to_question` 改为 `pub fn to_question_public`。）

注册到 `lib.rs`。

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: AI 大模型识别引擎"
```

### Task 16: 设置页与错误处理完善

**Files:**
- Create: `src/views/SettingsView.vue`
- Create: `src-tauri/src/commands/settings.rs`（或并入 bank）
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 后端 settings command**

Create `src-tauri/src/commands/settings.rs`:

```rust
use tauri::State;
use crate::db::{DbState, repo};

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
```

在 `commands/mod.rs` 加 `pub mod settings;`，注册到 `lib.rs`。

- [ ] **Step 2: 前端 SettingsView.vue**

```vue
<template>
  <div class="settings">
    <h2>设置</h2>
    <section>
      <h3>AI 识别引擎</h3>
      <label>API Key <input v-model="apiKey" @blur="save('ai_api_key', apiKey)" type="password" /></label>
      <label>Base URL <input v-model="baseUrl" @blur="save('ai_base_url', baseUrl)" /></label>
      <label>模型 <input v-model="model" @blur="save('ai_model', model)" /></label>
      <p class="hint">默认 GLM：Base URL https://open.bigmodel.cn/api/paas/v4，模型 glm-4v</p>
    </section>
    <section>
      <h3>OCR 引擎</h3>
      <p>{{ ocrAvailable ? '✓ Tesseract 可用' : '✗ 未检测到 Tesseract，图片识别将不可用' }}</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const apiKey = ref('')
const baseUrl = ref('https://open.bigmodel.cn/api/paas/v4')
const model = ref('glm-4v')
const ocrAvailable = ref(false)

onMounted(async () => {
  apiKey.value = (await invoke<string | null>('get_setting', { key: 'ai_api_key' })) || ''
  baseUrl.value = (await invoke<string | null>('get_setting', { key: 'ai_base_url' })) || baseUrl.value
  model.value = (await invoke<string | null>('get_setting', { key: 'ai_model' })) || model.value
})

async function save(key: string, value: string) {
  await invoke('set_setting', { key, value })
}
</script>

<style scoped>
section { background: #fff; border: 1px solid #eee; border-radius: 8px; padding: 16px; margin-bottom: 16px; }
label { display: block; margin: 8px 0; }
input { padding: 6px; border: 1px solid #ddd; border-radius: 4px; width: 320px; }
.hint { color: #888; font-size: 13px; }
</style>
```

- [ ] **Step 3: 在 ImportView 增加引擎选择**

在 ImportView 步骤1加单选：本地引擎 / AI 引擎。AI 引擎时调用 `import_with_ai` command（传 mammoth 提取的纯文本）。

- [ ] **Step 4: 验证全流程**

Run: `npm run tauri dev`
- 设置页配置 AI Key
- 导入时选 AI 引擎，验证 AI 识别
- 选本地引擎，验证 OCR + 规则解析

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: 设置页与 AI 引擎选择"
```

---

## Self-Review 检查

**Spec 覆盖**：
- 架构/技术栈 → Task 2 ✓
- 数据模型 → Task 3 ✓
- .docx 解析 → Task 4 ✓
- 结构化（题型识别+答案关联）→ Task 5, 6 ✓
- 导入流水线+校验页 → Task 7, 10 ✓
- 刷题全题型 → Task 11 ✓
- 多题库管理 → Task 8, 9 ✓
- 答题记录 → Task 11 ✓
- 错题本 → Task 12 ✓
- 统计 → Task 13 ✓
- OCR → Task 14 ✓
- AI 增强 → Task 15 ✓
- 设置 → Task 16 ✓
- 错误处理（.doc/置信度/答案缺失/AI失败）→ 散布在各 Task，Task 16 收口 ✓

**类型一致性**：`Question.q_type`（Rust）/ `question.type`（前端，serde rename）一致；`ParsedQuestion` 字段在 Task 5/15 一致；command 名称与 `api.ts` 调用一致。

**遗留**：`.doc` 转 `.docx`（LibreOffice）未单独成 Task，可在 Task 14 后补；模拟考试模式未实现（非 MVP 核心，后期优化）。
