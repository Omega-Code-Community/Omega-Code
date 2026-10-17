use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::Result as RusqliteResult;

pub mod ai_provider;
pub mod llm_model;
pub mod agent_info;
pub mod prompt_info;
pub mod prompt_placeholder;
pub mod agent_arena;
pub mod chat_session;
pub mod chat_message;
pub mod memory_info;
pub mod mcp_provider;
pub mod mcp_tool;
pub mod agent_mcp_rel;
pub mod project_info;
pub mod system_api_log;
pub mod system_config;

/// 解析数据库时间字段。
///
/// 兼容两种格式：
/// - RFC3339（如 `2026-01-01T12:00:00Z`）；
/// - SQLite `CURRENT_TIMESTAMP` 产出的 `YYYY-MM-DD HH:MM:SS`。
///
/// 统一返回 UTC 时间，避免各 model 重复实现且格式不一致导致的解析失败。
pub fn parse_db_time(raw: &str) -> RusqliteResult<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f"))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}
