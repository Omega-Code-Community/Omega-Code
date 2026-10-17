//! 应用配置
//!
//! 负责从环境变量（`.env` 文件由 `dotenv` 加载）中读取运行 Agent 所需的配置，
//! 并转换为 [`ProviderConfig`] 交给供应商工厂使用。
//!
//! 支持的环境变量：
//! | 变量 | 说明 | 默认值 |
//! |------|------|--------|
//! | `OMEGA_PROVIDER` | 供应商类型：compatible/claude/gemini/deepseek/ollama | compatible |
//! | `OMEGA_API_KEY` (回退 `API_KEY`) | API 密钥 | 空 |
//! | `OMEGA_BASE_URL` | API Base URL | https://api.openai.com/v1 |
//! | `OMEGA_MODEL` | 模型名称 | gpt-4o-mini |
//! | `OMEGA_TEMPERATURE` | 采样温度 | 0.7 |
//! | `OMEGA_MAX_TOKENS` | 单次最大输出 token | 不限制 |
//! | `OMEGA_SYSTEM_PROMPT` | 系统提示词 | 内置默认 |
//! | `OMEGA_WORKING_DIR` | 工作目录 | 当前目录 |

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::core::providers::{ProviderConfig, ProviderKind};

/// 默认系统提示词。
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are Omega Code, an AI coding assistant running in \
the user's terminal. Be concise, accurate and helpful. When you need to inspect or modify the \
project, use the available tools instead of guessing.";

/// 应用配置。
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub provider_kind: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: String,
    pub working_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider_kind: ProviderKind::Compatible,
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            temperature: Some(0.7),
            max_tokens: None,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl AppConfig {
    /// 从环境变量加载配置。会先尝试加载 `.env` 文件（若存在）。
    pub fn from_env() -> Result<Self> {
        // 忽略 .env 不存在的错误。
        let _ = dotenv::dotenv();

        let mut config = AppConfig::default();

        if let Ok(kind) = std::env::var("OMEGA_PROVIDER") {
            config.provider_kind = parse_provider_kind(&kind)
                .with_context(|| format!("Invalid OMEGA_PROVIDER: {}", kind))?;
        }

        if let Ok(key) = std::env::var("OMEGA_API_KEY").or_else(|_| std::env::var("API_KEY")) {
            config.api_key = key;
        }

        if let Ok(url) = std::env::var("OMEGA_BASE_URL") {
            config.base_url = url;
        }

        if let Ok(model) = std::env::var("OMEGA_MODEL") {
            config.model = model;
        }

        if let Ok(temp) = std::env::var("OMEGA_TEMPERATURE") {
            config.temperature = temp.parse::<f32>().ok();
        }

        if let Ok(max) = std::env::var("OMEGA_MAX_TOKENS") {
            config.max_tokens = max.parse::<u32>().ok();
        }

        if let Ok(prompt) = std::env::var("OMEGA_SYSTEM_PROMPT") {
            if !prompt.trim().is_empty() {
                config.system_prompt = prompt;
            }
        }

        if let Ok(dir) = std::env::var("OMEGA_WORKING_DIR") {
            config.working_dir = PathBuf::from(dir);
        }

        Ok(config)
    }

    /// 转换为供应商配置。
    pub fn provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            kind: self.provider_kind.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
    }
}

/// 将字符串解析为 [`ProviderKind`]。
pub fn parse_provider_kind(value: &str) -> Result<ProviderKind> {
    match value.trim().to_lowercase().as_str() {
        "compatible" | "openai" => Ok(ProviderKind::Compatible),
        "claude" | "anthropic" => Ok(ProviderKind::Claude),
        "gemini" | "google" => Ok(ProviderKind::Gemini),
        "deepseek" => Ok(ProviderKind::Deepseek),
        "ollama" => Ok(ProviderKind::Ollama),
        other => anyhow::bail!(
            "unknown provider kind '{}', expected one of: compatible, claude, gemini, deepseek, ollama",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kinds() {
        assert!(matches!(parse_provider_kind("Claude").unwrap(), ProviderKind::Claude));
        assert!(matches!(parse_provider_kind("deepseek").unwrap(), ProviderKind::Deepseek));
        assert!(parse_provider_kind("unknown").is_err());
    }
}
