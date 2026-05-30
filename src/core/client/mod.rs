mod config;
pub mod base_client;
pub mod ai_client;

pub use config::ClientConfig;
pub use base_client::BaseClient;
pub use ai_client::AiClient;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::stream::Stream;
use rand::{Rng, RngExt};
use reqwest::{
    multipart,
    Client,
    Method,
    Request,
    RequestBuilder,
    Response,
    StatusCode,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;
use log::{info, warn};
use tokio::fs;

// ============================================================
// Retry Config
// ============================================================

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,

    pub retry_interval: u64,

    pub use_exponential_backoff: bool,

    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval: 1000,
            use_exponential_backoff: true,
            backoff_factor: 2.0,
        }
    }
}

// ============================================================
// Request Context
// ============================================================

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: Method,
    pub url: String,
}

// ============================================================
// Request Result
// ============================================================

pub enum RequestResult {
    Response(Response),
    Error(reqwest::Error),
}

// ============================================================
// Retry Predicate
// ============================================================

pub type RetryPredicate =
Box<
    dyn Fn(&RequestContext, &RequestResult) -> bool
    + Send
    + Sync
    + 'static,
>;

// ============================================================
// Request Factory
// ============================================================

type BoxFuture<T> =
Pin<Box<dyn Future<Output = T> + Send>>;

pub type RequestFactory =
Box<
    dyn Fn() -> BoxFuture<Result<(RequestContext, Request)>>
    + Send
    + Sync,
>;

// ============================================================
// Retry Middleware
// ============================================================

pub struct RetryMiddleware {
    pub config: RetryConfig,

    pub predicate: Option<RetryPredicate>,
}

impl Default for RetryMiddleware {
    fn default() -> Self {
        Self {
            config: RetryConfig::default(),
            predicate: None,
        }
    }
}

impl RetryMiddleware {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            predicate: None,
        }
    }

    pub fn with_retry_predicate<F>(
        mut self,
        f: F,
    ) -> Self
    where
        F: Fn(&RequestContext, &RequestResult) -> bool
        + Send
        + Sync
        + 'static,
    {
        self.predicate = Some(Box::new(f));

        self
    }

    // ========================================================
    // Full Jitter Backoff
    // ========================================================

    fn calculate_wait_time(
        &self,
        attempts: u32,
    ) -> Duration {
        let base = self.config.retry_interval as f64;

        let max_wait = if self
            .config
            .use_exponential_backoff
        {
            base
                * self
                .config
                .backoff_factor
                .powi(attempts as i32)
        } else {
            base
        };

        let mut rng = rand::rng();

        let wait_ms =
            rng.random_range(0.0..=max_wait);

        Duration::from_millis(wait_ms as u64)
    }

    // ========================================================
    // Retry-After
    // ========================================================

    fn retry_after(
        response: &Response,
    ) -> Option<Duration> {
        let header = response
            .headers()
            .get("retry-after")?;

        let value = header.to_str().ok()?;

        let secs = value.parse::<u64>().ok()?;

        Some(Duration::from_secs(secs))
    }

    // ========================================================
    // Default Retry Rule
    // ========================================================

    fn should_retry(
        &self,
        ctx: &RequestContext,
        result: &RequestResult,
    ) -> bool {
        if let Some(predicate) = &self.predicate {
            return predicate(ctx, result);
        }

        // 默认只 retry 幂等请求
        let retryable_method = matches!(
            ctx.method,
            Method::GET
                | Method::HEAD
                | Method::PUT
                | Method::DELETE
        );

        if !retryable_method {
            return false;
        }

        match result {
            RequestResult::Response(resp) => {
                resp.status().is_server_error()
                    || resp.status()
                    == StatusCode::TOO_MANY_REQUESTS
            }

            RequestResult::Error(err) => {
                err.is_timeout()
                    || err.is_connect()
                    || err
                    .status()
                    .map(|s| s.is_server_error())
                    .unwrap_or(false)
            }
        }
    }

    // ========================================================
    // Execute
    // ========================================================

    pub async fn execute(
        &self,
        client: &Client,
        factory: RequestFactory,
    ) -> Result<Response> {
        let mut attempt = 0;

        loop {
            let (ctx, request) = factory().await?;

            info!(
                "sending request: method={}, url={}, attempt={}",
                ctx.method,
                ctx.url,
                attempt
            );

            let result =
                match client.execute(request).await {
                    Ok(resp) => {
                        RequestResult::Response(resp)
                    }

                    Err(err) => {
                        RequestResult::Error(err)
                    }
                };

            let should_retry =
                self.should_retry(&ctx, &result);

            if !should_retry
                || attempt >= self.config.max_retries
            {
                return match result {
                    RequestResult::Response(resp) => {
                        Ok(resp)
                    }

                    RequestResult::Error(err) => {
                        Err(anyhow!(err))
                    }
                };
            }

            attempt += 1;

            let wait = match &result {
                RequestResult::Response(resp) => {
                    Self::retry_after(resp)
                        .unwrap_or_else(|| {
                            self.calculate_wait_time(
                                attempt,
                            )
                        })
                }

                _ => self.calculate_wait_time(attempt),
            };

            warn!(
                "retrying request: method={}, url={}, attempt={}, wait_ms={}",
                ctx.method,
                ctx.url,
                attempt,
                wait.as_millis()
            );

            tokio::time::sleep(wait).await;
        }
    }
}

// ============================================================
// Http Client
// ============================================================

pub struct HttpClient {
    client: Client,

    retry_middleware: RetryMiddleware,
}

impl HttpClient {
    pub fn new(
        retry_config: Option<RetryConfig>,
    ) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(
                10,
            ))
            .timeout(Duration::from_secs(300))
            .pool_idle_timeout(Duration::from_secs(
                90,
            ))
            .tcp_keepalive(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,

            retry_middleware: retry_config
                .map(RetryMiddleware::new)
                .unwrap_or_default(),
        })
    }

    // ========================================================
    // Core Send
    // ========================================================

    pub async fn send(
        &self,
        builder: RequestBuilder,
    ) -> Result<Response> {
        let builder = builder
            .try_clone()
            .ok_or_else(|| {
                anyhow!(
                    "request builder cannot be cloned"
                )
            })?;

        let factory: RequestFactory =
            Box::new(move || {
                let builder = builder
                    .try_clone()
                    .ok_or_else(|| {
                        anyhow!(
                            "request builder clone failed"
                        )
                    });

                Box::pin(async move {
                    let builder = builder?;

                    let request = builder.build()?;

                    let ctx = RequestContext {
                        method: request.method().clone(),

                        url: request.url().to_string(),
                    };

                    Ok((ctx, request))
                })
            });

        self.retry_middleware
            .execute(&self.client, factory)
            .await
    }

    // ========================================================
    // GET JSON
    // ========================================================

    pub async fn get_json<T>(
        &self,
        url: &str,
        query: &impl Serialize,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .send(self.client.get(url).query(query))
            .await?;

        Ok(response.json().await?)
    }

    // ========================================================
    // POST JSON
    // ========================================================

    pub async fn post_json<T>(
        &self,
        url: &str,
        body: &impl Serialize,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .send(self.client.post(url).json(body))
            .await?;

        Ok(response.json().await?)
    }

    // ========================================================
    // GET TEXT
    // ========================================================

    pub async fn get_text(
        &self,
        url: &str,
    ) -> Result<String> {
        let response = self
            .send(self.client.get(url))
            .await?;

        Ok(response.text().await?)
    }

    // ========================================================
    // GET BYTES
    // ========================================================

    pub async fn get_bytes(
        &self,
        url: &str,
    ) -> Result<Bytes> {
        let response = self
            .send(self.client.get(url))
            .await?;

        Ok(response.bytes().await?)
    }

    // ========================================================
    // SSE / STREAM
    // ========================================================

    pub async fn get_stream(
        &self,
        url: &str,
    ) -> Result<
        impl Stream<
            Item = Result<Bytes, reqwest::Error>,
        >,
    > {
        let response = self
            .send(self.client.get(url))
            .await?;

        Ok(response.bytes_stream())
    }

    // ========================================================
    // Multipart Upload
    // ========================================================

    pub async fn upload_file<T>(
        &self,
        url: &str,
        field_name: &str,
        file_path: impl Into<PathBuf>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let path = file_path.into();

        let client = self.client.clone();

        let url = url.to_string();

        let field_name = field_name.to_string();

        let factory: RequestFactory =
            Box::new(move || {
                let client = client.clone();

                let path = path.clone();

                let url = url.clone();

                let field_name =
                    field_name.clone();

                Box::pin(async move {
                    let bytes =
                        fs::read(&path).await?;

                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let part =
                        multipart::Part::bytes(
                            bytes,
                        )
                            .file_name(filename);

                    let form =
                        multipart::Form::new()
                            .part(
                                field_name,
                                part,
                            );

                    let request = client
                        .post(url)
                        .multipart(form)
                        .build()?;

                    let ctx = RequestContext {
                        method: request
                            .method()
                            .clone(),

                        url: request
                            .url()
                            .to_string(),
                    };

                    Ok((ctx, request))
                })
            });

        let response = self
            .retry_middleware
            .execute(&self.client, factory)
            .await?;

        Ok(response.json().await?)
    }

    // ========================================================
    // Stream Upload
    // ========================================================

    pub async fn upload_stream<T, S>(
        &self,
        url: &str,
        stream_factory: impl Fn() -> S
        + Send
        + Sync
        + 'static,
    ) -> Result<T>
    where
        T: DeserializeOwned,

        S: Stream<
            Item = Result<
                Bytes,
                std::io::Error,
            >,
        > + Send
        + 'static,
    {
        let client = self.client.clone();

        let url = url.to_string();

        let factory: RequestFactory =
            Box::new(move || {
                let client = client.clone();

                let url = url.clone();

                let stream = stream_factory();

                Box::pin(async move {
                    let body =
                        reqwest::Body::wrap_stream(
                            stream,
                        );

                    let request = client
                        .post(url)
                        .body(body)
                        .build()?;

                    let ctx = RequestContext {
                        method: request
                            .method()
                            .clone(),

                        url: request
                            .url()
                            .to_string(),
                    };

                    Ok((ctx, request))
                })
            });

        let response = self
            .retry_middleware
            .execute(&self.client, factory)
            .await?;

        Ok(response.json().await?)
    }

    // ========================================================
    // Retry Predicate
    // ========================================================

    pub fn with_retry_predicate<F>(
        mut self,
        predicate: F,
    ) -> Self
    where
        F: Fn(
            &RequestContext,
            &RequestResult,
        ) -> bool
        + Send
        + Sync
        + 'static,
    {
        self.retry_middleware = self
            .retry_middleware
            .with_retry_predicate(predicate);

        self
    }

    // ========================================================
    // Inner Client
    // ========================================================

    pub fn inner(&self) -> &Client {
        &self.client
    }
}