// core/provider.rs

use anyhow::Result;
use async_trait::async_trait;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse>;
}