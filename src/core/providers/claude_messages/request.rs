use serde::Serialize;

#[derive(Serialize)]
pub struct ClaudeRequest {
    pub model: String,

    pub max_tokens: u32,

    pub messages: Vec<ClaudeMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    pub stream: bool,
}

#[derive(Serialize)]
pub struct ClaudeMessage {
    pub role: String,

    pub content: String,
}