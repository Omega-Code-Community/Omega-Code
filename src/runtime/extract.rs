use flate2::read::GzDecoder;

use std::{
    fs::File,
    io,
    path::Path,
};

use tar::Archive;
use zip::ZipArchive;

pub struct ExtractProgress {
    pub current: u64,
    pub total: u64,
}

pub fn extract_zip<F>(
    archive_path: &Path,
    target_dir: &Path,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(ExtractProgress),
{
    let file = File::open(archive_path)?;

    let mut archive = ZipArchive::new(file)?;

    let total = archive.len() as u64;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let outpath = target_dir.join(entry.name());

        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = File::create(&outpath)?;

            io::copy(&mut entry, &mut outfile)?;
        }

        callback(ExtractProgress {
            current: i as u64 + 1,
            total,
        });
    }

    Ok(())
}

pub fn extract_tar_gz<F>(
    archive_path: &Path,
    target_dir: &Path,
    mut callback: F,
) -> anyhow::Result<()>
where
    F: FnMut(ExtractProgress),
{
    let file = File::open(archive_path)?;

    let decoder = GzDecoder::new(file);

    let mut archive = Archive::new(decoder);

    let mut index = 0;

    for entry in archive.entries()? {
        let mut entry = entry?;

        entry.unpack_in(target_dir)?;

        index += 1;

        callback(ExtractProgress {
            current: index,
            total: 0,
        });
    }

    Ok(())
}