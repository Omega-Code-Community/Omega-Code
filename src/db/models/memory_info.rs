use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 记忆信息表（`memory_info`）。
///
/// 对应《系统功能设计与数据库设计文档》四.9：以 `memory_uuid` 唯一标识一条记忆，
/// 区分用户级/项目级记忆，并支持向量化索引（`is_vector`/`vector_id`）与关键词检索。
#[derive(Clone, Debug)]
pub struct MemoryInfo {
    pub id: i32,
    /// 记忆唯一标识。
    pub memory_uuid: String,
    /// 记忆类型：user（用户级）/ project（项目级）。
    pub memory_type: String,
    /// 是否已向量化：1 是 / 0 否。
    pub is_vector: i8,
    /// 向量索引 ID（外部向量库引用）。
    pub vector_id: Option<String>,
    /// 检索关键词（逗号分隔或 JSON）。
    pub keywords: Option<String>,
    /// 状态：1 有效 / 0 失效。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for MemoryInfo {
    fn table_name() -> &'static str {
        "memory_info"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS memory_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            memory_uuid TEXT NOT NULL,
            memory_type TEXT NOT NULL,
            is_vector INTEGER NOT NULL DEFAULT 0,
            vector_id TEXT,
            keywords TEXT,
            status INTEGER NOT NULL DEFAULT 1,
            create_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            remark TEXT
        )
        "#
    }

    fn from_row(row: &Row) -> RusqliteResult<Self> {
        let create_time = parse_db_time(&row.get::<_, String>("create_time")?)?;
        let update_time = parse_db_time(&row.get::<_, String>("update_time")?)?;

        Ok(Self {
            id: row.get("id")?,
            memory_uuid: row.get("memory_uuid")?,
            memory_type: row.get("memory_type")?,
            is_vector: row.get("is_vector")?,
            vector_id: row.get("vector_id")?,
            keywords: row.get("keywords")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "memory_uuid, memory_type, is_vector, vector_id, keywords, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.memory_uuid.clone()),
            Box::new(self.memory_type.clone()),
            Box::new(self.is_vector),
            Box::new(self.vector_id.clone()),
            Box::new(self.keywords.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        memory_uuid = ?,
        memory_type = ?,
        is_vector = ?,
        vector_id = ?,
        keywords = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.memory_uuid.clone()),
            Box::new(self.memory_type.clone()),
            Box::new(self.is_vector),
            Box::new(self.vector_id.clone()),
            Box::new(self.keywords.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
