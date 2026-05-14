use chrono::{DateTime, Utc};
pub struct LLMModel {
    pub id: i32,
    pub provider_id: i32,
    pub name: String,
    pub avatar: String,
    pub model_code: String,
    pub max_tokens: i32,
    pub max_response_tokens: i32,
    pub temperature: f32,
    pub weight: i32,
    pub call_count: i64,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}