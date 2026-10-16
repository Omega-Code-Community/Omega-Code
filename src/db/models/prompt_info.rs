use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 提示词表（`prompt_info`）。
///
/// 对应《系统功能设计与数据库设计文档》四.4：支持模板化配置、占位符动态渲染、
/// 版本迭代（同名称下 `version` 唯一）与质量校验。
#[derive(Clone, Debug)]
pub struct PromptInfo {
    pub id: i32,
    /// 提示词名称，非空。
    pub name: String,
    /// 提示词内容，支持占位符模板。
    pub content: String,
    /// 提示词简介。
    pub description: Option<String>,
    /// 业务分类标签。
    pub category: Option<String>,
    /// 角色类型，枚举约束：System/User。
    pub role: Option<String>,
    /// 版本号，如 v1.0.0，同名称下版本唯一。
    pub version: Option<String>,
    /// 版本备注（迭代说明）。
    pub version_desc: Option<String>,
    /// 标签信息。
    pub tags: Option<String>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for PromptInfo {
    fn table_name() -> &'static str {
        "prompt_info"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS prompt_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            description TEXT,
            category TEXT,
            role TEXT,
            version TEXT,
            version_desc TEXT,
            tags TEXT,
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
            content: row.get("content")?,
            description: row.get("description")?,
            category: row.get("category")?,
            role: row.get("role")?,
            version: row.get("version")?,
            version_desc: row.get("version_desc")?,
            tags: row.get("tags")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "name, content, description, category, role, version, version_desc, tags, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.content.clone()),
            Box::new(self.description.clone()),
            Box::new(self.category.clone()),
            Box::new(self.role.clone()),
            Box::new(self.version.clone()),
            Box::new(self.version_desc.clone()),
            Box::new(self.tags.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        content = ?,
        description = ?,
        category = ?,
        role = ?,
        version = ?,
        version_desc = ?,
        tags = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.content.clone()),
            Box::new(self.description.clone()),
            Box::new(self.category.clone()),
            Box::new(self.role.clone()),
            Box::new(self.version.clone()),
            Box::new(self.version_desc.clone()),
            Box::new(self.tags.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
