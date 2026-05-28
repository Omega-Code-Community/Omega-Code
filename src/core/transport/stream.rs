use futures_util::StreamExt;

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