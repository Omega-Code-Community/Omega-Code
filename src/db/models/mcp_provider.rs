use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// MCP 服务提供方表（`mcp_provider`）。
///
/// 对应《系统功能设计与数据库设计文档》四.10：登记一个 MCP Server（如 Python 工具端），
/// 记录其连接地址、鉴权、超时与健康检查接口。`timeout` 单位为秒。
#[derive(Clone, Debug)]
pub struct McpProvider {
    pub id: i32,
    /// 提供方名称。
    pub name: String,
    /// MCP Server 连接地址。
    pub url: String,
    /// 鉴权密钥（可空）。
    pub auth_key: Option<String>,
    /// 请求超时，单位：秒。
    pub timeout: i32,
    /// 健康检查接口地址（可空）。
    pub health_api: Option<String>,
    /// 状态：1 启用 / 0 停用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for McpProvider {
    fn table_name() -> &'static str {
        "mcp_provider"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS mcp_provider (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            auth_key TEXT,
            timeout INTEGER NOT NULL DEFAULT 30,
            health_api TEXT,
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
            url: row.get("url")?,
            auth_key: row.get("auth_key")?,
            timeout: row.get("timeout")?,
            health_api: row.get("health_api")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "name, url, auth_key, timeout, health_api, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.url.clone()),
            Box::new(self.auth_key.clone()),
            Box::new(self.timeout),
            Box::new(self.health_api.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        url = ?,
        auth_key = ?,
        timeout = ?,
        health_api = ?,
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
            Box::new(self.url.clone()),
            Box::new(self.auth_key.clone()),
            Box::new(self.timeout),
            Box::new(self.health_api.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
