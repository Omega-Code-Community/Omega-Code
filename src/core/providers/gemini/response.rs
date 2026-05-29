use serde::Deserialize;

#[derive(Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiContentResponse,

    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct GeminiContentResponse {
    pub parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
pub struct GeminiPartResponse {
    pub text: String,
}