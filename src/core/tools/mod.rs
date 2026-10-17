//! 工具系统
//!
//! 定义 Agent 可调用的工具抽象 [`Tool`]、工具注册表 [`ToolRegistry`]，
//! 以及若干内置工具（读取文件、写入文件、列出目录）。
//!
//! 由于当前各供应商的请求体尚未内置原生 function-calling 字段，Agent 采用
//! 基于提示词的 ReAct 方式调用工具：工具定义会被注入系统提示词，模型以约定的
//! JSON 代码块表达调用意图，Agent 解析后执行并把结果回灌到对话中。
//! [`ToolRegistry::definitions_prompt`] 负责生成这段工具说明。

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::runtime::Runtime;

/// 工具抽象。所有工具都需线程安全，以便在异步任务中共享。
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一名称（供模型调用时引用）。
    fn name(&self) -> &str;

    /// 工具功能描述。
    fn description(&self) -> &str;

    /// 参数的 JSON Schema 描述。
    fn parameters(&self) -> Value;

    /// 执行工具，返回文本结果（会回灌给模型）。
    async fn execute(&self, args: Value) -> Result<String>;
}

/// 工具注册表，管理一组可用工具。
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// 注册一个工具。
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    /// 按名称查找工具。
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    /// 已注册工具数量。
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// 执行指定工具。
    pub async fn execute(&self, name: &str, args: Value) -> Result<String> {
        match self.get(name) {
            Some(tool) => tool.execute(args).await,
            None => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    /// 生成注入系统提示词的工具说明文本。
    pub fn definitions_prompt(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut prompt = String::from(
            "# Available Tools\n\n\
             You can use the following tools. To call a tool, respond with ONLY a fenced \
             code block tagged `tool` containing a JSON object with `name` and `arguments`, e.g.:\n\n\
             ```tool\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"src/main.rs\"}}\n```\n\n\
             Wait for the tool result before continuing. Do not fabricate tool outputs.\n\n",
        );

        for tool in &self.tools {
            prompt.push_str(&format!(
                "- `{}`: {}\n  parameters: {}\n",
                tool.name(),
                tool.description(),
                tool.parameters()
            ));
        }

        prompt
    }

    /// 创建带有默认内置工具的注册表。
    pub fn with_defaults(runtime: Arc<Runtime>) -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(ReadFileTool::new(runtime.clone())));
        registry.register(Arc::new(WriteFileTool::new(runtime.clone())));
        registry.register(Arc::new(ListDirTool::new(runtime)));
        registry
    }
}

// ============================================================
// 内置工具：read_file
// ============================================================

/// 读取文本文件内容。
pub struct ReadFileTool {
    runtime: Arc<Runtime>,
}

impl ReadFileTool {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the text content of a file within the project. Args: {\"path\": string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to the project root" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .context("read_file requires a 'path' string argument")?;

        let resolved = self.runtime.resolve_within(path)?;
        let content = tokio::fs::read_to_string(&resolved)
            .await
            .with_context(|| format!("Failed to read file '{}'", resolved.display()))?;

        Ok(content)
    }
}

// ============================================================
// 内置工具：write_file
// ============================================================

/// 写入（覆盖）文本文件，自动创建父目录。
pub struct WriteFileTool {
    runtime: Arc<Runtime>,
}

impl WriteFileTool {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write text content to a file within the project, creating parent directories as needed. \
         Args: {\"path\": string, \"content\": string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path relative to the project root" },
                "content": { "type": "string", "description": "Full text content to write" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .context("write_file requires a 'path' string argument")?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .context("write_file requires a 'content' string argument")?;

        let resolved = self.runtime.resolve_within(path)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(&resolved, content)
            .await
            .with_context(|| format!("Failed to write file '{}'", resolved.display()))?;

        Ok(format!("Wrote {} bytes to {}", content.len(), resolved.display()))
    }
}

// ============================================================
// 内置工具：list_dir
// ============================================================

/// 列出目录内容。
pub struct ListDirTool {
    runtime: Arc<Runtime>,
}

impl ListDirTool {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List files and subdirectories of a directory within the project. Args: {\"path\": string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path relative to the project root" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let resolved = self.runtime.resolve_within(path)?;
        let mut entries = tokio::fs::read_dir(&resolved)
            .await
            .with_context(|| format!("Failed to read directory '{}'", resolved.display()))?;

        let mut lines = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let suffix = if file_type.is_dir() { "/" } else { "" };
            lines.push(format!("{}{}", entry.file_name().to_string_lossy(), suffix));
        }
        lines.sort();

        if lines.is_empty() {
            Ok(format!("{} is empty", resolved.display()))
        } else {
            Ok(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        assert!(registry.execute("nope", json!({})).await.is_err());
    }

    #[tokio::test]
    async fn write_then_read_file() {
        let dir = std::env::temp_dir().join("omega_tools_test");
        let runtime = Arc::new(Runtime::new(&dir));
        let registry = ToolRegistry::with_defaults(runtime);

        registry
            .execute("write_file", json!({ "path": "a.txt", "content": "hello" }))
            .await
            .unwrap();

        let content = registry
            .execute("read_file", json!({ "path": "a.txt" }))
            .await
            .unwrap();

        assert_eq!(content, "hello");

        tokio::fs::remove_dir_all(&dir).await.ok();
    }
}
