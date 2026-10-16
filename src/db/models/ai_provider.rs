use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// AI 供应商表（`ai_provider`）。
///
/// 对应《系统功能设计与数据库设计文档》四.1：统一管理接入的 AI 厂商服务，
/// 除 Ollama 外 API 地址硬编码预置，密钥隔离存储。
#[derive(Clone, Debug)]
pub struct AiProvider {
    pub id: i32,
    /// 供应商名称，非空。
    pub name: String,
    /// 头像地址（图片 URL / 本地路径）。
    pub avatar: Option<String>,
    /// API 地址，非空。
    pub api: String,
    /// API 密钥，可为空（Ollama 非强制）。
    pub key: Option<String>,
    /// 请求超时时间，单位：秒，默认 30。
    pub timeout: i32,
    /// 启用状态：0-禁用 1-启用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for AiProvider {
    fn table_name() -> &'static str {
        "ai_provider"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS ai_provider (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            avatar TEXT,
            api TEXT NOT NULL,
            key TEXT,
            timeout INTEGER NOT NULL DEFAULT 30,
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
            name: row.get("name")?,
            avatar: row.get("avatar")?,
            api: row.get("api")?,
            key: row.get("key")?,
            timeout: row.get("timeout")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "name, avatar, api, key, timeout, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.avatar.clone()),
            Box::new(self.api.clone()),
            Box::new(self.key.clone()),
            Box::new(self.timeout),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        avatar = ?,
        api = ?,
        key = ?,
        timeout = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.avatar.clone()),
            Box::new(self.api.clone()),
            Box::new(self.key.clone()),
            Box::new(self.timeout),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
