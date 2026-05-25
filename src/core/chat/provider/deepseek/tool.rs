use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::enums::ToolType;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: ToolType,
    pub function: FunctionObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionObject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: ToolType,
    pub function: ToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Strategy(String),
    Tool(ToolChoiceObject),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChoiceObject {
    pub r#type: ToolType,
    pub function: ToolChoiceFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}