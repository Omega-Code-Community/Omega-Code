use serde::{Deserialize, Serialize};

/// GET /models 请求（无参数，空结构体）
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ListModelsRequest {}

/// 模型列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

/// 单个模型信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub owned_by: String,
}