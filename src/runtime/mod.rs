pub mod manifest;
pub mod download;
pub mod extract;
pub mod runtime;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct ExtractProgress {
    pub extracted: Option<u64>,
    pub total: Option<u64>,
}