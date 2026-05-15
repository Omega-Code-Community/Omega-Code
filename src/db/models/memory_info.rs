use chrono::{DateTime, Utc};
pub struct MemoryInfo {
    pub id: i32,
    pub memory_uuid: String,
    pub name: String,
    pub memory_type: i8, // default 0
    pub is_vector: i8, // default 0
    pub vector_id: Option<String>,
    pub keywords: Option<String>,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}