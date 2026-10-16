use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 系统 API 调用日志表（`system_api_log`）。
///
/// 对应《系统功能设计与数据库设计文档》四.14：记录每次外部 API（大模型/工具）调用，
/// 通过 `rel_uuid` 关联业务实体（如 `chat_message.round_uuid`），并统计 Token 消耗与耗时。
#[derive(Clone, Debug)]
pub struct SystemApiLog {
    pub id: i32,
    /// 关联业务 UUID（可空）。
    pub rel_uuid: Option<String>,
    /// 调用状态：1 成功 / 0 失败。
    pub status: i8,
    /// 本次调用 Token 消耗。
    pub token_cost: i32,
    /// 调用耗时，单位：毫秒。
    pub duration: i32,
    /// 调用内容/详情（可空，JSON 或文本）。
    pub content: Option<String>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for SystemApiLog {
    fn table_name() -> &'static str {
        "system_api_log"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS system_api_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rel_uuid TEXT,
            status INTEGER NOT NULL DEFAULT 1,
            token_cost INTEGER NOT NULL DEFAULT 0,
            duration INTEGER NOT NULL DEFAULT 0,
            content TEXT,
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
            rel_uuid: row.get("rel_uuid")?,
            status: row.get("status")?,
            token_cost: row.get("token_cost")?,
            duration: row.get("duration")?,
            content: row.get("content")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "rel_uuid, status, token_cost, duration, content, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.rel_uuid.clone()),
            Box::new(self.status),
            Box::new(self.token_cost),
            Box::new(self.duration),
            Box::new(self.content.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        rel_uuid = ?,
        status = ?,
        token_cost = ?,
        duration = ?,
        content = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.rel_uuid.clone()),
            Box::new(self.status),
            Box::new(self.token_cost),
            Box::new(self.duration),
            Box::new(self.content.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
