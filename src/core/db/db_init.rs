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
        CREATE INDEX IF NOT EXISTS idx_chat_session_chat_uuid         ON chat_session(chat_uuid);
        CREATE INDEX IF NOT EXISTS idx_chat_message_session_id        ON chat_message(session_id);
        CREATE INDEX IF NOT EXISTS idx_agent_arena_agent_id           ON agent_arena(agent_id);
        CREATE INDEX IF NOT EXISTS idx_agent_arena_arena_id           ON agent_arena(arena_id);
        CREATE INDEX IF NOT EXISTS idx_memory_info_session_id         ON memory_info(session_id);
        CREATE INDEX IF NOT EXISTS idx_agent_mcp_rel_agent_id         ON agent_mcp_rel(agent_id);
        CREATE INDEX IF NOT EXISTS idx_agent_mcp_rel_mcp_id           ON agent_mcp_rel(mcp_id);
        CREATE INDEX IF NOT EXISTS idx_prompt_placeholder_prompt_id   ON prompt_placeholder(prompt_id);
        CREATE INDEX IF NOT EXISTS idx_llm_model_provider_id          ON llm_model(provider_id);
        CREATE INDEX IF NOT EXISTS idx_mcp_tool_provider_id           ON mcp_tool(provider_id);
        CREATE INDEX IF NOT EXISTS idx_agent_info_model_id             ON agent_info(model_id);
        CREATE INDEX IF NOT EXISTS idx_agent_info_prompt_id           ON agent_info(prompt_id);
        ",
    )?;
    info!("{}", t!("database_table_create", table_name = "table_indexes"));
    Ok(())
}
