use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    platform::{
        detect::{current_platform, Os},
        get_app_runtime_dir,
    },
    runtime::{

        download::*,
        extract::*,
        manifest::*,
    },
};

pub struct RuntimePaths {
    pub java: PathBuf,
    pub python: PathBuf,
}

pub async fn ensure_jre<
    DF,
    EF,
>(
    mut download_callback: DF,
    mut extract_callback: EF,
) -> anyhow::Result<PathBuf>
where
    DF: FnMut(DownloadProgress),
    EF: FnMut(ExtractProgress),
{
    let platform = current_platform();

    let runtime_root =
        get_app_runtime_dir()?;

    let current_dir =
        runtime_root
            .join("jre")
            .join("current");

    if let Ok(path) =
        find_java_binary(
            &current_dir,
            platform.os,
        )
    {
        return Ok(path);
    }

    let package =
        jre_package(platform);

    let download_dir =
        runtime_root.join("downloads");

    let tmp_dir =
        runtime_root.join("tmp");

    fs::create_dir_all(&download_dir)?;
    fs::create_dir_all(&tmp_dir)?;

    let archive_path =
        download_dir.join("jre.download");

    let install_tmp =
        tmp_dir.join("jre.tmp");

    if install_tmp.exists() {
        fs::remove_dir_all(&install_tmp)?;
    }

    download_file(
        package.url,
        &archive_path,
        &mut download_callback,
    )
        .await?;

    if package.is_sha256 {
        verify_sha256(
            &archive_path,
            package.checksum,
        )?;
    } else {
        verify_md5(
            &archive_path,
            package.checksum,
        )?;
    }

    fs::create_dir_all(&install_tmp)?;

    if package.archive_name.ends_with(".zip") {
        extract_zip(
            &archive_path,
            &install_tmp,
            &mut extract_callback,
        )?;
    } else {
        extract_tar_gz(
            &archive_path,
            &install_tmp,
            &mut extract_callback,
        )?;
    }

    if current_dir.exists() {
        fs::remove_dir_all(&current_dir)?;
    }

    if let Some(parent) = current_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(
        &install_tmp,
        &current_dir,
    )?;

    find_java_binary(
        &current_dir,
        platform.os,
    )
}

pub async fn ensure_python<
    DF,
    EF,
>(
    mut download_callback: DF,
    mut extract_callback: EF,
) -> anyhow::Result<PathBuf>
where
    DF: FnMut(DownloadProgress),
    EF: FnMut(ExtractProgress),
{
    let platform = current_platform();

    let runtime_root =
        get_app_runtime_dir()?;

    let current_dir =
        runtime_root
            .join("python")
            .join("current");

    if let Ok(path) =
        find_python_binary(
            &current_dir,
            platform.os,
        )
    {
        return Ok(path);
    }

    let package =
        python_package(platform);

    let download_dir =
        runtime_root.join("downloads");

    let tmp_dir =
        runtime_root.join("tmp");

    fs::create_dir_all(&download_dir)?;
    fs::create_dir_all(&tmp_dir)?;

    let archive_path =
        download_dir.join("python.download");

    let install_tmp =
        tmp_dir.join("python.tmp");

    if install_tmp.exists() {
        fs::remove_dir_all(&install_tmp)?;
    }

    download_file(
        package.url,
        &archive_path,
        &mut download_callback,
    )
        .await?;

    if package.is_sha256 {
        verify_sha256(
            &archive_path,
            package.checksum,
        )?;
    } else {
        verify_md5(
            &archive_path,
            package.checksum,
        )?;
    }

    fs::create_dir_all(&install_tmp)?;

    if package.archive_name.ends_with(".zip") {
        extract_zip(
            &archive_path,
            &install_tmp,
            &mut extract_callback,
        )?;
    } else {
        extract_tar_gz(
            &archive_path,
            &install_tmp,
            &mut extract_callback,
        )?;
    }

    if current_dir.exists() {
        fs::remove_dir_all(&current_dir)?;
    }

    if let Some(parent) = current_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(
        &install_tmp,
        &current_dir,
    )?;

    find_python_binary(
        &current_dir,
        platform.os,
    )
}

fn find_java_binary(
    dir: &Path,
    os: Os,
) -> anyhow::Result<PathBuf> {
    let names = match os {
        Os::Windows => vec!["java.exe"],
        _ => vec!["java"],
    };

    find_binary_recursive(dir, &names)
}

fn find_python_binary(
    dir: &Path,
    os: Os,
) -> anyhow::Result<PathBuf> {
    let names = match os {
        Os::Windows => vec!["python.exe"],
        _ => vec!["python3", "python"],
    };

    find_binary_recursive(dir, &names)
}

fn find_binary_recursive(
    dir: &Path,
    names: &[&str],
) -> anyhow::Result<PathBuf> {
    if !dir.exists() {
        anyhow::bail!("runtime not installed");
    }

    for entry in walk(dir)? {
        if let Some(name) =
            entry.file_name()
        {
            let name =
                name.to_string_lossy();

            if names.iter().any(|n| *n == name) {
                return Ok(entry);
            }
        }
    }

    anyhow::bail!("binary not found")
}

fn walk(
    dir: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = vec![];

    for entry in fs::read_dir(dir)? {
        let entry = entry?;

        let path = entry.path();

        if path.is_dir() {
            result.extend(walk(&path)?);
        } else {
            result.push(path);
        }
    }

    Ok(result)
}