#[derive(Debug)]
pub struct VersionMetadata {
    pub version: String,
}

pub fn get_omega_version() -> Vec<VersionMetadata> {
    let version = env!("CARGO_PKG_VERSION").to_string();

    vec![VersionMetadata {
        version,
    }]
}