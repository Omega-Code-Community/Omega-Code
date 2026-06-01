use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::Stream;
use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

use crate::core::client::base_client::BaseClient;
use crate::core::client::config::ClientConfig;
use crate::core::transport::http_transoprt::HttpTransport;
use crate::core::transport::retry::RetryPolicy;
use crate::core::transport::TransportRequest;

#[derive(Clone)]
pub struct AiClient {
    transport: HttpTransport,
}

impl AiClient {
    pub fn new(config: ClientConfig, retry: RetryPolicy) -> Result<Self> {
        let base_client = BaseClient::new(config)?;
        let transport = HttpTransport::new(base_client, retry);
        
        Ok(Self { transport })
    }

    pub async fn post_json<T>(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: &impl Serialize,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| anyhow!("Failed to serialize request body: {}", e))?;

        let request = TransportRequest {
            method: Method::POST,
            url: url.to_string(),
            headers,
            body: Some(Bytes::from(body_bytes)),
        };

        let response = self.transport.execute(request).await?;
        
        serde_json::from_slice(&response.body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    pub async fn get_json<T>(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        query: &impl Serialize,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let query_string = serde_urlencoded::to_string(query)
            .map_err(|e| anyhow!("Failed to serialize query: {}", e))?;
        
        let url_with_query = if query_string.is_empty() {
            url.to_string()
        } else {
            format!("{}?{}", url, query_string)
        };

        let request = TransportRequest {
            method: Method::GET,
            url: url_with_query,
            headers,
            body: None,
        };

        let response = self.transport.execute(request).await?;
        
        serde_json::from_slice(&response.body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    pub async fn post_stream<T>(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: &impl Serialize,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| anyhow!("Failed to serialize request body: {}", e))?;

        let request = TransportRequest {
            method: Method::POST,
            url: url.to_string(),
            headers,
            body: Some(Bytes::from(body_bytes)),
        };

        self.transport.execute_stream(request).await.map_err(|e| anyhow!(e))
    }

    pub fn transport(&self) -> &HttpTransport {
        &self.transport
    }
}