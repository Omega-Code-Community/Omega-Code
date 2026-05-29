use serde::Serialize;

#[derive(Serialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
pub struct GeminiContent {
    pub role: String,

    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Serialize)]
pub struct GeminiGenerationConfig {
    pub temperature: Option<f32>,

    pub max_output_tokens: Option<u32>,
}