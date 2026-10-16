//! 命令行参数体系
//!
//! 依据《命令行参数设计》定义 Omega CLI 的完整参数结构：全局启动参数
//! （Workspace 路径、`--mode` 启动模式）与六大类子命令（模型 / 供应商 / Agent /
//! 会话 / 记忆 / 安全 / 性能）。
//!
//! 参数优先级：**命令行参数 > 配置文件 > 系统默认配置**（见设计文档六.开发备注）。
//! 本模块只负责「解析与校验」，具体命令的业务落地在 [`commands`] 中实现。

pub mod commands;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Omega CLI 顶层参数。
///
/// 无子命令时以 [`Cli::workspace`] 解析出的路径作为 Workspace 启动交互式 TUI；
/// 带子命令时执行对应的管理动作（见 [`commands::dispatch`]）。
#[derive(Parser, Debug)]
#[command(
    name = "Omega",
    version,
    about = "Omega Code —— CLI-First 本地 AI Agent 全栈运行时",
    long_about = None,
    arg_required_else_help = false
)]
pub struct Cli {
    /// 本地路径（绝对或相对），作为 Omega 的 Workspace，用于工程配置与文件索引构建。
    #[arg(value_name = "LocalPath", global = false)]
    pub path: Option<PathBuf>,

    /// 启动模式：single（单模式）/ workspace（工作空间模式），决定 Agent、MCP 的加载范围与权限管控。
    #[arg(long, value_enum, default_value_t = Mode::Single)]
    pub mode: Mode,

    /// 子命令（缺省时启动交互式 TUI）。
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// 启动模式。
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    /// 单模式：仅当前 Workspace，加载范围最小。
    Single,
    /// 工作空间模式：完整加载工作空间的 Agent、MCP 能力。
    Workspace,
}

impl Mode {
    /// 解析后的模式字符串（用于写入运行配置）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Single => "single",
            Mode::Workspace => "workspace",
        }
    }
}

impl Cli {
    /// 解析 Workspace 路径：显式 `LocalPath` 优先，否则回退到当前工作目录。
    ///
    /// 相对路径基于当前进程工作目录解析为绝对路径。
    pub fn workspace(&self) -> anyhow::Result<PathBuf> {
        let base = match &self.path {
            Some(p) => p.clone(),
            None => std::env::current_dir()?,
        };
        Ok(if base.is_absolute() {
            base
        } else {
            std::env::current_dir()?.join(base)
        })
    }
}

/// 全部子命令。
#[derive(Subcommand, Debug)]
pub enum Command {
    /// LLM 模型管理：list / status / set / sync / delete。
    Model {
        #[command(subcommand)]
        action: ModelCommand,
    },
    /// AI 供应商配置：加载指定供应商，可选覆盖 API 密钥。
    Supplier(SupplierArgs),
    /// Agent 管理：use / mcp。
    Agent {
        #[command(subcommand)]
        action: AgentCommand,
    },
    /// 会话管理：加载会话、清除上下文，或 list / rename / delete。
    Chat(ChatArgs),
    /// 记忆加载：指定加载的记忆类型。
    Memory(MemoryArgs),
    /// 安全策略：指定 MCP 安全策略。
    Security(SecurityArgs),
    /// 性能模式：指定 LLM 使用策略与 Token 消耗档位。
    Performance(PerformanceArgs),
    /// 竞技场：对 Agent 单轮输出星级打分与查看排行榜。
    Arena {
        #[command(subcommand)]
        action: ArenaCommand,
    },
    /// 提示词管理：list / show / diff / rollback / validate / render（§3.6 版本对比、回滚与校验预览）。
    Prompt {
        #[command(subcommand)]
        action: PromptCommand,
    },
}

/// `model` 子命令。
#[derive(Subcommand, Debug)]
pub enum ModelCommand {
    /// 列出模型（默认仅启用；`-a` 含未启用）。
    List {
        /// 列出所有可用模型（含未启用）。
        #[arg(short = 'a')]
        all: bool,
        /// 跳过确认步骤。
        #[arg(short = 'y')]
        yes: bool,
    },
    /// 查看指定模型的状态与参数配置。
    Status {
        /// 模型标识（`model_code`），如 deepseek-v4。
        name: String,
        /// 跳过确认步骤。
        #[arg(short = 'y')]
        yes: bool,
    },
    /// 配置模型参数（覆盖模型默认配置）。
    Set {
        /// 模型标识（`model_code`）。
        name: String,
        /// 采样温度，取值 0.0~1.0（Agent 级参数）。
        #[arg(short = 't')]
        temperature: Option<f32>,
        /// 最大上下文 Token（正整数）。
        #[arg(long = "max-tokens")]
        max_tokens: Option<i32>,
    },
    /// 从供应商 API 自动同步模型列表到本地库（§3.4 模型自动同步）。
    Sync {
        /// 供应商 ID（或名称）。
        #[arg(short = 's')]
        supplier: String,
    },
    /// 删除模型（物理删除；被 Agent 引用时拒绝，§3.4 增删查）。
    Delete {
        /// 模型标识（`model_code`）。
        name: String,
        /// 跳过二次确认。
        #[arg(short = 'y')]
        yes: bool,
    },
}

/// `supplier` 子命令参数。
#[derive(Args, Debug)]
pub struct SupplierArgs {
    /// 供应商 ID（或名称）。
    #[arg(short = 's')]
    pub supplier: String,
    /// API 密钥（优先级高于配置文件）。
    #[arg(long = "api-key")]
    pub api_key: Option<String>,
}

/// `agent` 子命令。
#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// 指定默认使用的 Agent。
    Use {
        /// Agent ID。
        #[arg(short = 'a')]
        agent: i32,
        /// 覆盖 Agent 默认绑定的 Prompt ID。
        #[arg(long)]
        prompt: Option<i32>,
    },
    /// 指定 Agent 挂载的 MCP 工具集合。
    Mcp {
        /// Agent ID。
        #[arg(short = 'a')]
        agent: i32,
        /// MCP 工具 ID 列表，逗号分隔（如 1,2,3）。
        #[arg(long, value_delimiter = ',', required = true)]
        mcp: Vec<i32>,
    },
    /// 指定作为多 Agent 调度器（Coordinator）的 Agent。
    Coordinator {
        /// Agent ID。
        #[arg(short = 'a')]
        agent: i32,
    },
}

/// `chat` 子命令参数（既可加载会话，也可执行 `clear`）。
#[derive(Args, Debug)]
pub struct ChatArgs {
    /// 会话 UUID（与数据库 `chat_uuid` 一致），用于加载历史、继续对话。
    #[arg(long = "chat-uuid")]
    pub chat_uuid: Option<String>,
    /// 会话操作子命令。
    #[command(subcommand)]
    pub action: Option<ChatCommand>,
}

/// `chat` 的操作子命令。
#[derive(Subcommand, Debug)]
pub enum ChatCommand {
    /// 清除当前会话上下文缓存，重置会话状态（不删除历史记录）。
    Clear,
    /// 列出会话（id、标题、UUID、绑定模型、消息数）。
    List,
    /// 重命名会话标题（§3.9）。
    Rename {
        /// 会话 ID。
        id: i32,
        /// 新标题。
        title: String,
    },
    /// 物理删除会话及其全部消息（需 `-y` 二次确认，§3.9）。
    Delete {
        /// 会话 ID。
        id: i32,
        /// 跳过二次确认。
        #[arg(short = 'y')]
        yes: bool,
    },
}

/// `memory` 子命令参数。
#[derive(Args, Debug)]
pub struct MemoryArgs {
    /// 记忆类型：user / project / all / none。
    #[arg(long = "type", value_enum)]
    pub mem_type: MemoryType,
}

/// 记忆类型。
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum MemoryType {
    /// 用户记忆。
    User,
    /// 项目记忆。
    Project,
    /// 全部记忆。
    All,
    /// 不加载记忆。
    None,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::User => "user",
            MemoryType::Project => "project",
            MemoryType::All => "all",
            MemoryType::None => "none",
        }
    }
}

/// `security` 子命令参数。
#[derive(Args, Debug)]
pub struct SecurityArgs {
    /// 安全策略：default / strict / bypass。
    #[arg(long, value_enum)]
    pub mode: SecurityMode,
}

/// MCP 安全策略。
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum SecurityMode {
    /// 默认策略。
    Default,
    /// 严格策略。
    Strict,
    /// 宽松策略。
    Bypass,
}

impl SecurityMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityMode::Default => "default",
            SecurityMode::Strict => "strict",
            SecurityMode::Bypass => "bypass",
        }
    }
}

/// `performance` 子命令参数。
#[derive(Args, Debug)]
pub struct PerformanceArgs {
    /// 性能模式：performance / balance / quality。
    #[arg(long, value_enum)]
    pub mode: PerformanceMode,
}

/// 性能模式。
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PerformanceMode {
    /// 性能模式（低延迟、低 Token 消耗）。
    Performance,
    /// 平衡模式。
    Balance,
    /// 质量模式（高质量、高 Token 消耗）。
    Quality,
}

impl PerformanceMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PerformanceMode::Performance => "performance",
            PerformanceMode::Balance => "balance",
            PerformanceMode::Quality => "quality",
        }
    }
}

/// `arena` 子命令。
#[derive(Subcommand, Debug)]
pub enum ArenaCommand {
    /// 对指定 Agent 打分（1-5 星）并附可选评价。
    Vote {
        /// Agent ID。
        #[arg(short = 'a')]
        agent: i32,
        /// 星级评分，取值 1-5。
        #[arg(long)]
        score: i8,
        /// 文字评价（可选）。
        #[arg(long)]
        comment: Option<String>,
        /// 关联会话 UUID（可选）。
        #[arg(long = "chat-uuid")]
        chat_uuid: Option<String>,
        /// 关联单轮消息 UUID（可选）。
        #[arg(long = "round-uuid")]
        round_uuid: Option<String>,
        /// 准确性维度评分（1-5，可选）。
        #[arg(long)]
        acc: Option<i8>,
        /// 相关性维度评分（1-5，可选）。
        #[arg(long)]
        rel: Option<i8>,
        /// 流畅性维度评分（1-5，可选）。
        #[arg(long)]
        flu: Option<i8>,
        /// 有用性维度评分（1-5，可选）。
        #[arg(long)]
        helpful: Option<i8>,
    },
    /// 按平均评分展示 Agent 排行榜。
    List,
    /// 生成指定 Agent 的 Prompt 优化报告（LLM 驱动，打印并落盘）。
    Report {
        /// Agent ID。
        #[arg(short = 'a')]
        agent: i32,
    },
}

/// `prompt` 子命令（§3.6 提示词管理与版本能力）。
#[derive(Subcommand, Debug)]
pub enum PromptCommand {
    /// 列出全部提示词（按名称、版本排序）。
    List,
    /// 查看指定提示词版本的完整内容与元信息。
    Show {
        /// 提示词 ID。
        id: i32,
    },
    /// 对比两个提示词版本的内容差异（行级 diff）。
    Diff {
        /// 旧版本提示词 ID。
        from: i32,
        /// 新版本提示词 ID。
        to: i32,
    },
    /// 回滚：以指定历史版本内容生成同名新版本（追加，不覆盖历史）。
    Rollback {
        /// 目标历史版本提示词 ID。
        id: i32,
        /// 新版本备注（可选）。
        #[arg(long)]
        desc: Option<String>,
    },
    /// 校验提示词语法与占位符完整性（§3.6.5）。
    Validate {
        /// 提示词 ID。
        id: i32,
    },
    /// 渲染预览：用占位符值填充并打印最终提示词（§3.6.5）。
    Render {
        /// 提示词 ID。
        id: i32,
        /// 以指定 Agent 上下文填充 {AgentList}/{MCPList}（可选）。
        #[arg(long)]
        agent: Option<i32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// CLI 定义自身必须合法（ clap derive 结构体校验）。
    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_global_path_and_mode() {
        let cli = Cli::try_parse_from(["Omega", "./proj", "--mode", "workspace"]).unwrap();
        assert_eq!(cli.path, Some(PathBuf::from("./proj")));
        assert_eq!(cli.mode, Mode::Workspace);
        assert!(cli.command.is_none());
    }

    #[test]
    fn mode_defaults_to_single() {
        let cli = Cli::try_parse_from(["Omega"]).unwrap();
        assert_eq!(cli.mode, Mode::Single);
    }

    #[test]
    fn parses_model_subcommands() {
        let cli = Cli::try_parse_from(["Omega", "model", "list", "-a"]).unwrap();
        match cli.command {
            Some(Command::Model {
                action: ModelCommand::List { all, .. },
            }) => assert!(all),
            other => panic!("unexpected: {other:?}"),
        }

        let cli = Cli::try_parse_from(["Omega", "model", "set", "gpt-4o", "-t", "0.3", "--max-tokens", "8192"]).unwrap();
        match cli.command {
            Some(Command::Model {
                action:
                    ModelCommand::Set {
                        name,
                        temperature,
                        max_tokens,
                    },
            }) => {
                assert_eq!(name, "gpt-4o");
                assert_eq!(temperature, Some(0.3));
                assert_eq!(max_tokens, Some(8192));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_model_sync() {
        let cli = Cli::try_parse_from(["Omega", "model", "sync", "-s", "openai"]).unwrap();
        match cli.command {
            Some(Command::Model {
                action: ModelCommand::Sync { supplier },
            }) => assert_eq!(supplier, "openai"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_model_delete() {
        let cli = Cli::try_parse_from(["Omega", "model", "delete", "gpt-4o", "-y"]).unwrap();
        match cli.command {
            Some(Command::Model {
                action: ModelCommand::Delete { name, yes },
            }) => {
                assert_eq!(name, "gpt-4o");
                assert!(yes);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_chat_session_management() {
        assert!(matches!(
            Cli::try_parse_from(["Omega", "chat", "list"]).unwrap().command,
            Some(Command::Chat(ChatArgs {
                action: Some(ChatCommand::List),
                ..
            }))
        ));

        let cli = Cli::try_parse_from(["Omega", "chat", "rename", "4", "Sprint planning"]).unwrap();
        match cli.command {
            Some(Command::Chat(ChatArgs {
                action: Some(ChatCommand::Rename { id, title }),
                ..
            })) => {
                assert_eq!(id, 4);
                assert_eq!(title, "Sprint planning");
            }
            other => panic!("unexpected: {other:?}"),
        }

        let cli = Cli::try_parse_from(["Omega", "chat", "delete", "4", "-y"]).unwrap();
        match cli.command {
            Some(Command::Chat(ChatArgs {
                action: Some(ChatCommand::Delete { id, yes }),
                ..
            })) => {
                assert_eq!(id, 4);
                assert!(yes);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_agent_mcp_tool_ids() {
        let cli = Cli::try_parse_from(["Omega", "agent", "mcp", "-a", "5", "--mcp", "1,2,3"]).unwrap();
        match cli.command {
            Some(Command::Agent {
                action: AgentCommand::Mcp { agent, mcp },
            }) => {
                assert_eq!(agent, 5);
                assert_eq!(mcp, vec![1, 2, 3]);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_agent_coordinator() {
        let cli = Cli::try_parse_from(["Omega", "agent", "coordinator", "-a", "3"]).unwrap();
        match cli.command {
            Some(Command::Agent {
                action: AgentCommand::Coordinator { agent },
            }) => assert_eq!(agent, 3),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_arena_vote() {
        let cli = Cli::try_parse_from(
            ["Omega", "arena", "vote", "-a", "2", "--score", "5", "--comment", "good"],
        )
        .unwrap();
        match cli.command {
            Some(Command::Arena {
                action:
                    ArenaCommand::Vote {
                        agent,
                        score,
                        comment,
                        chat_uuid,
                        round_uuid,
                        ..
                    },
            }) => {
                assert_eq!(agent, 2);
                assert_eq!(score, 5);
                assert_eq!(comment.as_deref(), Some("good"));
                assert!(chat_uuid.is_none());
                assert!(round_uuid.is_none());
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_arena_vote_with_dims() {
        let cli = Cli::try_parse_from(
            ["Omega", "arena", "vote", "-a", "2", "--score", "4", "--acc", "5", "--rel", "3"],
        )
        .unwrap();
        match cli.command {
            Some(Command::Arena {
                action: ArenaCommand::Vote { acc, rel, flu, helpful, .. },
            }) => {
                assert_eq!(acc, Some(5));
                assert_eq!(rel, Some(3));
                assert_eq!(flu, None);
                assert_eq!(helpful, None);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_arena_report() {
        let cli = Cli::try_parse_from(["Omega", "arena", "report", "-a", "7"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Arena {
                action: ArenaCommand::Report { agent: 7 }
            })
        ));
    }

    #[test]
    fn parses_arena_list() {
        let cli = Cli::try_parse_from(["Omega", "arena", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Arena {
                action: ArenaCommand::List
            })
        ));
    }

    #[test]
    fn parses_prompt_subcommands() {
        let cli = Cli::try_parse_from(["Omega", "prompt", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Prompt {
                action: PromptCommand::List
            })
        ));

        let cli = Cli::try_parse_from(["Omega", "prompt", "diff", "3", "7"]).unwrap();
        match cli.command {
            Some(Command::Prompt {
                action: PromptCommand::Diff { from, to },
            }) => {
                assert_eq!(from, 3);
                assert_eq!(to, 7);
            }
            other => panic!("unexpected: {other:?}"),
        }

        let cli =
            Cli::try_parse_from(["Omega", "prompt", "rollback", "5", "--desc", "revert"]).unwrap();
        match cli.command {
            Some(Command::Prompt {
                action: PromptCommand::Rollback { id, desc },
            }) => {
                assert_eq!(id, 5);
                assert_eq!(desc.as_deref(), Some("revert"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_prompt_validate_and_render() {
        assert!(matches!(
            Cli::try_parse_from(["Omega", "prompt", "validate", "8"]).unwrap().command,
            Some(Command::Prompt {
                action: PromptCommand::Validate { id: 8 }
            })
        ));

        let cli = Cli::try_parse_from(["Omega", "prompt", "render", "8", "--agent", "3"]).unwrap();
        match cli.command {
            Some(Command::Prompt {
                action: PromptCommand::Render { id, agent },
            }) => {
                assert_eq!(id, 8);
                assert_eq!(agent, Some(3));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_chat_clear_and_uuid() {
        let cli = Cli::try_parse_from(["Omega", "chat", "clear"]).unwrap();
        match cli.command {
            Some(Command::Chat(ChatArgs {
                action: Some(ChatCommand::Clear),
                ..
            })) => {}
            other => panic!("unexpected: {other:?}"),
        }

        let cli = Cli::try_parse_from(["Omega", "chat", "--chat-uuid", "abc-123"]).unwrap();
        match cli.command {
            Some(Command::Chat(ChatArgs { chat_uuid, action } )) => {
                assert_eq!(chat_uuid.as_deref(), Some("abc-123"));
                assert!(action.is_none());
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_memory_security_performance() {
        let cli = Cli::try_parse_from(["Omega", "memory", "--type", "project"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Memory(MemoryArgs {
                mem_type: MemoryType::Project
            }))
        ));

        let cli = Cli::try_parse_from(["Omega", "security", "--mode", "strict"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Security(SecurityArgs {
                mode: SecurityMode::Strict
            }))
        ));

        let cli = Cli::try_parse_from(["Omega", "performance", "--mode", "quality"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Performance(PerformanceArgs {
                mode: PerformanceMode::Quality
            }))
        ));
    }

    #[test]
    fn workspace_resolves_relative_and_absolute() {
        let abs = Cli::try_parse_from(["Omega", "C:/some/proj"]).unwrap();
        assert_eq!(abs.workspace().unwrap(), PathBuf::from("C:/some/proj"));

        let rel = Cli::try_parse_from(["Omega", "proj"]).unwrap();
        let resolved = rel.workspace().unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("proj"));
    }
}
