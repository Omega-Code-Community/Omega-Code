use serde::Deserialize;

#[derive(Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ClaudeContent>,

    pub stop_reason: Option<String>,

    pub usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
pub struct ClaudeContent {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: Option<u32>,

    pub output_tokens: Option<u32>,
}