// ============================================================
// ai/call/mod.rs
// ============================================================

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================
// Role
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

// ============================================================
// Message
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,

    pub content: Vec<ContentPart>,
}

// ============================================================
// Content Part
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text {
        text: String,
    },

    #[serde(rename = "image")]
    Image {
        url: String,
    },

    #[serde(rename = "thinking")]
    Thinking {
        text: String,
    },

    #[serde(rename = "tool_call")]
    ToolCall {
        id: String,

        name: String,

        arguments: Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_call_id: String,

        content: String,
    },
}

// ============================================================
// Tool
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,

    pub description: Option<String>,

    pub parameters: Value,
}

// ============================================================
// Reasoning Config
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub enabled: bool,

    pub budget_tokens: Option<u32>,
}

// ============================================================
// Request
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,

    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(default)]
    pub stream: bool,

    #[serde(default)]
    pub tools: Vec<ToolDefinition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,

    #[serde(default)]
    pub metadata: Value,
}

// ============================================================
// Usage
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,

    pub completion_tokens: u32,

    pub total_tokens: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

// ============================================================
// Response
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub model: String,

    pub message: Message,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,

    #[serde(default)]
    pub metadata: Value,
}

// ============================================================
// Tool Call Event
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    pub id: String,

    pub name: String,

    pub arguments: Value,
}

// ============================================================
// Tool Result Event
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultEvent {
    pub tool_call_id: String,

    pub content: String,
}

// ============================================================
// Stream Event
// ============================================================

#[derive(Debug, Clone)]
pub enum LlmEvent {
    Text(String),

    Reasoning(String),

    ToolCall(ToolCallEvent),

    ToolResult(ToolResultEvent),

    Usage(TokenUsage),

    Metadata(Value),

    Binary(Bytes),

    Done,
}

// ============================================================
// Stream Type
// ============================================================

pub type LlmStream =
BoxStream<'static, Result<LlmEvent>>;

// ============================================================
// Provider Trait
// ============================================================

#[async_trait]
pub trait LlmProvider:
Send + Sync + 'static
{
    /// provider name
    fn name(&self) -> &'static str;

    /// supported models
    async fn models(&self) -> Result<Vec<String>>;

    /// supports reasoning
    fn supports_reasoning(&self) -> bool {
        false
    }

    /// supports tools
    fn supports_tools(&self) -> bool {
        false
    }

    /// supports stream
    fn supports_stream(&self) -> bool {
        true
    }

    /// normal call
    async fn call(
        &self,
        request: LlmRequest,
    ) -> Result<LlmResponse>;

    /// stream call
    async fn stream(
        &self,
        request: LlmRequest,
    ) -> Result<LlmStream>;
}

// ============================================================
// Provider Registry
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;

pub struct ProviderRegistry {
    providers:
        HashMap<String, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register<P>(
        mut self,
        provider: P,
    ) -> Self
    where
        P: LlmProvider,
    {
        self.providers.insert(
            provider.name().to_string(),
            Arc::new(provider),
        );

        self
    }

    pub fn get(
        &self,
        name: &str,
    ) -> Option<Arc<dyn LlmProvider>> {
        self.providers.get(name).cloned()
    }
}

// ============================================================
// Call Manager
// ============================================================

pub struct CallManager {
    registry: ProviderRegistry,
}

impl CallManager {
    pub fn new(
        registry: ProviderRegistry,
    ) -> Self {
        Self { registry }
    }

    pub async fn call(
        &self,
        provider: &str,
        request: LlmRequest,
    ) -> Result<LlmResponse> {
        let provider = self
            .registry
            .get(provider)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "provider not found: {}",
                    provider
                )
            })?;

        provider.call(request).await
    }

    pub async fn stream(
        &self,
        provider: &str,
        request: LlmRequest,
    ) -> Result<LlmStream> {
        let provider = self
            .registry
            .get(provider)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "provider not found: {}",
                    provider
                )
            })?;

        provider.stream(request).await
    }
}