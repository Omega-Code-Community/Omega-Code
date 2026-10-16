//! MCP 工具调度
//!
//! 依据《系统功能设计与数据库设计文档》四.10/四.11 与《后端开发手册》：Agent 通过
//! `agent_mcp_rel` 获得授权的 MCP 工具集合（`mcp_tool`），每个工具归属于某个
//! MCP Server（`mcp_provider`）。本模块负责把这些「数据库登记的远程工具」适配成本地
//! [`Tool`]，并在调用时经由 [`rmcp`] 客户端（Streamable HTTP 传输）路由到对应的
//! Python MCP Server 执行。
//!
//! 设计要点：
//! - **权限校验**：只有出现在授权表中的 `tool_code` 才能被路由，未授权调用直接拒绝。
//! - **路由**：以 `tool_code → provider_url` 建立映射，调用时按工具的归属地址转发。
//! - **适配**：[`McpTool`] 把远程工具包装为本地 [`Tool`]，其名称即 `tool_code`，
//!   参数 Schema 取自 `mcp_tool.parameters`（缺省为空 object）。
//!
//! [`Tool`]: crate::core::tools::Tool
//! [`rmcp`]: https://docs.rs/rmcp

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientInfo};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::ServiceExt;
use serde_json::{json, Map, Value};

use crate::core::tools::Tool;

/// 从数据库加载的一个已授权 MCP 工具规格。
#[derive(Debug, Clone)]
pub struct McpToolSpec {
    /// `mcp_tool.id`。
    pub tool_id: i32,
    /// 工具调用标识（`mcp_tool.tool_code`），也是本地 [`Tool::name`]。
    pub tool_code: String,
    /// 工具展示名（`mcp_tool.name`）。
    pub name: String,
    /// 工具描述（`mcp_tool.description`，可空）。
    pub description: Option<String>,
    /// 入参 JSON Schema（`mcp_tool.parameters`，可空）。
    pub parameters: Option<String>,
    /// 归属 MCP Server 地址（`mcp_provider.url`）。
    pub provider_url: String,
}

/// MCP 工具路由器：维护 `tool_code → provider_url` 的授权映射并转发调用。
///
/// 由 [`build_tools`] 在多个 [`McpTool`] 之间共享（`Arc`），保证「未登记即不可调用」
/// 的权限边界集中在一处。
#[derive(Debug, Default, Clone)]
pub struct McpToolRouter {
    routes: HashMap<String, String>,
}

impl McpToolRouter {
    /// 依据一组授权工具规格构建路由表。
    pub fn from_specs(specs: &[McpToolSpec]) -> Self {
        let mut routes = HashMap::new();
        for spec in specs {
            routes.insert(spec.tool_code.clone(), spec.provider_url.clone());
        }
        Self { routes }
    }

    /// 判断某个 `tool_code` 是否已授权（存在于路由表中）。
    pub fn is_authorized(&self, tool_code: &str) -> bool {
        self.routes.contains_key(tool_code)
    }

    /// 取得某个 `tool_code` 归属的 MCP Server 地址。
    pub fn provider_url(&self, tool_code: &str) -> Option<&str> {
        self.routes.get(tool_code).map(|s| s.as_str())
    }

    /// 已授权工具数量。
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// 路由调用一个远程 MCP 工具。
    ///
    /// 未授权的 `tool_code` 会被直接拒绝（权限校验），否则转发到其归属 Server。
    pub async fn call(&self, tool_code: &str, args: Map<String, Value>) -> Result<String> {
        let url = self
            .provider_url(tool_code)
            .with_context(|| format!("MCP tool '{}' is not authorized", tool_code))?;
        call_mcp_tool(url, tool_code, args).await
    }
}

/// 经由 Streamable HTTP 传输连接 MCP Server，调用单个工具并抽取文本结果。
///
/// 连接为一次性短连接：调用完成后主动 `cancel` 释放后台任务。若服务端返回
/// `is_error == true`，则以错误形式上抛（附服务端返回的文本）。
pub async fn call_mcp_tool(
    url: &str,
    tool_code: &str,
    args: Map<String, Value>,
) -> Result<String> {
    let transport = StreamableHttpClientTransport::from_uri(url.to_string());
    let client = ClientInfo::default()
        .serve(transport)
        .await
        .with_context(|| format!("failed to connect to MCP server at {}", url))?;

    let result = client
        .call_tool(CallToolRequestParams::new(tool_code.to_string()).with_arguments(args))
        .await
        .with_context(|| format!("MCP tool '{}' call failed", tool_code));

    // 无论成功与否都尽量释放连接（忽略 cancel 自身的错误）。
    let _ = client.cancel().await;

    let result = result?;
    if result.is_error == Some(true) {
        anyhow::bail!(
            "MCP tool '{}' returned error: {}",
            tool_code,
            extract_text(&result)
        );
    }
    Ok(extract_text(&result))
}

/// 从 [`CallToolResult`] 中抽取文本：优先拼接所有文本内容块，
/// 若无文本则回退到 `structured_content` 的 JSON 字符串。
fn extract_text(result: &CallToolResult) -> String {
    let mut text = String::new();
    for content in &result.content {
        if let Some(t) = content.as_text() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&t.text);
        }
    }
    if text.is_empty() {
        if let Some(structured) = &result.structured_content {
            return structured.to_string();
        }
    }
    text
}

/// 把一个远程 MCP 工具适配为本地 [`Tool`]。
///
/// 名称即 `tool_code`，参数 Schema 取自 `mcp_tool.parameters`，执行时经共享的
/// [`McpToolRouter`] 路由到归属 Server。
pub struct McpTool {
    code: String,
    description: String,
    parameters: Value,
    router: Arc<McpToolRouter>,
}

impl McpTool {
    /// 依据规格与共享路由器构造。
    pub fn new(spec: &McpToolSpec, router: Arc<McpToolRouter>) -> Self {
        let description = spec
            .description
            .clone()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| spec.name.clone());
        Self {
            code: spec.tool_code.clone(),
            description,
            parameters: parse_parameters(spec.parameters.as_deref()),
            router,
        }
    }
}

/// 解析 `mcp_tool.parameters`（JSON Schema 字符串）。
///
/// 缺省或非法时回退为一个空 object schema，保证提示词中始终有可用的参数描述。
fn parse_parameters(raw: Option<&str>) -> Value {
    let default = json!({ "type": "object", "properties": {} });
    match raw {
        Some(s) if !s.trim().is_empty() => serde_json::from_str::<Value>(s).unwrap_or(default),
        _ => default,
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.code
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let map = match args {
            Value::Object(m) => m,
            Value::Null => Map::new(),
            other => {
                let mut m = Map::new();
                m.insert("value".to_string(), other);
                m
            }
        };
        self.router.call(&self.code, map).await
    }
}

/// 依据一组已授权工具规格构建本地 [`Tool`] 集合（共享同一个 [`McpToolRouter`]）。
///
/// 返回的工具可直接注册进 [`ToolRegistry`](crate::core::tools::ToolRegistry)。
pub fn build_tools(specs: Vec<McpToolSpec>) -> Vec<Arc<dyn Tool>> {
    let router = Arc::new(McpToolRouter::from_specs(&specs));
    specs
        .iter()
        .map(|spec| Arc::new(McpTool::new(spec, router.clone())) as Arc<dyn Tool>)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(tool_code: &str, url: &str) -> McpToolSpec {
        McpToolSpec {
            tool_id: 1,
            tool_code: tool_code.to_string(),
            name: tool_code.to_string(),
            description: None,
            parameters: None,
            provider_url: url.to_string(),
        }
    }

    #[test]
    fn router_authorizes_only_registered_tools() {
        let router = McpToolRouter::from_specs(&[
            spec("add", "http://localhost:8000/mcp"),
            spec("echo", "http://localhost:8001/mcp"),
        ]);
        assert!(router.is_authorized("add"));
        assert!(router.is_authorized("echo"));
        assert!(!router.is_authorized("drop_table"));
        assert_eq!(router.provider_url("add"), Some("http://localhost:8000/mcp"));
        assert_eq!(router.provider_url("nope"), None);
        assert_eq!(router.len(), 2);
    }

    #[tokio::test]
    async fn router_rejects_unauthorized_call() {
        let router = McpToolRouter::from_specs(&[spec("add", "http://localhost:8000/mcp")]);
        let err = router.call("secret", Map::new()).await.unwrap_err();
        assert!(err.to_string().contains("not authorized"));
    }

    #[test]
    fn parse_parameters_falls_back_to_empty_object() {
        let fallback = json!({ "type": "object", "properties": {} });
        assert_eq!(parse_parameters(None), fallback);
        assert_eq!(parse_parameters(Some("")), fallback);
        assert_eq!(parse_parameters(Some("not json")), fallback);

        let schema = json!({ "type": "object", "properties": { "a": { "type": "integer" } } });
        assert_eq!(parse_parameters(Some(&schema.to_string())), schema);
    }

    #[test]
    fn build_tools_maps_specs_with_shared_router() {
        let tools = build_tools(vec![
            spec("add", "http://localhost:8000/mcp"),
            spec("echo", "http://localhost:8001/mcp"),
        ]);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "add");
        assert_eq!(tools[1].name(), "echo");
        // 描述缺省时回退到 name。
        assert_eq!(tools[0].description(), "add");
        // 参数缺省时为空 object schema。
        assert_eq!(tools[0].parameters()["type"], "object");
    }
}
