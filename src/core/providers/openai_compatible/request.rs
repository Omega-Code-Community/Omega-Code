use serde::Serialize;

#[derive(Serialize)]
pub struct OpenAiRequest {
    pub model: String,

    pub messages: Vec<OpenAiMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    pub stream: bool,
}

#[derive(Serialize)]
pub struct OpenAiMessage {
    pub role: String,

    pub content: String,
}