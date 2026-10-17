use futures_util::StreamExt;

// 预留：SSE 原始字节流调试打印。当前流式响应由 AiClient::post_sse 处理，此函数尚未接入。
#[allow(dead_code)]
pub async fn read_sse(
    response: reqwest::Response,
) -> anyhow::Result<()> {
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        let text = String::from_utf8_lossy(&chunk);

        println!("{}", text);
    }

    Ok(())
}