use bytes::Bytes;
use reqwest::Method;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TransportRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Bytes>,
}