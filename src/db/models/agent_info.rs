use chrono::{DateTime, Utc};
pub struct AgentInfo {
    pub id: i32,
    pub llm_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub prompt_id: i32,
    pub avatar: String,
    pub call_count: i64,// default 0
    pub avg_score: f32,// default 0.0
    pub temperature: f32, // default 0.7
    pub top_p: f32, // default 0.95
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}