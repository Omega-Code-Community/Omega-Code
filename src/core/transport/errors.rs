use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("request timeout")]
    Timeout,

    #[error("rate limited")]
    RateLimited,

    #[error("server error: {0}")]
    Server(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),
}