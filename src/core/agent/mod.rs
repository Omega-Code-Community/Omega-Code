//! Agent 系统
//!
//! 顶层 Agent 负责把底层能力编排成一次完整的对话轮次：
//! - **流式对话**：通过 [`Provider::complete_stream`] 增量获取模型输出，并以
//!   [`AgentEvent::Token`] 事件实时转发给上层（如 TUI）。
//! - **工具调用循环**：采用基于提示词的 ReAct 方式，解析模型输出中的工具调用意图，
//!   执行工具（见 [`crate::core::tools`]），把结果回灌后继续对话，直到模型给出最终答复。
//! - **会话持久化**：可选地把用户/助手/工具消息写入 `chat_message` 表，并支持加载历史。
//!
//! 使用方式：
//! ```ignore
//! let mut agent = Agent::from_config(&config)?;
//! agent.enable_persistence(1, Some("New Chat"))?;
//! let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
//! tokio::spawn(async move { agent.run_turn("hello", tx).await });
//! while let Some(event) = rx.recv().await { /* ... */ }
//! ```

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt};
use log::warn;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::core::config::{parse_provider_kind, AppConfig, DEFAULT_SYSTEM_PROMPT};
use crate::core::mcp::{self, McpToolSpec};
use crate::core::memory::{MemoryExtractor, MemoryScope, MemoryStore};
use crate::core::prompt;
use crate::core::providers::{
    create_provider, CompletionRequest, Message, Provider, ProviderConfig, ProviderKind, Role,
};
use crate::core::runtime::Runtime;
use crate::core::tools::ToolRegistry;
use crate::platform::get_database_path;

/// 默认最大工具调用迭代次数，防止无限循环。
pub const DEFAULT_MAX_ITERATIONS: usize = 8;

/// Agent 运行过程中向上层发送的事件。
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// 模型输出的一个增量文本片段。
    Token(String),
    /// Agent 决定调用某个工具。
    ToolCall { name: String, arguments: Value },
    /// 工具执行完成并返回结果。
    ToolResult { name: String, output: String },
    /// 本轮对话结束。
    Done,
    /// 发生错误（发送后本轮通常随之结束）。
    Error(String),
}

/// 解析出的一次工具调用。
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// 从模型输出中解析 `tool` 代码块形式的工具调用。
///
/// 约定格式：
/// ````text
/// ```tool
/// {"name": "read_file", "arguments": {"path": "src/main.rs"}}
/// ```
/// ````
pub fn parse_tool_call(content: &str) -> Option<ToolCall> {
    const MARKER: &str = "```tool";

    let start = content.find(MARKER)?;
    let after_marker = &content[start + MARKER.len()..];
    let end = after_marker.find("```")?;
    let json_str = after_marker[..end].trim();

    let value: Value = serde_json::from_str(json_str).ok()?;
    let name = value.get("name")?.as_str()?.to_string();
    let arguments = value.get("arguments").cloned().unwrap_or_else(|| json!({}));

    Some(ToolCall { name, arguments })
}

// ============================================================
// 会话持久化
// ============================================================

/// 定位/创建 provider→model 记录所需的信息。
#[derive(Debug, Clone)]
pub struct ModelRef {
    /// 供应商名称（在 `ai_provider` 中按名匹配），如 "compatible"。
    pub provider_name: String,
    /// 供应商 API Base URL。
    pub provider_api: String,
    /// 供应商 API 密钥（可选）。
    pub provider_key: Option<String>,
    /// 模型代码（`llm_model.model_code`），如 "gpt-4o-mini"。
    pub model_code: String,
    /// 模型展示名（可选，默认取 `model_code`）。
    pub model_name: Option<String>,
}

/// 从数据库加载的 Agent 完整配置（agent_info + llm_model + ai_provider + prompt_info + agent_mcp_rel）。
#[derive(Debug, Clone)]
pub struct DbAgentConfig {
    /// Agent 主键。
    pub agent_id: i32,
    /// Agent 名称。
    pub agent_name: String,
    /// 绑定的 LLM 模型 ID（`agent_info.llm_id`）。
    pub llm_id: i32,
    /// 绑定的 Prompt ID（`agent_info.prompt_id`，用于加载自定义占位符）。
    pub prompt_id: Option<i32>,
    /// 供应商名称（用于推断 [`ProviderKind`]）。
    pub provider_name: String,
    /// 供应商 API Base URL。
    pub base_url: String,
    /// 供应商 API 密钥。
    pub api_key: Option<String>,
    /// 模型调用标识（`llm_model.model_code`）。
    pub model_code: String,
    /// 单次输出上限（`llm_model.max_response_tokens`，0 表示不限）。
    pub max_response_tokens: i32,
    /// 系统提示词（`prompt_info.content`，可能为空）。
    pub system_prompt: String,
    /// 采样温度。
    pub temperature: Option<f32>,
    /// 核采样 top_p。
    pub top_p: Option<f32>,
    /// 授权的 MCP 工具 ID 列表（`agent_mcp_rel`）。
    pub mcp_tool_ids: Vec<i32>,
}

/// 待持久化的一条消息记录（对应 `chat_message` 新 schema）。
pub struct MessageRecord<'a> {
    /// 会话 UUID，关联 `chat_session.chat_uuid`。
    pub chat_uuid: &'a str,
    /// 单轮消息 UUID，同一轮 user/assistant/tool 共享。
    pub round_uuid: Option<&'a str>,
    /// 消息角色：user/assistant/system/tool。
    pub role: &'a str,
    /// 消息正文。
    pub content: &'a str,
    /// 单轮输入 Token 消耗。
    pub input_tokens: i32,
    /// 单轮输出 Token 消耗。
    pub output_tokens: i32,
    /// 调用耗时（毫秒）。
    pub duration: i32,
    /// 模型参数快照（JSON）。
    pub model_config: Option<&'a str>,
}

/// 会话存储：把对话消息持久化到 SQLite。
///
/// 内部使用 [`Mutex<Connection>`] 保证 `Send + Sync`，可安全地被 Agent 在异步任务中共享。
pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    /// 打开（或创建）应用数据库连接。
    pub fn new() -> Result<Self> {
        let path = get_database_path()?;
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow::anyhow!("SessionStore mutex poisoned"))
    }

    /// 测试专用：用已有连接构造（例如内存库）。
    #[cfg(test)]
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// 确保 `ai_provider`→`llm_model` 记录存在，返回 `llm_model` 的 id。
    ///
    /// 供应商按 `name` 匹配、模型按 `(provider_id, model_code)` 匹配；缺失时以真实配置创建，
    /// 从而建立正确的 provider→model 关联（而非占位的 `provider_id = 0`）。
    pub fn ensure_model(&self, m: &ModelRef) -> Result<i32> {
        let conn = self.lock()?;

        // 1) 查/建供应商。
        let provider_id: i32 = match conn
            .query_row(
                "SELECT id FROM ai_provider WHERE name = ?1 ORDER BY id ASC LIMIT 1",
                params![m.provider_name],
                |row| row.get(0),
            )
            .optional()?
        {
            Some(id) => id,
            None => {
                conn.execute(
                    "INSERT INTO ai_provider (name, avatar, api, key, timeout, status) \
                     VALUES (?1, NULL, ?2, ?3, 30, 1)",
                    params![m.provider_name, m.provider_api, m.provider_key],
                )?;
                conn.last_insert_rowid() as i32
            }
        };

        // 2) 查/建模型。
        let model_name = m.model_name.clone().unwrap_or_else(|| m.model_code.clone());
        let model_id: i32 = match conn
            .query_row(
                "SELECT id FROM llm_model WHERE provider_id = ?1 AND model_code = ?2 \
                 ORDER BY id ASC LIMIT 1",
                params![provider_id, m.model_code],
                |row| row.get(0),
            )
            .optional()?
        {
            Some(id) => id,
            None => {
                conn.execute(
                    "INSERT INTO llm_model (provider_id, name, avatar, model_code, max_tokens, max_response_tokens, weight, call_count, status) \
                     VALUES (?1, ?2, NULL, ?3, 0, 0, 0, 0, 1)",
                    params![provider_id, model_name, m.model_code],
                )?;
                conn.last_insert_rowid() as i32
            }
        };

        Ok(model_id)
    }

    /// 创建一条会话记录。
    pub fn create_session(&self, uuid: &str, title: Option<&str>, llm_id: i32) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO chat_session (chat_uuid, title, llm_id, is_deleted) VALUES (?1, ?2, ?3, 0)",
            params![uuid, title, llm_id],
        )?;
        Ok(())
    }

    /// 保存一条消息（`chat_message` 新 schema：按 `round_uuid` 分轮，记录 Token/耗时/模型快照）。
    pub fn save_message(&self, rec: &MessageRecord) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO chat_message \
             (chat_uuid, round_uuid, role, content, input_tokens, output_tokens, duration, model_config) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                rec.chat_uuid,
                rec.round_uuid,
                rec.role,
                rec.content,
                rec.input_tokens,
                rec.output_tokens,
                rec.duration,
                rec.model_config,
            ],
        )?;
        Ok(())
    }

    /// 加载某个会话的历史消息（按时间升序）。
    pub fn load_history(&self, session_uuid: &str) -> Result<Vec<Message>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT role, content FROM chat_message WHERE chat_uuid = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![session_uuid], |row| {
            let role: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((role, content))
        })?;

        let mut history = Vec::new();
        for row in rows {
            let (role, content) = row?;
            let role = match role.as_str() {
                "system" => Role::System,
                "assistant" => Role::Assistant,
                _ => Role::User,
            };
            history.push(Message { role, content });
        }

        Ok(history)
    }

    /// 从数据库加载某个 Agent 的完整配置。
    ///
    /// 关联查询 `agent_info`→`llm_model`→`ai_provider` 与 `prompt_info`，
    /// 并读取 `agent_mcp_rel` 得到授权的 MCP 工具 ID 列表。任一主链缺失时报错。
    pub fn load_agent_config(&self, agent_id: i32) -> Result<DbAgentConfig> {
        let conn = self.lock()?;

        let row = conn
            .query_row(
                "SELECT a.name, a.llm_id, a.prompt_id, a.temperature, a.top_p, \
                        m.model_code, m.max_response_tokens, \
                        p.name, p.api, p.key \
                 FROM agent_info a \
                 JOIN llm_model m ON m.id = a.llm_id \
                 JOIN ai_provider p ON p.id = m.provider_id \
                 WHERE a.id = ?1",
                params![agent_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i32>(1)?,
                        row.get::<_, Option<i32>>(2)?,
                        row.get::<_, Option<f32>>(3)?,
                        row.get::<_, Option<f32>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i32>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, Option<String>>(9)?,
                    ))
                },
            )
            .optional()?
            .with_context(|| {
                format!(
                    "agent_info id={} not found or its llm_model/ai_provider is missing",
                    agent_id
                )
            })?;

        let (
            agent_name,
            llm_id,
            prompt_id,
            temperature,
            top_p,
            model_code,
            max_response_tokens,
            provider_name,
            base_url,
            api_key,
        ) = row;

        // 提示词内容（可选）。
        let system_prompt = match prompt_id {
            Some(pid) => conn
                .query_row(
                    "SELECT content FROM prompt_info WHERE id = ?1",
                    params![pid],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_default(),
            None => String::new(),
        };

        // 授权的 MCP 工具 ID。
        let mut stmt =
            conn.prepare("SELECT mcp_tool_id FROM agent_mcp_rel WHERE agent_id = ?1")?;
        let mcp_tool_ids: Vec<i32> = stmt
            .query_map(params![agent_id], |row| row.get::<_, i32>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(DbAgentConfig {
            agent_id,
            agent_name,
            llm_id,
            prompt_id,
            provider_name,
            base_url,
            api_key,
            model_code,
            max_response_tokens,
            system_prompt,
            temperature,
            top_p,
            mcp_tool_ids,
        })
    }

    /// 加载某个 Agent 已授权且启用的 MCP 工具规格（用于工具调度）。
    ///
    /// 关联 `agent_mcp_rel`→`mcp_tool`→`mcp_provider`，仅返回工具与提供方均处于
    /// 启用状态（`status = 1`）的记录，从而在执行前完成权限与可用性过滤。
    pub fn load_agent_mcp_tools(&self, agent_id: i32) -> Result<Vec<McpToolSpec>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.tool_code, t.name, t.description, t.parameters, p.url \
             FROM agent_mcp_rel r \
             JOIN mcp_tool t ON t.id = r.mcp_tool_id \
             JOIN mcp_provider p ON p.id = t.provider_id \
             WHERE r.agent_id = ?1 AND t.status = 1 AND p.status = 1",
        )?;

        let specs = stmt
            .query_map(params![agent_id], |row| {
                Ok(McpToolSpec {
                    tool_id: row.get(0)?,
                    tool_code: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    parameters: row.get(4)?,
                    provider_url: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(specs)
    }

    /// 加载可被当前 Agent 调用的其它启用 Agent 名称（用于 `{AgentList}` 占位符）。
    ///
    /// 系统未建立 agent-agent 关联表，故取「除自身外所有 `status = 1` 的 Agent」作为合理近似。
    pub fn load_callable_agents(&self, exclude_id: i32) -> Result<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT name FROM agent_info WHERE status = 1 AND id != ?1 ORDER BY id ASC")?;
        let names = stmt
            .query_map(params![exclude_id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(names)
    }

    /// 加载某个 Prompt 的启用占位符（`key`, `value`, `default_value`），用于自定义占位符解析。
    pub fn load_prompt_placeholders(
        &self,
        prompt_id: i32,
    ) -> Result<Vec<(String, Option<String>, Option<String>)>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT key, value, default_value FROM prompt_placeholder \
             WHERE prompt_id = ?1 AND status = 1",
        )?;
        let rows = stmt
            .query_map(params![prompt_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 记录一次外部 API（大模型）调用日志到 `system_api_log`。
    ///
    /// `rel_uuid` 关联业务实体（如 `chat_message.round_uuid`），`status` 为 1 成功 / 0 失败。
    pub fn log_api(
        &self,
        rel_uuid: Option<&str>,
        status: i8,
        token_cost: i32,
        duration: i32,
        content: Option<&str>,
    ) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO system_api_log (rel_uuid, status, token_cost, duration, content) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rel_uuid, status, token_cost, duration, content],
        )?;
        Ok(())
    }
}

// ============================================================
// Agent
// ============================================================

/// 顶层对话 Agent。
pub struct Agent {
    provider: Box<dyn Provider>,
    model: String,
    system_prompt: String,
    tools: ToolRegistry,
    history: Vec<Message>,
    max_iterations: usize,
    store: Option<Arc<SessionStore>>,
    session_uuid: String,
    /// 供应商元数据（用于持久化时建立 provider→model 关联）。
    provider_name: String,
    provider_api: String,
    provider_key: Option<String>,
    /// 采样温度（来自 Agent 配置，作用于每次请求）。
    temperature: Option<f32>,
    /// 核采样 top_p（记录于模型快照；当前请求体未透传，留待 provider 扩展）。
    top_p: Option<f32>,
    /// 单次输出上限。
    max_tokens: Option<u32>,
    /// 数据库驱动的 Agent 主键（`from_database` 时设置）。
    agent_id: Option<i32>,
    /// 数据库驱动的 LLM 模型主键（`start_session` 建会话时使用）。
    llm_id: Option<i32>,
    /// 已渲染的记忆上下文（会话初始化时一次性注入系统提示词）。
    memory_context: String,
    /// 记忆抽取器（作用域非 None 时启用，每轮对话后异步抽取记忆）。
    extractor: Option<Arc<MemoryExtractor>>,
    /// 最近一轮对话的 round_uuid（供 TUI 内投票关联本轮回复）。
    last_round_uuid: Option<String>,
}

impl Agent {
    /// 使用给定组件构造 Agent。
    pub fn new(
        provider: Box<dyn Provider>,
        model: String,
        system_prompt: String,
        tools: ToolRegistry,
    ) -> Self {
        Self {
            provider,
            model,
            system_prompt,
            tools,
            history: Vec::new(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            store: None,
            session_uuid: generate_uuid(),
            provider_name: String::new(),
            provider_api: String::new(),
            provider_key: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            agent_id: None,
            llm_id: None,
            memory_context: String::new(),
            extractor: None,
            last_round_uuid: None,
        }
    }

    /// 基于 [`AppConfig`] 构造 Agent，自动创建供应商并注册默认内置工具。
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let provider = create_provider(config.provider_config())?;
        let runtime = Arc::new(Runtime::new(config.working_dir.clone()));
        let tools = ToolRegistry::with_defaults(runtime);

        let key = if config.api_key.is_empty() {
            None
        } else {
            Some(config.api_key.clone())
        };

        Ok(Self::new(
            provider,
            config.model.clone(),
            config.system_prompt.clone(),
            tools,
        )
        .with_provider_meta(config.provider_kind.as_str(), config.base_url.clone(), key)
        .with_sampling(config.temperature, None, config.max_tokens))
    }

    /// 设置供应商元数据（用于持久化时建立 provider→model 关联）。
    pub fn with_provider_meta(
        mut self,
        name: impl Into<String>,
        api: impl Into<String>,
        key: Option<String>,
    ) -> Self {
        self.provider_name = name.into();
        self.provider_api = api.into();
        self.provider_key = key;
        self
    }

    /// 设置采样参数（温度 / top_p / 单次输出上限）。
    pub fn with_sampling(
        mut self,
        temperature: Option<f32>,
        top_p: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Self {
        self.temperature = temperature;
        self.top_p = top_p;
        self.max_tokens = max_tokens;
        self
    }

    /// 基于数据库配置构造 Agent（数据库驱动）。
    ///
    /// 从 `agent_info`+`llm_model`+`ai_provider`+`prompt_info`+`agent_mcp_rel` 加载完整配置，
    /// 据此创建供应商、系统提示词与采样参数，并绑定持久化存储（会话由 [`Agent::start_session`] 创建）。
    /// `mcp_tool_ids` 供后续 MCP 工具调度做权限校验（见 `crate::core::tools`）。
    pub fn from_database(agent_id: i32, working_dir: PathBuf) -> Result<Self> {
        let store = SessionStore::new()?;
        let cfg = store.load_agent_config(agent_id)?;
        // 已授权且启用的 MCP 工具（权限校验：仅 agent_mcp_rel 中登记且状态启用者可用）。
        let mcp_specs = store.load_agent_mcp_tools(agent_id)?;
        // 先收集工具名称用于 `{MCPList}` 占位符，再交给 build_tools 消费 mcp_specs。
        let mcp_names: Vec<String> = mcp_specs.iter().map(|s| s.name.clone()).collect();

        let provider_kind =
            parse_provider_kind(&cfg.provider_name).unwrap_or(ProviderKind::Compatible);
        let provider = create_provider(ProviderConfig {
            kind: provider_kind,
            api_key: cfg.api_key.clone().unwrap_or_default(),
            base_url: cfg.base_url.clone(),
            model: cfg.model_code.clone(),
        })?;

        let runtime = Arc::new(Runtime::new(working_dir));
        let mut tools = ToolRegistry::with_defaults(runtime);
        for tool in mcp::build_tools(mcp_specs) {
            tools.register(tool);
        }

        let raw_prompt = if cfg.system_prompt.trim().is_empty() {
            DEFAULT_SYSTEM_PROMPT.to_string()
        } else {
            cfg.system_prompt.clone()
        };

        // 解析系统提示词占位符：`{AgentList}`=可调用 Agent，`{MCPList}`=已授权工具，
        // 其余自定义占位符来自 prompt_placeholder（加载失败不阻断，退化为空）。
        let callable = store.load_callable_agents(cfg.agent_id).unwrap_or_default();
        let placeholders = cfg
            .prompt_id
            .map(|pid| store.load_prompt_placeholders(pid).unwrap_or_default())
            .unwrap_or_default();
        let vars = prompt::build_vars(&callable, &mcp_names, &placeholders);
        let system_prompt = prompt::render(&raw_prompt, &vars);

        let max_tokens = if cfg.max_response_tokens > 0 {
            Some(cfg.max_response_tokens as u32)
        } else {
            None
        };

        let mut agent = Self::new(provider, cfg.model_code.clone(), system_prompt, tools)
            .with_provider_meta(&cfg.provider_name, &cfg.base_url, cfg.api_key.clone())
            .with_sampling(cfg.temperature, cfg.top_p, max_tokens);
        agent.agent_id = Some(cfg.agent_id);
        agent.llm_id = Some(cfg.llm_id);
        agent.store = Some(Arc::new(store));
        Ok(agent)
    }

    /// 将该 Agent 升级为多 Agent 调度器（Coordinator）。
    ///
    /// 复用自身供应商/模型配置构造 [`crate::core::scheduler::AgentWorkerFactory`]，注册四个
    /// 调度工具（create/resume/stop/await worker），追加协调者提示词，并放宽工具迭代上限
    /// （单工具/轮 + 默认 8 次不足以协调多 Worker）。仅用于被标记为 `coordinator_agent_id`
    /// 的 Agent（见 `src/pages/chat.rs`）。
    pub fn enable_coordinator(&mut self, working_dir: PathBuf) -> Result<()> {
        use crate::core::scheduler::{self, AgentWorkerFactory, WorkerPool};

        // `provider_api` 即 base_url（见 `with_provider_meta`）。
        let provider_config = ProviderConfig {
            kind: parse_provider_kind(&self.provider_name).unwrap_or(ProviderKind::Compatible),
            api_key: self.provider_key.clone().unwrap_or_default(),
            base_url: self.provider_api.clone(),
            model: self.model.clone(),
        };
        let factory = Arc::new(AgentWorkerFactory {
            provider_config,
            working_dir,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
        });
        let pool = Arc::new(WorkerPool::default());
        for tool in scheduler::build_tools(pool, factory) {
            self.tools.register(tool);
        }
        self.system_prompt.push_str(scheduler::COORDINATOR_PROMPT);
        self.max_iterations = self.max_iterations.max(24);
        Ok(())
    }

    /// 为该 Agent 启用记忆系统。
    ///
    /// 立即渲染当前作用域下的记忆上下文（一次性注入系统提示词，见 [`Agent::build_system_prompt`]）；
    /// 若作用域非 [`MemoryScope::None`]，则复用自身供应商/模型配置构造 [`MemoryExtractor`]，
    /// 在每轮对话结束后异步抽取并落库记忆（见 `run_turn`）。记忆上下文不随本轮抽取刷新，
    /// 新记忆下次会话生效（符合「黑盒自动管理」，避免每轮读文件）。
    pub fn enable_memory(&mut self, store: Arc<MemoryStore>, scope: MemoryScope) -> Result<()> {
        self.memory_context = store.render_context(scope)?;
        if scope != MemoryScope::None {
            // `provider_api` 即 base_url（见 `with_provider_meta`）。
            let provider_config = ProviderConfig {
                kind: parse_provider_kind(&self.provider_name).unwrap_or(ProviderKind::Compatible),
                api_key: self.provider_key.clone().unwrap_or_default(),
                base_url: self.provider_api.clone(),
                model: self.model.clone(),
            };
            self.extractor = Some(Arc::new(MemoryExtractor::new(
                provider_config,
                self.model.clone(),
                self.temperature,
                store,
                scope,
            )));
        }
        Ok(())
    }

    /// 数据库驱动 Agent 的数据库配置主键（若非数据库驱动则为 `None`）。
    pub fn agent_id(&self) -> Option<i32> {
        self.agent_id
    }

    /// 使用已绑定的存储创建新会话（数据库驱动场景，`llm_id` 已知）。
    ///
    /// 若尚未绑定存储则自动打开一个。会话标题缺省时用 Agent 名称或 "Chat"。
    pub fn start_session(&mut self, title: Option<&str>) -> Result<()> {
        let store = match &self.store {
            Some(s) => s.clone(),
            None => Arc::new(SessionStore::new()?),
        };
        let llm_id = self.llm_id.unwrap_or(0);
        let uuid = generate_uuid();
        store.create_session(&uuid, title, llm_id)?;
        self.store = Some(store);
        self.session_uuid = uuid;
        Ok(())
    }

    /// 设置最大工具调用迭代次数。
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations.max(1);
        self
    }

    /// 设置自定义工具注册表。
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    /// 当前会话 UUID。
    pub fn session_uuid(&self) -> &str {
        &self.session_uuid
    }

    /// 最近一轮对话的 round_uuid（尚无完成轮次时为 `None`）。
    pub fn last_round_uuid(&self) -> Option<&str> {
        self.last_round_uuid.as_deref()
    }

    /// 当前对话历史。
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// 开启持久化：创建新会话并绑定存储。
    pub fn enable_persistence(&mut self, llm_id: i32, title: Option<&str>) -> Result<()> {
        let store = Arc::new(SessionStore::new()?);
        let uuid = generate_uuid();
        store.create_session(&uuid, title, llm_id)?;
        self.store = Some(store);
        self.session_uuid = uuid;
        Ok(())
    }

    /// 基于当前模型配置开启持久化：自动确保 provider→model 记录存在并创建新会话。
    ///
    /// 适用于配置来自环境变量、没有显式 `llm_id` 的场景（如 TUI）。
    pub fn enable_persistence_auto(&mut self, title: Option<&str>) -> Result<()> {
        let store = Arc::new(SessionStore::new()?);
        let model_ref = ModelRef {
            provider_name: if self.provider_name.is_empty() {
                "custom".to_string()
            } else {
                self.provider_name.clone()
            },
            provider_api: self.provider_api.clone(),
            provider_key: self.provider_key.clone(),
            model_code: self.model.clone(),
            model_name: None,
        };
        let llm_id = store.ensure_model(&model_ref)?;
        let uuid = generate_uuid();
        store.create_session(&uuid, title, llm_id)?;
        self.store = Some(store);
        self.session_uuid = uuid;
        Ok(())
    }

    /// 加载已有会话：读取历史消息并绑定存储。
    pub fn load_session(&mut self, session_uuid: String) -> Result<()> {
        let store = Arc::new(SessionStore::new()?);
        self.history = store.load_history(&session_uuid)?;
        self.store = Some(store);
        self.session_uuid = session_uuid;
        Ok(())
    }

    /// 执行一轮完整对话（含流式输出与工具调用循环），过程事件通过 `tx` 发送。
    pub async fn run_turn(
        &mut self,
        user_input: &str,
        tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        self.history.push(Message {
            role: Role::User,
            content: user_input.to_string(),
        });
        // 本轮对话共享同一个 round_uuid，用于在 chat_message 中聚合单轮消息。
        let round_uuid = generate_uuid();
        self.last_round_uuid = Some(round_uuid.clone());
        let round = round_uuid.as_str();
        let input_tokens = estimate_tokens(user_input);
        self.persist(Some(round), "user", user_input, input_tokens, 0, 0);

        let mut iteration = 0;
        loop {
            iteration += 1;

            let mut messages = Vec::with_capacity(self.history.len() + 1);
            messages.push(Message {
                role: Role::System,
                content: self.build_system_prompt(),
            });
            messages.extend(self.history.iter().cloned());

            // 请求侧 Token 估算（流式无真实 usage，用字符数近似）。
            let req_tokens: i32 = messages.iter().map(|m| estimate_tokens(&m.content)).sum();

            let request = CompletionRequest {
                model: self.model.clone(),
                messages,
                temperature: self.temperature,
                max_tokens: self.max_tokens,
                stream: true,
            };

            let started = Instant::now();
            let mut stream = match self.provider.complete_stream(request).await {
                Ok(stream) => stream,
                Err(e) => {
                    let elapsed = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
                    let msg = e.to_string();
                    self.persist_api_log(Some(round), 0, 0, elapsed, Some(&msg));
                    let _ = tx.send(AgentEvent::Error(msg));
                    return Err(e);
                }
            };

            let mut accumulated = String::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(delta) => {
                        if !delta.content.is_empty() {
                            accumulated.push_str(&delta.content);
                            let _ = tx.send(AgentEvent::Token(delta.content));
                        }
                    }
                    Err(e) => {
                        let elapsed = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
                        let msg = e.to_string();
                        self.persist_api_log(Some(round), 0, req_tokens, elapsed, Some(&msg));
                        let _ = tx.send(AgentEvent::Error(msg));
                        return Err(e);
                    }
                }
            }
            let duration = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
            let output_tokens = estimate_tokens(&accumulated);
            self.persist_api_log(Some(round), 1, req_tokens + output_tokens, duration, None);

            self.history.push(Message {
                role: Role::Assistant,
                content: accumulated.clone(),
            });
            self.persist(Some(round), "assistant", &accumulated, 0, output_tokens, duration);

            // 若模型请求调用工具且尚未超出迭代上限，则执行并把结果回灌。
            if iteration < self.max_iterations {
                if let Some(call) = parse_tool_call(&accumulated) {
                    let _ = tx.send(AgentEvent::ToolCall {
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    });

                    let output = match self.tools.execute(&call.name, call.arguments.clone()).await
                    {
                        Ok(output) => output,
                        Err(e) => format!("Tool '{}' failed: {}", call.name, e),
                    };

                    let _ = tx.send(AgentEvent::ToolResult {
                        name: call.name.clone(),
                        output: output.clone(),
                    });

                    let tool_message = format!("[Tool result for '{}']\n{}", call.name, output);
                    self.history.push(Message {
                        role: Role::User,
                        content: tool_message.clone(),
                    });
                    self.persist(Some(round), "tool", &tool_message, 0, 0, 0);
                    continue;
                }
            }

            // 一轮结束后异步抽取记忆（作用域非 None 且有助手输出时）。
            if !accumulated.trim().is_empty() {
                if let Some(extractor) = &self.extractor {
                    let ex = extractor.clone();
                    let round_messages = vec![
                        Message {
                            role: Role::User,
                            content: user_input.to_string(),
                        },
                        Message {
                            role: Role::Assistant,
                            content: accumulated.clone(),
                        },
                    ];
                    tokio::spawn(async move {
                        if let Err(e) = ex.extract_from_round(&round_messages).await {
                            warn!("memory extraction failed: {}", e);
                        }
                    });
                }
            }

            let _ = tx.send(AgentEvent::Done);
            break;
        }

        Ok(())
    }

    /// 便捷方法：消费自身执行单轮对话，返回一个事件流。
    ///
    /// 适用于「一次性」调用；多轮对话请保留 Agent 并重复调用 [`Agent::run_turn`]。
    pub fn into_stream(mut self, user_input: String) -> impl Stream<Item = AgentEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            if let Err(e) = self.run_turn(&user_input, tx.clone()).await {
                let _ = tx.send(AgentEvent::Error(e.to_string()));
            }
        });
        tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
    }

    /// 组合系统提示词、记忆上下文与工具说明。
    fn build_system_prompt(&self) -> String {
        let tools = self.tools.definitions_prompt();
        let mut prompt = self.system_prompt.clone();
        if !self.memory_context.trim().is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&self.memory_context);
        }
        if !tools.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&tools);
        }
        prompt
    }

    /// 持久化一条消息（若已开启持久化）。
    fn persist(
        &self,
        round_uuid: Option<&str>,
        role: &str,
        content: &str,
        input_tokens: i32,
        output_tokens: i32,
        duration: i32,
    ) {
        if let Some(store) = &self.store {
            // 仅助手消息记录模型参数快照。
            let snapshot = if role == "assistant" {
                Some(self.model_config_snapshot())
            } else {
                None
            };
            let rec = MessageRecord {
                chat_uuid: &self.session_uuid,
                round_uuid,
                role,
                content,
                input_tokens,
                output_tokens,
                duration,
                model_config: snapshot.as_deref(),
            };
            if let Err(e) = store.save_message(&rec) {
                warn!("Failed to persist '{}' message: {}", role, e);
            }
        }
    }

    /// 记录一次主对话 LLM 调用日志到 `system_api_log`（若已开启持久化）。
    ///
    /// 日志失败仅告警，不影响对话主流程。仅记录主对话调用（记忆抽取、MCP 工具调用不计）。
    fn persist_api_log(
        &self,
        rel_uuid: Option<&str>,
        status: i8,
        token_cost: i32,
        duration: i32,
        content: Option<&str>,
    ) {
        if let Some(store) = &self.store {
            if let Err(e) = store.log_api(rel_uuid, status, token_cost, duration, content) {
                warn!("Failed to log api call: {}", e);
            }
        }
    }

    /// 构建模型参数快照（JSON），记录本轮实际使用的模型与关键参数。
    fn model_config_snapshot(&self) -> String {
        json!({
            "model": self.model,
            "provider": self.provider_name,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "max_tokens": self.max_tokens,
            "max_iterations": self.max_iterations,
        })
        .to_string()
    }
}

/// 粗略估算文本 Token 数（约每 4 个字符 1 个 token）。
///
/// 流式增量不携带 usage 统计，此处用字符数近似，保证 `chat_message`
/// 的 Token 列有可用值；若后续 provider 暴露真实 usage 可直接替换。
fn estimate_tokens(text: &str) -> i32 {
    let chars = text.chars().count();
    if chars == 0 {
        return 0;
    }
    ((chars + 3) / 4) as i32
}

/// 生成一个会话唯一标识（无外部 uuid 依赖）。
fn generate_uuid() -> String {
    use rand::RngExt;

    let ts = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default();
    let rand_part: u32 = rand::rng().random_range(0..u32::MAX);

    format!("{:x}-{:08x}", ts, rand_part)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_call_valid() {
        let content = "Let me read the file.\n```tool\n{\"name\":\"read_file\",\"arguments\":{\"path\":\"a.rs\"}}\n```";
        let call = parse_tool_call(content).unwrap();
        assert_eq!(call.name, "read_file");
        assert_eq!(call.arguments["path"], "a.rs");
    }

    #[test]
    fn parse_tool_call_none() {
        assert!(parse_tool_call("just a normal answer").is_none());
    }

    #[test]
    fn uuid_is_unique_enough() {
        assert_ne!(generate_uuid(), generate_uuid());
    }

    #[test]
    fn estimate_tokens_approximates_by_chars() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    /// 构造含完整关联链的内存库，验证 `load_agent_config` 的 JOIN 查询。
    #[test]
    fn load_agent_config_joins_provider_model_prompt() {
        use crate::core::db::Model;
        use crate::db::models::agent_info::AgentInfo;
        use crate::db::models::agent_mcp_rel::AgentMcpRel;
        use crate::db::models::ai_provider::AiProvider;
        use crate::db::models::llm_model::LLMModel;
        use crate::db::models::prompt_info::PromptInfo;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(AiProvider::create_table_sql()).unwrap();
        conn.execute_batch(LLMModel::create_table_sql()).unwrap();
        conn.execute_batch(PromptInfo::create_table_sql()).unwrap();
        conn.execute_batch(AgentInfo::create_table_sql()).unwrap();
        conn.execute_batch(AgentMcpRel::create_table_sql()).unwrap();

        conn.execute(
            "INSERT INTO ai_provider (id, name, api, key, timeout, status) VALUES (1, 'openai', 'https://api.openai.com/v1', 'sk-x', 30, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO llm_model (id, provider_id, name, model_code, max_response_tokens, status) VALUES (10, 1, 'GPT-4o', 'gpt-4o', 4096, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO prompt_info (id, name, content) VALUES (100, 'sys', 'You are helpful.')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_info (id, llm_id, name, prompt_id, temperature, top_p, status) VALUES (5, 10, 'Coder', 100, 0.3, 0.9, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_mcp_rel (agent_id, mcp_tool_id) VALUES (5, 77)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_mcp_rel (agent_id, mcp_tool_id) VALUES (5, 88)",
            [],
        ).unwrap();

        let store = SessionStore::from_connection(conn);
        let cfg = store.load_agent_config(5).unwrap();
        assert_eq!(cfg.agent_name, "Coder");
        assert_eq!(cfg.llm_id, 10);
        assert_eq!(cfg.provider_name, "openai");
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-x"));
        assert_eq!(cfg.model_code, "gpt-4o");
        assert_eq!(cfg.max_response_tokens, 4096);
        assert_eq!(cfg.system_prompt, "You are helpful.");
        assert_eq!(cfg.temperature, Some(0.3));
        assert_eq!(cfg.top_p, Some(0.9));
        assert_eq!(cfg.mcp_tool_ids, vec![77, 88]);

        // 不存在的 agent 应报错。
        assert!(store.load_agent_config(999).is_err());
    }

    /// 验证 `load_agent_mcp_tools` 的三表 JOIN 与启用状态过滤。
    #[test]
    fn load_agent_mcp_tools_filters_disabled() {
        use crate::core::db::Model;
        use crate::db::models::agent_mcp_rel::AgentMcpRel;
        use crate::db::models::mcp_provider::McpProvider;
        use crate::db::models::mcp_tool::McpTool;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(McpProvider::create_table_sql()).unwrap();
        conn.execute_batch(McpTool::create_table_sql()).unwrap();
        conn.execute_batch(AgentMcpRel::create_table_sql()).unwrap();

        // 两个 provider：一个启用、一个停用。
        conn.execute(
            "INSERT INTO mcp_provider (id, name, url, timeout, status) VALUES (1, 'py', 'http://localhost:8000/mcp', 30, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO mcp_provider (id, name, url, timeout, status) VALUES (2, 'off', 'http://localhost:9000/mcp', 30, 0)",
            [],
        ).unwrap();
        // 工具：add（启用）、echo（停用）、calc（启用但归属停用 provider）。
        conn.execute(
            "INSERT INTO mcp_tool (id, provider_id, name, tool_code, description, parameters, status) VALUES (77, 1, 'Add', 'add', 'add two', '{\"type\":\"object\"}', 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO mcp_tool (id, provider_id, name, tool_code, status) VALUES (78, 1, 'Echo', 'echo', 0)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO mcp_tool (id, provider_id, name, tool_code, status) VALUES (79, 2, 'Calc', 'calc', 1)",
            [],
        ).unwrap();
        // agent 5 授权全部三个工具。
        for tool_id in [77, 78, 79] {
            conn.execute(
                "INSERT INTO agent_mcp_rel (agent_id, mcp_tool_id) VALUES (5, ?1)",
                params![tool_id],
            ).unwrap();
        }

        let store = SessionStore::from_connection(conn);
        let specs = store.load_agent_mcp_tools(5).unwrap();
        // 仅 add 满足「工具启用 且 provider 启用」。
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].tool_id, 77);
        assert_eq!(specs[0].tool_code, "add");
        assert_eq!(specs[0].name, "Add");
        assert_eq!(specs[0].description.as_deref(), Some("add two"));
        assert_eq!(specs[0].parameters.as_deref(), Some("{\"type\":\"object\"}"));
        assert_eq!(specs[0].provider_url, "http://localhost:8000/mcp");

        // 未授权的 agent 得到空集合。
        assert!(store.load_agent_mcp_tools(999).unwrap().is_empty());
    }

    /// 验证 `log_api` 写入 `system_api_log` 后可查得各列。
    #[test]
    fn log_api_persists_row() {
        use crate::core::db::Model;
        use crate::db::models::system_api_log::SystemApiLog;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SystemApiLog::create_table_sql()).unwrap();
        let store = SessionStore::from_connection(conn);

        store
            .log_api(Some("round-1"), 1, 42, 123, None)
            .unwrap();
        store
            .log_api(Some("round-2"), 0, 0, 5, Some("boom"))
            .unwrap();

        let conn = store.conn.lock().unwrap();
        let (status, token_cost, duration, rel_uuid): (i8, i32, i32, Option<String>) = conn
            .query_row(
                "SELECT status, token_cost, duration, rel_uuid FROM system_api_log WHERE rel_uuid = ?1",
                params!["round-1"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(status, 1);
        assert_eq!(token_cost, 42);
        assert_eq!(duration, 123);
        assert_eq!(rel_uuid.as_deref(), Some("round-1"));

        let (status, content): (i8, Option<String>) = conn
            .query_row(
                "SELECT status, content FROM system_api_log WHERE rel_uuid = ?1",
                params!["round-2"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, 0);
        assert_eq!(content.as_deref(), Some("boom"));
    }

    /// 验证 `load_callable_agents` 排除自身且仅返回 status=1。
    #[test]
    fn load_callable_agents_excludes_self_and_disabled() {
        use crate::core::db::Model;
        use crate::db::models::agent_info::AgentInfo;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(AgentInfo::create_table_sql()).unwrap();
        conn.execute(
            "INSERT INTO agent_info (id, llm_id, name, status) VALUES (1, 1, 'Self', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_info (id, llm_id, name, status) VALUES (2, 1, 'Enabled', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_info (id, llm_id, name, status) VALUES (3, 1, 'Disabled', 0)",
            [],
        )
        .unwrap();

        let store = SessionStore::from_connection(conn);
        let names = store.load_callable_agents(1).unwrap();
        assert_eq!(names, vec!["Enabled".to_string()]);
    }

    /// 验证 `load_prompt_placeholders` 仅返回 status=1 的行，且携带 value/default_value。
    #[test]
    fn load_prompt_placeholders_filters_disabled() {
        use crate::core::db::Model;
        use crate::db::models::prompt_placeholder::PromptPlaceholder;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(PromptPlaceholder::create_table_sql())
            .unwrap();
        conn.execute(
            "INSERT INTO prompt_placeholder (prompt_id, key, value, default_value, status) \
             VALUES (100, 'City', 'Beijing', 'N/A', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompt_placeholder (prompt_id, key, value, default_value, status) \
             VALUES (100, 'Off', 'x', 'y', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prompt_placeholder (prompt_id, key, value, default_value, status) \
             VALUES (200, 'Other', 'z', NULL, 1)",
            [],
        )
        .unwrap();

        let store = SessionStore::from_connection(conn);
        let rows = store.load_prompt_placeholders(100).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "City");
        assert_eq!(rows[0].1.as_deref(), Some("Beijing"));
        assert_eq!(rows[0].2.as_deref(), Some("N/A"));
    }
}
