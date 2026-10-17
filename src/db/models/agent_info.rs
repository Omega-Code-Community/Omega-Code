use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 智能体表（`agent_info`）。
///
/// 对应《系统功能设计与数据库设计文档》四.3：Agent 以 LLM + 专属 Prompt 为核心，
/// 绑定 MCP 工具（经 `agent_mcp_rel` 中间表）、支持跨 Agent 能力复用。
/// 提示词 1 对 1 绑定，模型个性化参数（温度、top_p）在此配置生效。
#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub id: i32,
    /// LLM 模型 ID，关联 `llm_model.id` 逻辑外键。
    pub llm_id: i32,
    /// 智能体名称，非空。
    pub name: String,
    /// 智能体描述（简介说明）。
    pub description: Option<String>,
    /// 提示词 ID，关联 `prompt_info.id`，一对一逻辑唯一绑定。
    pub prompt_id: Option<i32>,
    /// 头像地址（图片 URL / 本地路径）。
    pub avatar: Option<String>,
    /// 温度，decimal(2,1)。
    pub temperature: Option<f32>,
    /// 核心采样 top_p，decimal(2,1)。
    pub top_p: Option<f32>,
    /// 累计调用次数。
    pub call_count: i64,
    /// 平均评分，0-5 分，竞技场评价聚合均值。
    pub avg_score: Option<f32>,
    /// 启用状态：0-禁用 1-启用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for AgentInfo {
    fn table_name() -> &'static str {
        "agent_info"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS agent_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            llm_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            prompt_id INTEGER,
            avatar TEXT,
            temperature REAL,
            top_p REAL,
            call_count INTEGER NOT NULL DEFAULT 0,
            avg_score REAL,
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
            llm_id: row.get("llm_id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            prompt_id: row.get("prompt_id")?,
            avatar: row.get("avatar")?,
            temperature: row.get("temperature")?,
            top_p: row.get("top_p")?,
            call_count: row.get("call_count")?,
            avg_score: row.get("avg_score")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "llm_id, name, description, prompt_id, avatar, temperature, top_p, call_count, avg_score, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.llm_id),
            Box::new(self.name.clone()),
            Box::new(self.description.clone()),
            Box::new(self.prompt_id),
            Box::new(self.avatar.clone()),
            Box::new(self.temperature),
            Box::new(self.top_p),
            Box::new(self.call_count),
            Box::new(self.avg_score),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        llm_id = ?,
        name = ?,
        description = ?,
        prompt_id = ?,
        avatar = ?,
        temperature = ?,
        top_p = ?,
        call_count = ?,
        avg_score = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.llm_id),
            Box::new(self.name.clone()),
            Box::new(self.description.clone()),
            Box::new(self.prompt_id),
            Box::new(self.avatar.clone()),
            Box::new(self.temperature),
            Box::new(self.top_p),
            Box::new(self.call_count),
            Box::new(self.avg_score),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
