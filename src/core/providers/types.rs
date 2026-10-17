// core/types.rs

use std::pin::Pin;

use anyhow::Result;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

/// 流式补全的增量片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    /// 本次增量的文本内容（可能为空）。
    pub content: String,
    /// 若本片段是最后一个，则携带结束原因。
    pub finish_reason: Option<FinishReason>,
}

/// 流式补全返回的事件流类型别名。
pub type CompletionStream =
    Pin<Box<dyn Stream<Item = Result<StreamDelta>> + Send + 'static>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,

    pub messages: Vec<Message>,

    pub temperature: Option<f32>,

    pub max_tokens: Option<u32>,

    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,

    pub completion_tokens: Option<u32>,

    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,

    pub finish_reason: Option<FinishReason>,

    pub usage: Option<Usage>,
}