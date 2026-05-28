use reqwest::{Client, Request, Response};
use crate::core::client::config::ClientConfig;

#[derive(Clone)]
pub struct BaseClient {
    inner: Client,
}

impl BaseClient {
    pub fn new(config: ClientConfig) -> anyhow::Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .user_agent(config.user_agent);

        if let Some(proxy) = config.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }

        let client = builder.build()?;

        Ok(Self {
            inner: client,
        })
    }

    pub async fn execute(
        &self,
        request: Request,
    ) -> Result<Response, reqwest::Error> {
        self.inner.execute(request).await
    }

    pub fn inner(&self) -> &Client {
        &self.inner
    }
}