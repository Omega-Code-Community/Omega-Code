use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAiResponse {
    pub choices: Vec<OpenAiChoice>,

    pub usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
pub struct OpenAiChoice {
    pub message: OpenAiAssistantMessage,

    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenAiAssistantMessage {
    pub content: String,
}

#[derive(Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: Option<u32>,

    pub completion_tokens: Option<u32>,

    pub total_tokens: Option<u32>,
}