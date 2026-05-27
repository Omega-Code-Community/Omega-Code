use crate::core::providers::basic::InterfaceFormat;

mod request;
mod model;

pub struct OpenAI {
    base_url: String,
    interface_format: InterfaceFormat,
    pub authorization: String,
}

impl Default for OpenAI {
    fn default() -> Self {
        OpenAI {
            base_url: "https://api.openai.com".to_string(),
            interface_format: InterfaceFormat::Responses,
            authorization: "sk-...".to_string()
        }
    }
}