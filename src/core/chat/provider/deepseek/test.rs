// 修复：删除重复导入
use crate::core::chat::provider::deepseek::{model::*, enums::*, message::*, tool::*, chat::*};
use reqwest::Client;
use std::env;
use dotenv::dotenv;
use serde_json;

const API_BASE: &str = "https://api.deepseek.com";

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    /// 获取 API Key（加载 .env 文件）
    fn get_api_key() -> String {
        dotenv().ok();
        env::var("API_KEY").expect("❌ 请在 .env 文件中设置 API_KEY 环境变量")
    }

    // 1. 测试：获取模型列表 ✅ 修复：移除 /v1 前缀
    #[tokio::test]
    async fn test_list_models() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let resp: ListModelsResponse = client
            .get(format!("{API_BASE}/models")) // 修复点1
            .header("Authorization", &auth)
            .send()
            .await?
            .json()
            .await?;

        assert_eq!(resp.object, "list");
        assert!(!resp.data.is_empty());

        println!("\n=====================================");
        println!("📋 模型列表测试结果");
        println!("=====================================");
        println!("{}", serde_json::to_string_pretty(&resp)?);
        println!("✅ 模型列表测试通过，共 {} 个模型", resp.data.len());

        Ok(())
    }

    // 2. 测试：普通对话 ✅ 修复：MessageContent 取值
    #[tokio::test]
    async fn test_chat_normal() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let req = CreateChatCompletionRequest {
            model: "deepseek-v4-flash".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("你好".into()),
                ..Default::default()
            }],
            max_tokens: Some(100),
            ..Default::default()
        };

        let resp: ChatCompletionResponse = client
            .post(format!("{API_BASE}/v1/chat/completions"))
            .header("Authorization", &auth)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        assert!(!resp.choices.is_empty());
        assert_eq!(resp.model, "deepseek-v4-flash");

        // 修复点2：直接获取 MessageContent 文本
        let reply = match &resp.choices[0].message.content {
            MessageContent::Text(text) => text,
            _ => "非文本消息",
        };

        println!("\n=====================================");
        println!("💬 普通对话测试结果");
        println!("=====================================");
        println!("AI 回复：{}", reply);
        println!("完整响应：{}", serde_json::to_string_pretty(&resp)?);
        println!("✅ 普通对话测试通过");

        Ok(())
    }

    // 3. 测试：深度思考模型 ✅ 修复：删除不存在的 reasoning_content 字段
    #[tokio::test]
    async fn test_chat_reasoner() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let req = CreateChatCompletionRequest {
            model: "deepseek-reasoner".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("1024*1024等于多少？".into()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let resp: ChatCompletionResponse = client
            .post(format!("{API_BASE}/v1/chat/completions"))
            .header("Authorization", &auth)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        assert!(!resp.choices.is_empty());

        // 修复点3：移除无此字段的打印
        let reply = match &resp.choices[0].message.content {
            MessageContent::Text(text) => text,
            _ => "非文本消息",
        };

        println!("\n=====================================");
        println!("🤯 深度思考模型测试结果");
        println!("=====================================");
        println!("最终回复：{}", reply);
        println!("完整响应：{}", serde_json::to_string_pretty(&resp)?);
        println!("✅ 思考模型测试通过");

        Ok(())
    }

    // 4. 测试：JSON 格式输出 ✅ 修复：MessageContent 取值
    #[tokio::test]
    async fn test_chat_json_format() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let req = CreateChatCompletionRequest {
            model: "deepseek-v4-flash".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("返回一个包含name和age的JSON".into()),
                ..Default::default()
            }],
            response_format: Some(ResponseFormat::default()),
            ..Default::default()
        };

        let resp: ChatCompletionResponse = client
            .post(format!("{API_BASE}/v1/chat/completions"))
            .header("Authorization", &auth)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        assert!(!resp.choices.is_empty());

        let json_result = match &resp.choices[0].message.content {
            MessageContent::Text(text) => text,
            _ => "非文本消息",
        };

        println!("\n=====================================");
        println!("📄 JSON 格式输出测试结果");
        println!("=====================================");
        println!("JSON 内容：{}", json_result);
        println!("完整响应：{}", serde_json::to_string_pretty(&resp)?);
        println!("✅ JSON输出测试通过");

        Ok(())
    }

    // 5. 测试：工具调用 ✅ 无报错，保持原样
    #[tokio::test]
    async fn test_chat_tool_call() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let req = CreateChatCompletionRequest {
            model: "deepseek-v4-flash".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("查询上海天气".into()),
                ..Default::default()
            }],
            tools: Some(vec![Tool {
                r#type: ToolType::Function,
                function: FunctionObject {
                    name: "get_weather".into(),
                    description: Some("获取城市天气".into()),
                    parameters: Some(serde_json::to_value(
                        serde_json::json!({
                            "type": "object",
                            "properties": {
                                "city": {"type": "string"}
                            },
                            "required": ["city"]
                        })
                    )?),
                },
            }]),
            tool_choice: Some(ToolChoice::Strategy("auto".into())),
            ..Default::default()
        };

        let resp: ChatCompletionResponse = client
            .post(format!("{API_BASE}/v1/chat/completions"))
            .header("Authorization", &auth)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        assert!(resp.choices[0].message.tool_calls.is_some());

        let tool_call = &resp.choices[0].message.tool_calls.as_ref().unwrap()[0];
        println!("\n=====================================");
        println!("🔧 工具调用测试结果");
        println!("=====================================");
        println!("调用函数：{}", tool_call.function.name);
        println!("调用参数：{}", tool_call.function.arguments);
        println!("完整响应：{}", serde_json::to_string_pretty(&resp)?);
        println!("✅ 工具调用测试通过");

        Ok(())
    }

    // 6. 测试：流式输出 ✅ 无报错，保持原样
    #[tokio::test]
    async fn test_chat_stream() -> anyhow::Result<()> {
        let client = Client::new();
        let auth = format!("Bearer {}", get_api_key());

        let req = CreateChatCompletionRequest {
            model: "deepseek-v4-flash".into(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("介绍Rust".into()),
                ..Default::default()
            }],
            stream: Some(true),
            stream_options: Some(StreamOptions {
                include_usage: Some(true),
            }),
            ..Default::default()
        };

        let mut stream = client
            .post(format!("{API_BASE}/v1/chat/completions"))
            .header("Authorization", &auth)
            .json(&req)
            .send()
            .await?
            .bytes_stream();

        println!("\n=====================================");
        println!("🌊 流式输出测试结果（实时打印）");
        println!("=====================================");

        let mut full_content = String::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let s = String::from_utf8_lossy(&chunk);
            print!("{}", s);
            full_content.push_str(&s);
        }

        assert!(!full_content.is_empty());
        println!("\n✅ 流式输出测试通过");

        Ok(())
    }
}