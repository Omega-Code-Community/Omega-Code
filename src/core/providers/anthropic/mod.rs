use crate::core::providers::basic::InterfaceFormat;

mod model;

pub struct Anthropic {
    base_url: String,
    anthropic_version: String,
    interface_format: InterfaceFormat, 
    pub x_api_key: String,
}

impl Default for Anthropic {
    fn default() -> Self {
        Anthropic {
            base_url: "https://api.anthropic.com".to_string(),
            anthropic_version: "2023-06-01".to_string(),
            interface_format: InterfaceFormat::Messages,
            x_api_key: "sk-ant-...".to_string()
        }
    }
}