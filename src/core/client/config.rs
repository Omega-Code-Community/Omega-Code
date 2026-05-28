use std::time::Duration;

#[derive(Clone)]
pub struct ClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub proxy: Option<String>,
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(10),
            proxy: None,
            user_agent: "agent-runtime/0.1.0".into(),
        }
    }
}