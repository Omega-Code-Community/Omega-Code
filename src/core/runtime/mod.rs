//! 运行时管理
//!
//! 该模块为 Agent 与工具系统提供统一的「执行上下文」：
//! - 维护当前工作目录（项目根目录）
//! - 提供相对路径解析，并对越界访问进行约束
//!
//! 注意：JRE / Python 等语言运行时的下载与安装逻辑位于顶层
//! [`crate::runtime`] 模块，本模块专注于 Agent 侧的工作目录上下文，
//! 二者职责不同，互不重叠。

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

/// Agent 运行时上下文。
///
/// 以项目工作目录为中心，为工具（读写文件、列目录等）提供路径解析能力。
#[derive(Debug, Clone)]
pub struct Runtime {
    working_dir: PathBuf,
}

impl Runtime {
    /// 使用指定工作目录创建运行时。
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
        }
    }

    /// 基于当前进程工作目录创建运行时。
    pub fn from_current_dir() -> Result<Self> {
        let dir = std::env::current_dir().context("Failed to get current working directory")?;
        Ok(Self::new(dir))
    }

    /// 当前工作目录。
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// 将用户提供的路径解析为绝对路径。
    ///
    /// - 绝对路径原样返回（经规范化处理）
    /// - 相对路径基于工作目录拼接
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_dir.join(path)
        };
        normalize(&full)
    }

    /// 解析路径并校验其位于工作目录之内，防止越界访问。
    ///
    /// 对于需要沙箱化工具（如写文件）时使用。
    pub fn resolve_within(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let resolved = self.resolve(path);

        let root = normalize(&self.working_dir);
        let is_within = resolved
            .ancestors()
            .any(|ancestor| ancestor == root.as_path());

        if is_within {
            Ok(resolved)
        } else {
            anyhow::bail!(
                "Path '{}' escapes the working directory '{}'",
                resolved.display(),
                root.display()
            )
        }
    }
}

/// 规范化路径：处理 `.` 与 `..` 组件，不访问文件系统。
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // 仅当上一级不是根前缀时才回退，避免越出文件系统根。
                if !matches!(result.components().next_back(), Some(Component::RootDir)) {
                    result.pop();
                }
            }
            other => result.push(other.as_os_str()),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_relative_path() {
        let rt = Runtime::new("/home/project");
        assert_eq!(rt.resolve("src/main.rs"), PathBuf::from("/home/project/src/main.rs"));
    }

    #[test]
    fn resolve_absolute_path() {
        let rt = Runtime::new("/home/project");
        assert_eq!(rt.resolve("/etc/hosts"), PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn within_working_dir() {
        let rt = Runtime::new("/home/project");
        assert!(rt.resolve_within("src/main.rs").is_ok());
        assert!(rt.resolve_within("../secret").is_err());
    }
}
