use chrono::{DateTime, Utc};
pub struct AgentArena {
    pub id: i32,
    pub chat_uuid: String,
    pub round_uuid: String,
    pub agent_id: i32,
    pub vote_type: i8,
    pub comment: String,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}