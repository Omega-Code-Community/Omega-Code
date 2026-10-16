use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 占位符表（`prompt_placeholder`）。
///
/// 对应《系统功能设计与数据库设计文档》四.5：Prompt 占位符定义，
/// `(prompt_id, key)` 联合唯一。
///
/// 注：设计文档字段名为 `default`，因其为 SQL 保留字，落地列名统一用 `default_value`，
/// 语义一致（占位符兜底默认值）。
#[derive(Clone, Debug)]
pub struct PromptPlaceholder {
    pub id: i32,
    /// 提示词 ID，关联 `prompt_info.id` 逻辑外键。
    pub prompt_id: i32,
    /// 占位符键，唯一标识（联合唯一约束 prompt_id + key）。
    pub key: String,
    /// 占位符标签（前端展示名称）。
    pub label: Option<String>,
    /// 占位符值（填充值）。
    pub value: Option<String>,
    /// 默认值（兜底），对应设计文档 `default` 字段。
    pub default_value: Option<String>,
    /// 输入类型：输入 / 下拉 / 文本域。
    pub input_type: Option<String>,
    /// 是否必填：0-否 1-是。
    pub required: i8,
    /// 启用状态：0-禁用 1-启用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for PromptPlaceholder {
    fn table_name() -> &'static str {
        "prompt_placeholder"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS prompt_placeholder (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prompt_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            label TEXT,
            value TEXT,
            default_value TEXT,
            input_type TEXT,
            required INTEGER NOT NULL DEFAULT 0,
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
            prompt_id: row.get("prompt_id")?,
            key: row.get("key")?,
            label: row.get("label")?,
            value: row.get("value")?,
            default_value: row.get("default_value")?,
            input_type: row.get("input_type")?,
            required: row.get("required")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "prompt_id, key, label, value, default_value, input_type, required, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.prompt_id),
            Box::new(self.key.clone()),
            Box::new(self.label.clone()),
            Box::new(self.value.clone()),
            Box::new(self.default_value.clone()),
            Box::new(self.input_type.clone()),
            Box::new(self.required),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        prompt_id = ?,
        key = ?,
        label = ?,
        value = ?,
        default_value = ?,
        input_type = ?,
        required = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.prompt_id),
            Box::new(self.key.clone()),
            Box::new(self.label.clone()),
            Box::new(self.value.clone()),
            Box::new(self.default_value.clone()),
            Box::new(self.input_type.clone()),
            Box::new(self.required),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
