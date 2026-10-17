use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 会话表（`chat_session`）。
///
/// 对应《系统功能设计与数据库设计文档》四.7：一个项目可包含多个会话，
/// `chat_uuid` 全局唯一并对外暴露。按《后端开发手册》3.2，本表保留 `is_deleted`
/// 逻辑删除字段（仅业务层过滤，无数据恢复）。
#[derive(Clone, Debug)]
pub struct ChatSession {
    pub id: i32,
    /// 会话唯一标识，全局唯一、对外暴露。
    pub chat_uuid: String,
    /// 会话名称。
    pub title: Option<String>,
    /// 绑定 LLM ID，关联 `llm_model.id` 逻辑外键。
    pub llm_id: i32,
    /// 逻辑删除：0-否 1-是。
    pub is_deleted: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for ChatSession {
    fn table_name() -> &'static str {
        "chat_session"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS chat_session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_uuid TEXT NOT NULL,
            title TEXT,
            llm_id INTEGER NOT NULL,
            is_deleted INTEGER NOT NULL DEFAULT 0,
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
            chat_uuid: row.get("chat_uuid")?,
            title: row.get("title")?,
            llm_id: row.get("llm_id")?,
            is_deleted: row.get("is_deleted")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "chat_uuid, title, llm_id, is_deleted, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.chat_uuid.clone()),
            Box::new(self.title.clone()),
            Box::new(self.llm_id),
            Box::new(self.is_deleted),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        chat_uuid = ?,
        title = ?,
        llm_id = ?,
        is_deleted = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.chat_uuid.clone()),
            Box::new(self.title.clone()),
            Box::new(self.llm_id),
            Box::new(self.is_deleted),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
