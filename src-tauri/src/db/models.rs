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
    #[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankStats {
    pub total: i64,
    pub practiced: i64,
    pub correct: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub id: i64,
    pub bank_id: i64,
    pub question_id: i64,
    pub created_at: String,
}
