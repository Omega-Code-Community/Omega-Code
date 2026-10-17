use rusqlite::Connection;
use anyhow::{Context, Result};
use log::{info};
use rust_i18n::t;
use crate::core::db::Model;
use crate::db::models::{
    ai_provider::AiProvider,
    llm_model::LLMModel,
    prompt_info::PromptInfo,
    agent_info::AgentInfo,
    prompt_placeholder::PromptPlaceholder,
    agent_arena::AgentArena,
    chat_session::ChatSession,
    chat_message::ChatMessage,
    memory_info::MemoryInfo,
    mcp_provider::McpProvider,
    mcp_tool::McpTool,
    agent_mcp_rel::AgentMcpRel,
    project_info::ProjectInfo,
    system_api_log::SystemApiLog,
    system_config::SystemConfig
};

pub fn init_all_tables(conn: &Connection) -> Result<()> {
    info!("Initializing database tables...");
    
    create_table_ai_provider(conn)
        .context("Failed to create ai_provider table")?;
    create_table_llm_model(conn)
        .context("Failed to create llm_model table")?;
    create_table_prompt_info(conn)
        .context("Failed to create prompt_info table")?;
    create_table_agent_info(conn)
        .context("Failed to create agent_info table")?;
    create_table_prompt_placeholder(conn)
        .context("Failed to create prompt_placeholder table")?;
    create_table_agent_arena(conn)
        .context("Failed to create agent_arena table")?;
    create_table_chat_session(conn)
        .context("Failed to create chat_session table")?;
    create_table_chat_message(conn)
        .context("Failed to create chat_message table")?;
    create_table_memory_info(conn)
        .context("Failed to create memory_info table")?;
    create_table_mcp_provider(conn)
        .context("Failed to create mcp_provider table")?;
    create_table_mcp_tool(conn)
        .context("Failed to create mcp_tool table")?;
    create_table_agent_mcp_rel(conn)
        .context("Failed to create agent_mcp_rel table")?;
    create_table_project_info(conn)
        .context("Failed to create project_info table")?;
    create_table_system_api_log(conn)
        .context("Failed to create system_api_log table")?;
    create_table_system_config(conn)
        .context("Failed to create system_config table")?;
    create_indexes(conn)
        .context("Failed to create indexes")?;
        
    info!("Database tables initialized successfully");
    Ok(())
}

// ============================================================
// Create table
// ============================================================
fn create_table_ai_provider(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='ai_provider'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(AiProvider::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "ai_provider"));
    }
    Ok(())
}

fn create_table_llm_model(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='llm_model'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(LLMModel::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "llm_model"));
    }
    Ok(())
}

fn create_table_prompt_info(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='prompt_info'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(PromptInfo::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "prompt_info"));
    }
    Ok(())
}

fn create_table_agent_info(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='agent_info'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(AgentInfo::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "agent_info"));
    }
    Ok(())
}

fn create_table_prompt_placeholder(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='prompt_placeholder'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(PromptPlaceholder::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "prompt_placeholder"));
    }
    Ok(())
}

fn create_table_agent_arena(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='agent_arena'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(AgentArena::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "agent_arena"));
    }
    Ok(())
}

fn create_table_chat_session(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='chat_session'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(ChatSession::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "chat_session"));
    }
    Ok(())
}

fn create_table_chat_message(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='chat_message'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(ChatMessage::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "chat_message"));
    }
    Ok(())
}

fn create_table_memory_info(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='memory_info'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(MemoryInfo::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "memory_info"));
    }
    Ok(())
}

fn create_table_mcp_provider(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='mcp_provider'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(McpProvider::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "mcp_provider"));
    }
    Ok(())
}

fn create_table_mcp_tool(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='mcp_tool'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(McpTool::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "mcp_tool"));
    }
    Ok(())
}

fn create_table_agent_mcp_rel(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='agent_mcp_rel'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(AgentMcpRel::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "agent_mcp_rel"));
    }
    Ok(())
}

fn create_table_project_info(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='project_info'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(ProjectInfo::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "project_info"));
    }
    Ok(())
}

fn create_table_system_api_log(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='system_api_log'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(SystemApiLog::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "system_api_log"));
    }
    Ok(())
}

fn create_table_system_config(conn: &Connection) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='system_config'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(SystemConfig::create_table_sql())?;
        info!("{}", t!("database_table_create", table_name = "system_config"));
    }
    Ok(())
}

// ============================================================
// Index
// ============================================================

fn create_indexes(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- 逻辑外键普通索引（关联字段）
        CREATE INDEX IF NOT EXISTS idx_chat_session_chat_uuid         ON chat_session(chat_uuid);
        CREATE INDEX IF NOT EXISTS idx_chat_message_chat_uuid         ON chat_message(chat_uuid);
        CREATE INDEX IF NOT EXISTS idx_chat_message_round_uuid        ON chat_message(round_uuid);
        CREATE INDEX IF NOT EXISTS idx_agent_arena_agent_id           ON agent_arena(agent_id);
        CREATE INDEX IF NOT EXISTS idx_agent_arena_chat_uuid          ON agent_arena(chat_uuid);
        CREATE INDEX IF NOT EXISTS idx_memory_info_memory_uuid        ON memory_info(memory_uuid);
        CREATE INDEX IF NOT EXISTS idx_agent_mcp_rel_agent_id         ON agent_mcp_rel(agent_id);
        CREATE INDEX IF NOT EXISTS idx_agent_mcp_rel_mcp_tool_id      ON agent_mcp_rel(mcp_tool_id);
        CREATE INDEX IF NOT EXISTS idx_prompt_placeholder_prompt_id   ON prompt_placeholder(prompt_id);
        CREATE INDEX IF NOT EXISTS idx_llm_model_provider_id          ON llm_model(provider_id);
        CREATE INDEX IF NOT EXISTS idx_mcp_tool_provider_id           ON mcp_tool(provider_id);
        CREATE INDEX IF NOT EXISTS idx_agent_info_llm_id              ON agent_info(llm_id);
        CREATE INDEX IF NOT EXISTS idx_agent_info_prompt_id           ON agent_info(prompt_id);
        CREATE INDEX IF NOT EXISTS idx_system_api_log_rel_uuid        ON system_api_log(rel_uuid);

        -- 唯一业务字段索引
        CREATE UNIQUE INDEX IF NOT EXISTS uk_llm_model_model_code       ON llm_model(model_code);
        CREATE UNIQUE INDEX IF NOT EXISTS uk_project_info_path          ON project_info(path);
        CREATE UNIQUE INDEX IF NOT EXISTS uk_prompt_placeholder_key     ON prompt_placeholder(prompt_id, key);
        ",
    )?;
    info!("{}", t!("database_table_create", table_name = "table_indexes"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::DatabaseManager;
    use crate::db::models::chat_message::ChatMessage;
    use crate::db::models::chat_session::ChatSession;
    use crate::db::models::project_info::ProjectInfo;

    /// 在内存库上执行全量建表 + 建索引，验证所有 CREATE TABLE / CREATE INDEX
    /// 语句在运行时均为合法 SQL（列名、唯一索引与设计文档 schema 一致）。
    fn mem_db() -> DatabaseManager {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        init_all_tables(&conn).expect("init_all_tables should succeed");
        DatabaseManager { conn }
    }

    #[test]
    fn init_all_tables_creates_schema_and_indexes() {
        let db = mem_db();
        // 15 张业务表均应存在
        let tables = [
            "ai_provider", "llm_model", "prompt_info", "agent_info",
            "prompt_placeholder", "agent_arena", "chat_session", "chat_message",
            "memory_info", "mcp_provider", "mcp_tool", "agent_mcp_rel",
            "project_info", "system_api_log", "system_config",
        ];
        for name in tables {
            let exists: bool = db
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "table {} should exist", name);
        }
        // 唯一索引应已创建
        let idx: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='uk_llm_model_model_code'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(idx, "unique index on llm_model(model_code) should exist");
    }

    #[test]
    fn chat_session_and_message_roundtrip() {
        let db = mem_db();
        let session = ChatSession {
            id: 0,
            chat_uuid: "chat-1".to_string(),
            title: Some("TUI Chat".to_string()),
            llm_id: 1,
            is_deleted: 0,
            create_time: chrono::Utc::now(),
            update_time: chrono::Utc::now(),
            remark: None,
        };
        let sid = db.create(session).unwrap();
        assert!(sid > 0);

        let msg = ChatMessage {
            id: 0,
            chat_uuid: "chat-1".to_string(),
            round_uuid: Some("round-1".to_string()),
            role: "user".to_string(),
            content: "hello".to_string(),
            input_tokens: 3,
            output_tokens: 5,
            duration: 42,
            model_config: Some("{\"temperature\":0.7}".to_string()),
            create_time: chrono::Utc::now(),
            update_time: chrono::Utc::now(),
            remark: None,
        };
        db.create(msg).unwrap();

        let loaded: Vec<ChatMessage> = db
            .find_where("chat_uuid = ?1", &[&"chat-1".to_string()])
            .unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "hello");
        assert_eq!(loaded[0].input_tokens, 3);
        assert_eq!(loaded[0].output_tokens, 5);
        assert_eq!(loaded[0].duration, 42);
        assert_eq!(loaded[0].round_uuid.as_deref(), Some("round-1"));
    }

    #[test]
    fn project_info_path_unique_index_enforced() {
        let db = mem_db();
        let mk = |path: &str| ProjectInfo {
            id: 0,
            name: "proj".to_string(),
            path: path.to_string(),
            create_at: None,
            updata_at: None,
            is_deleted: 0,
            create_time: chrono::Utc::now(),
            update_time: chrono::Utc::now(),
            remark: None,
        };
        db.create(mk("/a/b")).unwrap();
        // 相同 path 应触发唯一索引冲突
        assert!(db.create(mk("/a/b")).is_err());
        // 不同 path 允许
        db.create(mk("/c/d")).unwrap();
    }
}
