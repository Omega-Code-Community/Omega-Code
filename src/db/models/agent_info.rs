use chrono::{DateTime, Utc};
pub struct AgentInfo {
    pub id: i32,
    pub llm_id: i32,
    pub name: String,
    pub description: String,
    pub prompt_id: i32,
    pub avatar: String,
    pub params: String,
    pub call_count: i64,
    pub avg_score: f32,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}