use crate::core::providers::config::{ProviderConfig, ProviderKind};
use crate::core::providers::provider::Provider;
pub use types::{CompletionRequest, CompletionResponse, Message, Role, FinishReason, Usage};

mod types;
mod provider;
mod config;
mod openai_compatible;
mod claude_messages;
mod gemini;
mod ollama;

use openai_compatible::OpenAiCompatibleProvider;
use claude_messages::ClaudeProvider;
use gemini::GeminiProvider;
use ollama::OllamaProvider;

pub fn create_provider(config: ProviderConfig) -> anyhow::Result<Box<dyn Provider>> {
    match config.kind {
        ProviderKind::Compatible => {
            let provider = OpenAiCompatibleProvider::new(config.api_key, config.base_url)?;
            Ok(Box::new(provider))
        }
        ProviderKind::Claude => {
            let provider = ClaudeProvider::new(config.api_key)?;
            Ok(Box::new(provider))
        }
        ProviderKind::Gemini => {
            let provider = GeminiProvider::new(config.api_key, config.model)?;
            Ok(Box::new(provider))
        }
        ProviderKind::Deepseek => {
            let provider = OpenAiCompatibleProvider::new(config.api_key, config.base_url)?;
            Ok(Box::new(provider))
        }
        ProviderKind::Ollama => {
            let provider = OllamaProvider::new(config.base_url)?;
            Ok(Box::new(provider))
        }
    }
}

