use chrono::{DateTime, Utc};
pub struct ProjectInfo {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub is_deleted: i8, // default 0
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}