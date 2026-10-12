use std::time::Duration;
use std::pin::Pin;

use futures_util::Stream;
use tokio::time::sleep;
use crate::core::transport::errors::TransportError;
use crate::core::transport::request::TransportRequest;
use crate::core::transport::response::TransportResponse;
use crate::core::transport::retry::RetryPolicy;
use crate::core::transport::sse::{SseStream, sse_stream_to_json};
use crate::core::client::base_client::BaseClient;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct HttpTransport {
    client: BaseClient,
    retry: RetryPolicy,
}

impl HttpTransport {
    pub fn new(
        client: BaseClient,
        retry: RetryPolicy,
    ) -> Self {
        Self { client, retry }
    }

    pub async fn execute(
        &self,
        req: TransportRequest,
    ) -> Result<TransportResponse, TransportError> {
        let mut retries = 0;

        loop {
            let request = self.build_request(req.clone())?;

            match self.client.execute(request).await {
                Ok(resp) => {
                    let status = resp.status();

                    if status.as_u16() == 429 {
                        if retries >= self.retry.max_retries {
                            return Err(TransportError::RateLimited);
                        }

                        retries += 1;

                        sleep(self.backoff(retries)).await;
                        continue;
                    }

                    if status.is_server_error() {
                        if retries >= self.retry.max_retries {
                            return Err(
                                TransportError::Server(status.to_string())
                            );
                        }

                        retries += 1;

                        sleep(self.backoff(retries)).await;
                        continue;
                    }

                    let body = resp.bytes().await?;

                    return Ok(TransportResponse {
                        status,
                        body,
                    });
                }

                Err(err) => {
                    if retries >= self.retry.max_retries {
                        return Err(TransportError::Http(err));
                    }

                    retries += 1;

                    sleep(self.backoff(retries)).await;
                }
            }
        }
    }

    fn build_request(
        &self,
        req: TransportRequest,
    ) -> Result<reqwest::Request, TransportError> {
        let mut builder = self.client
            .inner()
            .request(req.method, req.url);

        for (k, v) in req.headers {
            builder = builder.header(k, v);
        }

        if let Some(body) = req.body {
            builder = builder.body(body);
        }

        Ok(builder.build()?)
    }

    fn backoff(&self, retry: usize) -> Duration {
        self.retry.base_delay * retry as u32
    }

    pub async fn execute_stream<T>(
        &self,
        req: TransportRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = anyhow::Result<T>> + Send + 'static>>, TransportError>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let request = self.build_request(req)?;
        
        let response = self.client.execute(request).await?;
        
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(TransportError::Server(status.to_string()));
        }

        let sse_stream = SseStream::new(response);
        let json_stream = sse_stream_to_json(sse_stream).await;
        
        Ok(json_stream)
    }

    /// 执行请求并返回原始 SSE 事件流，由调用方自行解析 `data` 字段。
    /// 相比 [`HttpTransport::execute_stream`]，此方法不会将事件反序列化为 JSON，
    /// 因此可以处理诸如 OpenAI 的 `data: [DONE]` 这类非 JSON 结束标记。
    pub async fn execute_sse(
        &self,
        req: TransportRequest,
    ) -> Result<SseStream, TransportError> {
        let request = self.build_request(req)?;

        let response = self.client.execute(request).await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(TransportError::Server(status.to_string()));
        }

        Ok(SseStream::new(response))
    }
}