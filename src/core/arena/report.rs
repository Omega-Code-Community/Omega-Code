//! Prompt 优化报告智能体（设计 3.8.3 数据闭环）
//!
//! 复用竞技场评估数据（星级分布、多维度均值、用户文字评价、当前系统提示词），以独立
//! provider 调 LLM 产出结构化的 Prompt 优化建议报告，并可落盘留档。
//!
//! 与 [`crate::core::memory::MemoryExtractor`] 同款模式：构造**独立** provider（不借用
//! Agent 的 `Box<dyn Provider>`），以便在 CLI 命令中一次性调用。LLM 调用不在单测覆盖内，
//! 单测仅验证纯函数 [`render_report_input`] 与落盘 [`save_report_to`]。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::core::providers::{create_provider, CompletionRequest, Message, ProviderConfig, Role};
use crate::platform::get_platform_app_dir;

use super::{ReportData, DIMENSIONS};

/// 报告智能体系统提示词。
pub const REPORT_PROMPT: &str = "\
你是一名 Prompt 优化专家。下面给出某个 Agent 的当前系统提示词、竞技场星级评分分布、\
多维度评估均值与用户文字评价。请据此产出一份结构化的 Prompt 优化建议报告，使用 Markdown，包含：
1. 问题诊断：结合低分档位与差评，指出当前提示词可能导致的问题；
2. 改进建议：针对每个问题给出可操作的改进方向；
3. 建议的新提示词要点：给出优化后提示词的关键条目（可直接采用的措辞）。
请保持客观、具体，避免空泛。";

/// Prompt 优化报告智能体。
pub struct PromptReportAgent {
    provider_config: ProviderConfig,
    model: String,
    temperature: Option<f32>,
}

impl PromptReportAgent {
    /// 构造报告智能体（复用被评 Agent 的供应商/模型配置）。
    pub fn new(provider_config: ProviderConfig, model: String, temperature: Option<f32>) -> Self {
        Self {
            provider_config,
            model,
            temperature,
        }
    }

    /// 调用 LLM 生成优化报告正文（Markdown）。
    pub async fn generate(&self, data: &ReportData) -> Result<String> {
        let provider = create_provider(self.provider_config.clone())?;
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: REPORT_PROMPT.to_string(),
                },
                Message {
                    role: Role::User,
                    content: render_report_input(data),
                },
            ],
            temperature: self.temperature,
            max_tokens: None,
            stream: false,
        };
        let response = provider.complete(request).await?;
        Ok(response.content)
    }
}

/// 将评估数据拼装为供 LLM 分析的 Markdown 输入（纯函数，可测）。
pub fn render_report_input(data: &ReportData) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Agent 评估数据：{} (#{})\n\n",
        data.agent_name, data.agent_id
    ));

    out.push_str("## 当前系统提示词\n");
    if data.system_prompt.trim().is_empty() {
        out.push_str("(未配置)\n");
    } else {
        out.push_str(&data.system_prompt);
        out.push('\n');
    }
    out.push('\n');

    let avg = match data.avg_score {
        Some(a) => format!("{:.1}", a),
        None => "-".to_string(),
    };
    out.push_str(&format!(
        "## 综合评分\n平均分：{} / 5，有效投票：{}\n\n",
        avg, data.vote_count
    ));

    out.push_str("## 星级分布\n");
    for star in 1..=5 {
        out.push_str(&format!(
            "- {} 星：{}\n",
            star, data.distribution[(star - 1) as usize]
        ));
    }
    out.push('\n');

    out.push_str("## 多维度评估均值\n");
    if data.dimensions.is_empty() {
        out.push_str("(无多维度数据)\n");
    } else {
        for (key, avg, count) in &data.dimensions {
            let label = DIMENSIONS
                .iter()
                .find(|(k, _)| *k == key.as_str())
                .map(|(_, l)| *l)
                .unwrap_or(key.as_str());
            out.push_str(&format!(
                "- {} ({})：{:.1} / 5（样本 {}）\n",
                label, key, avg, count
            ));
        }
    }
    out.push('\n');

    out.push_str("## 用户评价\n");
    if data.comments.is_empty() {
        out.push_str("(无文字评价)\n");
    } else {
        for c in &data.comments {
            out.push_str(&format!("- {}\n", c));
        }
    }

    out
}

/// 将报告写入 `~/.omega/reports/agent_<id>_<ts>.md`，返回文件路径。
pub fn save_report(agent_id: i32, content: &str) -> Result<PathBuf> {
    let dir = get_platform_app_dir()?.join("reports");
    save_report_to(&dir, agent_id, content)
}

/// 写入指定目录（供测试传 tempdir）。
fn save_report_to(dir: &Path, agent_id: i32, content: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create report dir: {}", dir.display()))?;
    let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let path = dir.join(format!("agent_{}_{}.md", agent_id, ts));
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write report: {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> ReportData {
        ReportData {
            agent_id: 7,
            agent_name: "Coder".to_string(),
            system_prompt: "You are a coder.".to_string(),
            avg_score: Some(3.5),
            vote_count: 4,
            distribution: [0, 1, 1, 1, 1],
            dimensions: vec![
                ("acc".to_string(), 4.0, 2),
                ("rel".to_string(), 3.0, 2),
            ],
            comments: vec!["too verbose".to_string(), "helpful".to_string()],
        }
    }

    #[test]
    fn render_report_input_contains_sections() {
        let text = render_report_input(&sample_data());
        assert!(text.contains("Coder"));
        assert!(text.contains("准确性"));
        assert!(text.contains("相关性"));
        assert!(text.contains("too verbose"));
        assert!(text.contains("You are a coder."));
        // 星级分布计数（distribution[4]=1 → “5 星：1”）。
        assert!(text.contains("5 星：1"));
    }

    #[test]
    fn render_report_input_handles_empty() {
        let data = ReportData {
            agent_id: 1,
            agent_name: "A".to_string(),
            system_prompt: String::new(),
            avg_score: None,
            vote_count: 0,
            distribution: [0; 5],
            dimensions: vec![],
            comments: vec![],
        };
        let text = render_report_input(&data);
        assert!(text.contains("(未配置)"));
        assert!(text.contains("(无多维度数据)"));
        assert!(text.contains("(无文字评价)"));
    }

    #[test]
    fn save_report_to_writes_file() {
        let dir = std::env::temp_dir().join(format!(
            "omega-report-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let path = save_report_to(&dir, 7, "# hello").unwrap();
        assert!(path.exists());
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(name.starts_with("agent_7_"));
        assert!(name.ends_with(".md"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# hello");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
