use anyhow::Result;
use async_trait::async_trait;
use crate::core::client::AiClient;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

#[async_trait]
pub trait Provider: Send + Sync {
    fn client(&self) -> &AiClient;
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
}