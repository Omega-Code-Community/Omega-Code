use chrono::{DateTime, Utc};
pub struct MemoryInfo {
    pub id: i32,
    pub memory_uuid: String,
    pub name: String,
    pub memory_type: i8,
    pub is_vector: i8,
    pub vector_id: String,
    pub keywords: String,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}