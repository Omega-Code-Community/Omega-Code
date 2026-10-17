use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 消息表（`chat_message`）。
///
/// 对应《系统功能设计与数据库设计文档》四.8：拆分每轮对话独立存储，
/// 通过 `chat_uuid` 关联会话、`round_uuid` 标识单轮消息，并记录 Token 消耗、
/// 调用耗时与模型参数快照。
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: i32,
    /// 会话 UUID，关联 `chat_session.chat_uuid`。
    pub chat_uuid: String,
    /// 单轮消息 UUID，单轮唯一标识。
    pub round_uuid: Option<String>,
    /// 消息角色：user/assistant/system。
    pub role: String,
    /// 消息内容（对话正文）。
    pub content: String,
    /// 单轮输入消耗 Token 数。
    pub input_tokens: i32,
    /// 单轮输出消耗 Token 数。
    pub output_tokens: i32,
    /// 调用耗时，单位：毫秒。
    pub duration: i32,
    /// 模型参数快照（JSON 存储当时模型配置）。
    pub model_config: Option<String>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for ChatMessage {
    fn table_name() -> &'static str {
        "chat_message"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS chat_message (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_uuid TEXT NOT NULL,
            round_uuid TEXT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            duration INTEGER NOT NULL DEFAULT 0,
            model_config TEXT,
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
            round_uuid: row.get("round_uuid")?,
            role: row.get("role")?,
            content: row.get("content")?,
            input_tokens: row.get("input_tokens")?,
            output_tokens: row.get("output_tokens")?,
            duration: row.get("duration")?,
            model_config: row.get("model_config")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "chat_uuid, round_uuid, role, content, input_tokens, output_tokens, duration, model_config, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.chat_uuid.clone()),
            Box::new(self.round_uuid.clone()),
            Box::new(self.role.clone()),
            Box::new(self.content.clone()),
            Box::new(self.input_tokens),
            Box::new(self.output_tokens),
            Box::new(self.duration),
            Box::new(self.model_config.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        chat_uuid = ?,
        round_uuid = ?,
        role = ?,
        content = ?,
        input_tokens = ?,
        output_tokens = ?,
        duration = ?,
        model_config = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.chat_uuid.clone()),
            Box::new(self.round_uuid.clone()),
            Box::new(self.role.clone()),
            Box::new(self.content.clone()),
            Box::new(self.input_tokens),
            Box::new(self.output_tokens),
            Box::new(self.duration),
            Box::new(self.model_config.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
