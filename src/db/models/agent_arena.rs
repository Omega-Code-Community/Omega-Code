use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 竞技场投票表（`agent_arena`）。
///
/// 对应《系统功能设计与数据库设计文档》四.6 / 3.8：记录用户对单轮回答的
/// **星级评分（1-5）与文字评价**，通过 `chat_uuid`/`round_uuid` 关联具体消息；
/// 评分聚合为 `agent_info.avg_score`（0-5 均值）。
#[derive(Clone, Debug)]
pub struct AgentArena {
    pub id: i32,
    /// 会话 UUID，关联 `chat_session.chat_uuid`。
    pub chat_uuid: String,
    /// 单轮消息 UUID，关联 `chat_message.round_uuid`。
    pub round_uuid: Option<String>,
    /// 被投票的 Agent ID，关联 `agent_info.id`。
    pub agent_id: i32,
    /// 投票类型：星级评分 1-5（竞技场打分，聚合为 `agent_info.avg_score`）。
    pub vote_type: i8,
    /// 用户评价文本。
    pub comment: Option<String>,
    /// 状态：1 有效 / 0 无效。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for AgentArena {
    fn table_name() -> &'static str {
        "agent_arena"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS agent_arena (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_uuid TEXT NOT NULL,
            round_uuid TEXT,
            agent_id INTEGER NOT NULL,
            vote_type INTEGER NOT NULL DEFAULT 0,
            comment TEXT,
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
            chat_uuid: row.get("chat_uuid")?,
            round_uuid: row.get("round_uuid")?,
            agent_id: row.get("agent_id")?,
            vote_type: row.get("vote_type")?,
            comment: row.get("comment")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "chat_uuid, round_uuid, agent_id, vote_type, comment, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.chat_uuid.clone()),
            Box::new(self.round_uuid.clone()),
            Box::new(self.agent_id),
            Box::new(self.vote_type),
            Box::new(self.comment.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        chat_uuid = ?,
        round_uuid = ?,
        agent_id = ?,
        vote_type = ?,
        comment = ?,
        status = ?,
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
            Box::new(self.agent_id),
            Box::new(self.vote_type),
            Box::new(self.comment.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
