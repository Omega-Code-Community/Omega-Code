use futures_util::StreamExt;
use sha2::{Digest, Sha256};

use std::{
    fs::File,
    io::Write,
    path::Path,
    time::Instant,
};

pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
    pub percent: f64,
}

pub async fn download_file<F>(
    url: &str,
    save_path: &Path,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(DownloadProgress),
{
    let response = reqwest::get(url).await?;

    let total = response.content_length().unwrap_or(0);

    let mut stream = response.bytes_stream();

    let mut file = File::create(save_path)?;

    let mut downloaded = 0u64;

    let start = Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        file.write_all(&chunk)?;

        downloaded += chunk.len() as u64;

        let elapsed = start.elapsed().as_secs_f64();

        callback(DownloadProgress {
            downloaded,
            total,
            speed: downloaded as f64 / elapsed,
            percent: if total == 0 {
                0.0
            } else {
                downloaded as f64 / total as f64 * 100.0
            },
        });
    }

    Ok(())
}

pub fn verify_sha256(
    path: &Path,
    expected: &str,
) -> anyhow::Result<()> {
    let data = std::fs::read(path)?;

    let mut hasher = Sha256::new();

    hasher.update(data);

    let result = hex::encode(hasher.finalize());

    if result != expected {
        anyhow::bail!("sha256 mismatch");
    }

    Ok(())
}

pub fn verify_md5(
    path: &Path,
    expected: &str,
) -> anyhow::Result<()> {
    let data = std::fs::read(path)?;

    let result = format!("{:x}", md5::compute(data));

    if result != expected {
        anyhow::bail!("md5 mismatch");
    }

    Ok(())
}