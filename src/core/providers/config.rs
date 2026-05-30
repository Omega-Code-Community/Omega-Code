// providers/config.rs

#[derive(Debug, Clone)]
pub enum ProviderKind {
    Compatible,
    Claude,
    Gemini,
    Deepseek,
    Ollama,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}