use chrono::{DateTime, Utc};
pub struct LLMModel {
    pub id: i32,
    pub provider_id: i32,
    pub name: Option<String>,
    pub avatar: String,
    pub model_code: String,
    pub max_tokens: i32, // default 0
    pub max_response_tokens: i32, // default 0
    pub weight: i32, // default 0
    pub call_count: i64, // default 0
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}