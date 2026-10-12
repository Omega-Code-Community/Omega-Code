// providers/config.rs

#[derive(Debug, Clone)]
pub enum ProviderKind {
    Compatible,
    Claude,
    Gemini,
    Deepseek,
    Ollama,
}

impl ProviderKind {
    /// 稳定的小写标识，用作供应商名称（与 `parse_provider_kind` 互逆）。
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Compatible => "compatible",
            ProviderKind::Claude => "claude",
            ProviderKind::Gemini => "gemini",
            ProviderKind::Deepseek => "deepseek",
            ProviderKind::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}