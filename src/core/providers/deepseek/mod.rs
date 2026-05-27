use crate::core::providers::basic::InterfaceFormat;

mod model;

pub struct Deepseek {
    base_url: String,
    interface_format: InterfaceFormat,
    pub authorization: String,
}

impl Default for Deepseek {
    fn default() -> Self {
        Deepseek {
            base_url: "https://api.deepseek.com".to_string(),
            interface_format: InterfaceFormat::Completions,
            authorization: "sk-...".to_string()
        }
    }
}