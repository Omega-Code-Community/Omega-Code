// providers/config.rs

#[derive(Debug, Clone)]
pub enum ProviderKind {
    OpenAiCompatible,
    Claude,
    Gemini,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,

    pub api_key: String,

    pub base_url: String,

    pub model: String,
}