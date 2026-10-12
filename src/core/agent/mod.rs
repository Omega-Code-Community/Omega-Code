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

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt};
use log::warn;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::core::config::AppConfig;
use crate::core::providers::{
    create_provider, CompletionRequest, Message, Provider, Role,
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
                     VALUES (?1, '', ?2, ?3, 5000, 1)",
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
                     VALUES (?1, ?2, '', ?3, 0, 0, 0, 0, 1)",
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
            "INSERT INTO chat_session (chat_uuid, title, llm_id, status) VALUES (?1, ?2, ?3, 1)",
            params![uuid, title, llm_id],
        )?;
        Ok(())
    }

    /// 保存一条消息。
    pub fn save_message(
        &self,
        session_uuid: &str,
        role: &str,
        content: &str,
        model: Option<&str>,
    ) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO chat_message (session_id, role, content, model, status) VALUES (?1, ?2, ?3, ?4, 1)",
            params![session_uuid, role, content, model],
        )?;
        Ok(())
    }

    /// 加载某个会话的历史消息（按时间升序）。
    pub fn load_history(&self, session_uuid: &str) -> Result<Vec<Message>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT role, content FROM chat_message WHERE session_id = ?1 ORDER BY id ASC",
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
        .with_provider_meta(config.provider_kind.as_str(), config.base_url.clone(), key))
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
        self.persist("user", user_input, None);

        let mut iteration = 0;
        loop {
            iteration += 1;

            let mut messages = Vec::with_capacity(self.history.len() + 1);
            messages.push(Message {
                role: Role::System,
                content: self.build_system_prompt(),
            });
            messages.extend(self.history.iter().cloned());

            let request = CompletionRequest {
                model: self.model.clone(),
                messages,
                temperature: None,
                max_tokens: None,
                stream: true,
            };

            let mut stream = match self.provider.complete_stream(request).await {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(AgentEvent::Error(e.to_string()));
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
                        let _ = tx.send(AgentEvent::Error(e.to_string()));
                        return Err(e);
                    }
                }
            }

            self.history.push(Message {
                role: Role::Assistant,
                content: accumulated.clone(),
            });
            self.persist("assistant", &accumulated, Some(self.model.as_str()));

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
                    self.persist("tool", &tool_message, None);
                    continue;
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

    /// 组合系统提示词与工具说明。
    fn build_system_prompt(&self) -> String {
        let tools = self.tools.definitions_prompt();
        if tools.is_empty() {
            self.system_prompt.clone()
        } else {
            format!("{}\n\n{}", self.system_prompt, tools)
        }
    }

    /// 持久化一条消息（若已开启持久化）。
    fn persist(&self, role: &str, content: &str, model: Option<&str>) {
        if let Some(store) = &self.store {
            if let Err(e) = store.save_message(&self.session_uuid, role, content, model) {
                warn!("Failed to persist '{}' message: {}", role, e);
            }
        }
    }
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
}
