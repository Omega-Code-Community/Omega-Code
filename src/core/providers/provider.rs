use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream;
use crate::core::client::AiClient;
use crate::core::providers::types::{
    CompletionRequest, CompletionResponse, CompletionStream, StreamDelta,
};

#[async_trait]
pub trait Provider: Send + Sync {
    fn client(&self) -> &AiClient;

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// 流式补全。默认实现回退到 [`Provider::complete`]，
    /// 将完整结果作为单个增量片段发出；支持原生流式的供应商可覆盖此方法。
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream> {
        let response = self.complete(request).await?;
        let delta = StreamDelta {
            content: response.content,
            finish_reason: response.finish_reason,
        };
        Ok(Box::pin(stream::once(async move { Ok(delta) })))
    }
}