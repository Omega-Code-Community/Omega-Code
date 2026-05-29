use crate::core::providers::config::{ProviderConfig, ProviderKind};
use crate::core::providers::provider::Provider;

mod types;
mod provider;
mod config;
mod openai_compatible;
mod claude_messages;
mod gemini;

pub fn create_provider(
    config: ProviderConfig,
) -> Box<dyn Provider> {

    match config.kind {

        ProviderKind::OpenAiCompatible => {
            Box::new(
                OpenAiCompatibleProvider::new(
                    config.api_key,
                    config.base_url,
                )
            )
        }

        ProviderKind::Claude => {
            Box::new(
                ClaudeProvider::new(
                    config.api_key,
                )
            )
        }

        ProviderKind::Gemini => {
            Box::new(
                GeminiProvider::new(
                    config.api_key,
                )
            )
        }
    }
}