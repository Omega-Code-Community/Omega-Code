use chrono::{DateTime, Utc};
pub struct SystemAPILog {
    pub id: i32,
    pub rel_uuid: String,
    pub token_cost: i32,
    pub duration: i32,
    pub content: String,
    pub status: i8, // 0: success, 1: failed
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}