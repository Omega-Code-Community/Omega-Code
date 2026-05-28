use std::time::Duration;

use tokio::time::sleep;
use crate::core::transport::errors::TransportError;
use crate::core::transport::request::TransportRequest;
use crate::core::transport::response::TransportResponse;
use crate::core::transport::retry::RetryPolicy;
use crate::core::client::base_client::BaseClient;
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
}