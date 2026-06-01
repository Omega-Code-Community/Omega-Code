use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::Response;
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

#[derive(Debug)]
pub enum SseError {
    Http(reqwest::Error),
    InvalidData(String),
    Deserialization(String),
}

impl std::fmt::Display for SseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SseError::Http(e) => write!(f, "HTTP error: {}", e),
            SseError::InvalidData(s) => write!(f, "Invalid SSE data: {}", s),
            SseError::Deserialization(s) => write!(f, "Deserialization error: {}", s),
        }
    }
}

impl std::error::Error for SseError {}

pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: Vec<u8>,
}

impl SseStream {
    pub fn new(response: Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: Vec::new(),
        }
    }

    fn parse_chunk(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        
        let mut events = Vec::new();
        let mut start = 0;
        
        while let Some(end) = self.buffer[start..].windows(2).position(|w| w == b"\r\n" || w == b"\n\n") {
            let end_pos = start + end + 2;
            let line = &self.buffer[start..end_pos];
            
            let event = self.parse_line(line);
            if let Some(e) = event {
                events.push(e);
            }
            
            start = end_pos;
        }
        
        if start > 0 {
            self.buffer = self.buffer[start..].to_vec();
        }
        
        events
    }

    fn parse_line(&self, line: &[u8]) -> Option<SseEvent> {
        let line = String::from_utf8_lossy(line).trim().to_string();
        
        if line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.is_empty() {
            return None;
        }

        let field = parts[0].trim();
        let value = parts.get(1).map(|s| s.trim_start_matches(' ')).unwrap_or("");

        let mut event = SseEvent {
            event: None,
            data: String::new(),
            id: None,
        };

        match field {
            "event" => event.event = Some(value.to_string()),
            "data" => event.data = value.to_string(),
            "id" => event.id = Some(value.to_string()),
            _ => {}
        }

        Some(event)
    }
}

impl Stream for SseStream {
    type Item = Result<SseEvent, SseError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let mut events = self.parse_chunk(&chunk);
                    if !events.is_empty() {
                        return Poll::Ready(Some(Ok(events.remove(0))));
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(SseError::Http(e))));
                }
                Poll::Ready(None) => {
                    if !self.buffer.is_empty() {
                        let mut events = self.parse_chunk(&[]);
                        if !events.is_empty() {
                            return Poll::Ready(Some(Ok(events.remove(0))));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

pub async fn parse_sse_chunk(chunk: &[u8]) -> Result<SseEvent> {
    let line = String::from_utf8_lossy(chunk).trim().to_string();
    
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.is_empty() {
        return Err(anyhow!("Empty SSE line"));
    }

    let field = parts[0].trim();
    let value = parts.get(1).map(|s| s.trim_start_matches(' ')).unwrap_or("");

    Ok(SseEvent {
        event: if field == "event" { Some(value.to_string()) } else { None },
        data: if field == "data" { value.to_string() } else { String::new() },
        id: if field == "id" { Some(value.to_string()) } else { None },
    })
}

pub async fn parse_sse_raw(raw: &str) -> Result<Vec<SseEvent>> {
    let mut events = Vec::new();
    let mut current_event = SseEvent {
        event: None,
        data: String::new(),
        id: None,
    };

    for line in raw.lines() {
        let line = line.trim();
        
        if line.is_empty() {
            if !current_event.data.is_empty() {
                events.push(current_event.clone());
            }
            current_event = SseEvent {
                event: None,
                data: String::new(),
                id: None,
            };
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 {
            continue;
        }

        let field = parts[0].trim();
        let value = parts[1].trim_start_matches(' ');

        match field {
            "event" => current_event.event = Some(value.to_string()),
            "data" => current_event.data += value,
            "id" => current_event.id = Some(value.to_string()),
            _ => {}
        }
    }

    if !current_event.data.is_empty() {
        events.push(current_event);
    }

    Ok(events)
}

pub async fn sse_stream_to_json<T>(
    stream: SseStream,
) -> Pin<Box<dyn Stream<Item = Result<T>> + Send + 'static>>
where
    T: DeserializeOwned + Send + 'static,
{
    let mapped = stream.map(move |event_result| {
        match event_result {
            Ok(event) => {
                if event.data.is_empty() {
                    return Err(anyhow!("Empty SSE data"));
                }
                serde_json::from_str::<T>(&event.data)
                    .map_err(|e| anyhow!("Failed to deserialize SSE event: {}", e))
            }
            Err(e) => Err(anyhow!("SSE error: {}", e)),
        }
    });

    Box::pin(mapped)
}