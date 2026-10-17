use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// LLM 模型表（`llm_model`）。
///
/// 对应《系统功能设计与数据库设计文档》四.2：模型为公共基座能力，无独立运行参数，
/// 个性化配置（温度、上下文限制等）由 Agent 独立配置。`model_code` 逻辑唯一。
#[derive(Clone, Debug)]
pub struct LLMModel {
    pub id: i32,
    /// 供应商 ID，关联 `ai_provider.id` 逻辑外键。
    pub provider_id: i32,
    /// 头像地址（图片 URL / 本地路径）。
    pub avatar: Option<String>,
    /// 模型名称，非空。
    pub name: String,
    /// 模型标识，接口调用唯一标识，非空、逻辑唯一。
    pub model_code: String,
    /// 最大上下文（最大输入 token）。
    pub max_tokens: i32,
    /// 单次输出上限（单次生成最大 token）。
    pub max_response_tokens: i32,
    /// 负载分发权重。
    pub weight: i32,
    /// 累计调用次数。
    pub call_count: i64,
    /// 启用状态：0-禁用 1-启用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for LLMModel {
    fn table_name() -> &'static str {
        "llm_model"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS llm_model (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id INTEGER NOT NULL,
            avatar TEXT,
            name TEXT NOT NULL,
            model_code TEXT NOT NULL,
            max_tokens INTEGER NOT NULL DEFAULT 0,
            max_response_tokens INTEGER NOT NULL DEFAULT 0,
            weight INTEGER NOT NULL DEFAULT 0,
            call_count INTEGER NOT NULL DEFAULT 0,
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
            provider_id: row.get("provider_id")?,
            avatar: row.get("avatar")?,
            name: row.get("name")?,
            model_code: row.get("model_code")?,
            max_tokens: row.get("max_tokens")?,
            max_response_tokens: row.get("max_response_tokens")?,
            weight: row.get("weight")?,
            call_count: row.get("call_count")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "provider_id, avatar, name, model_code, max_tokens, max_response_tokens, weight, call_count, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.avatar.clone()),
            Box::new(self.name.clone()),
            Box::new(self.model_code.clone()),
            Box::new(self.max_tokens),
            Box::new(self.max_response_tokens),
            Box::new(self.weight),
            Box::new(self.call_count),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        provider_id = ?,
        avatar = ?,
        name = ?,
        model_code = ?,
        max_tokens = ?,
        max_response_tokens = ?,
        weight = ?,
        call_count = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.avatar.clone()),
            Box::new(self.name.clone()),
            Box::new(self.model_code.clone()),
            Box::new(self.max_tokens),
            Box::new(self.max_response_tokens),
            Box::new(self.weight),
            Box::new(self.call_count),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
