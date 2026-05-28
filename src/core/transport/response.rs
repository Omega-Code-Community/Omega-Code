use bytes::Bytes;
use reqwest::StatusCode;

pub struct TransportResponse {
    pub status: StatusCode,
    pub body: Bytes,
}