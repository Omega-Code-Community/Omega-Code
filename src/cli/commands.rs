//! 命令处理
//!
//! 把 [`crate::cli`] 解析出的子命令落地为具体的管理动作：读写本地 SQLite
//! （经 [`DatabaseManager`]）并把结果打印到标准输出。交互式 TUI 的启动不在本模块，
//! 由 [`crate::run_with`] 决定。
//!
//! 设计约束（《命令行参数设计》六.开发备注）：
//! - 参数优先级：命令行 > 配置文件 > 系统默认；策略类命令（模型温度、记忆、安全、
//!   性能）统一以键值形式落地到 `system_config`，作为可被后续启动读取的「配置覆盖」。
//! - 敏感字段（API Key）仅在需要时写入，不回显明文。

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::core::agent::SessionStore;
use crate::core::arena::report::{self, PromptReportAgent};
use crate::core::config::parse_provider_kind;
use crate::core::db::DatabaseManager;
use crate::core::arena::{ArenaService, VoteInput};
use crate::core::prompt::{build_vars, render, validate_prompt};
use crate::core::providers::{create_provider, ProviderConfig, ProviderKind};
use crate::db::models::agent_info::AgentInfo;
use crate::db::models::ai_provider::AiProvider;
use crate::db::models::chat_session::ChatSession;
use crate::db::models::llm_model::LLMModel;
use crate::db::models::prompt_info::PromptInfo;
use crate::db::models::system_config::SystemConfig;

use super::{
    AgentCommand, ArenaCommand, ChatArgs, ChatCommand, Command, MemoryArgs, ModelCommand,
    PerformanceArgs, PromptCommand, SecurityArgs, SupplierArgs,
};

/// 分发并执行一个子命令。
pub async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Model { action } => model(action).await,
        Command::Supplier(args) => supplier(args),
        Command::Agent { action } => agent(action),
        Command::Chat(args) => chat(args),
        Command::Memory(args) => memory(args),
        Command::Security(args) => security(args),
        Command::Performance(args) => performance(args),
        Command::Arena { action } => arena(action).await,
        Command::Prompt { action } => prompt(action),
    }
}

// ============================================================
// model
// ============================================================

async fn model(action: ModelCommand) -> Result<()> {
    let db = DatabaseManager::new()?;
    match action {
        ModelCommand::List { all, .. } => {
            let models = db.read_all::<LLMModel>()?;
            println!("{:<5}{:<28}{:<12}{:<10}", "id", "model_code", "context", "status");
            let mut shown = 0usize;
            for m in models {
                if !all && m.status != 1 {
                    continue;
                }
                println!(
                    "{:<5}{:<28}{:<12}{:<10}",
                    m.id,
                    m.model_code,
                    m.max_tokens,
                    if m.status == 1 { "enabled" } else { "disabled" }
                );
                shown += 1;
            }
            if shown == 0 {
                println!("(no models matched)");
            }
        }
        ModelCommand::Status { name, .. } => {
            let mut found = db.find_where::<LLMModel>("model_code = ?1", &[&name])?;
            let m = found
                .first()
                .with_context(|| format!("model '{}' not found", name))?;
            println!("model      : {}", m.model_code);
            println!("id         : {}", m.id);
            println!("name       : {}", m.name);
            println!("provider   : #{}", m.provider_id);
            println!("context    : {}", m.max_tokens);
            println!("max_output : {}", m.max_response_tokens);
            println!("weight     : {}", m.weight);
            println!("calls      : {}", m.call_count);
            println!("status     : {}", if m.status == 1 { "enabled" } else { "disabled" });
            // 温度是 Agent 级参数，若曾以命令行覆盖则一并展示。
            if let Some(temp) = read_config(&db, &format!("model.{}.temperature", m.model_code))? {
                println!("temperature: {} (override)", temp);
            }
            let _ = &mut found;
        }
        ModelCommand::Set {
            name,
            temperature,
            max_tokens,
        } => {
            let mut found = db.find_where::<LLMModel>("model_code = ?1", &[&name])?;
            let m = found
                .first_mut()
                .with_context(|| format!("model '{}' not found", name))?;

            if let Some(mt) = max_tokens {
                if mt <= 0 {
                    anyhow::bail!("--max-tokens must be a positive integer");
                }
                m.max_tokens = mt;
                db.update(m.clone())?;
                println!("set model '{}' max context tokens = {}", name, mt);
            }
            if let Some(t) = temperature {
                if !(0.0..=1.0).contains(&t) {
                    anyhow::bail!("-t must be within 0.0~1.0");
                }
                // 温度属于 Agent 级个性化参数（llm_model 无对应列），
                // 以 system_config 覆盖项持久化，供后续加载时优先采用。
                upsert_config(
                    &db,
                    &format!("model.{}.temperature", name),
                    &t.to_string(),
                    "float",
                    &format!("CLI override temperature for model {}", name),
                    "model",
                )?;
                println!("set model '{}' temperature override = {}", name, t);
            }
            if max_tokens.is_none() && temperature.is_none() {
                println!("nothing to set (provide -t and/or --max-tokens)");
            }
        }
        ModelCommand::Sync { supplier } => {
            // 解析供应商（先 ID 再名称），与 `supplier` 命令保持一致。
            let mut resolved = match supplier.parse::<i32>() {
                Ok(id) => db.read_by_id::<AiProvider>(id)?,
                Err(_) => None,
            };
            if resolved.is_none() {
                resolved = db
                    .find_where::<AiProvider>("name = ?1", &[&supplier])?
                    .into_iter()
                    .next();
            }
            let provider = resolved
                .with_context(|| format!("supplier '{}' not found", supplier))?;

            // 由供应商行构建配置（kind 取名称，base_url 取 api，key 可空）。
            let config = ProviderConfig {
                kind: parse_provider_kind(&provider.name).unwrap_or(ProviderKind::Compatible),
                api_key: provider.key.clone().unwrap_or_default(),
                base_url: provider.api.clone(),
                model: String::new(),
            };
            let codes = create_provider(config)?.list_models().await?;
            let (added, skipped) = sync_models(&db, provider.id, &codes)?;
            println!(
                "synced {} model(s) from supplier '{}' (#{}): +{} added, {} existing",
                codes.len(),
                provider.name,
                provider.id,
                added,
                skipped
            );
        }
        ModelCommand::Delete { name, yes } => {
            // 先做存在性与引用完整性校验（逻辑外键，业务层保障）。
            match check_model_deletable(&db, &name)? {
                ModelDeletable::NotFound => {
                    anyhow::bail!("model '{}' not found", name)
                }
                ModelDeletable::Blocked { refs } => anyhow::bail!(
                    "model '{}' is referenced by {} agent(s); reassign them before deleting",
                    name,
                    refs
                ),
                ModelDeletable::Ready(m) => {
                    if !yes {
                        println!(
                            "this will permanently delete model '{}' (#{}); re-run with -y to confirm",
                            m.model_code, m.id
                        );
                    } else {
                        db.delete::<LLMModel>(m.id)?;
                        println!("deleted model '{}' (#{})", m.model_code, m.id);
                    }
                }
            }
        }
    }
    Ok(())
}

// ============================================================
// supplier
// ============================================================

fn supplier(args: SupplierArgs) -> Result<()> {
    let db = DatabaseManager::new()?;

    // 先按 ID 匹配，失败再按名称匹配。
    let mut provider = match args.supplier.parse::<i32>() {
        Ok(id) => db.read_by_id::<AiProvider>(id)?,
        Err(_) => None,
    };
    if provider.is_none() {
        provider = db
            .find_where::<AiProvider>("name = ?1", &[&args.supplier])?
            .into_iter()
            .next();
    }
    let mut provider =
        provider.with_context(|| format!("supplier '{}' not found", args.supplier))?;

    if let Some(key) = &args.api_key {
        provider.key = Some(key.clone());
        db.update(provider.clone())?;
        println!("updated api key for supplier '{}' (#{})", provider.name, provider.id);
    }

    println!(
        "supplier #{} '{}' -> {} [{}]",
        provider.id,
        provider.name,
        provider.api,
        if provider.status == 1 { "enabled" } else { "disabled" }
    );
    Ok(())
}

// ============================================================
// agent
// ============================================================

fn agent(action: AgentCommand) -> Result<()> {
    let db = DatabaseManager::new()?;
    match action {
        AgentCommand::Use { agent, prompt } => {
            let mut a = db
                .read_by_id::<AgentInfo>(agent)?
                .with_context(|| format!("agent #{} not found", agent))?;
            if let Some(pid) = prompt {
                a.prompt_id = Some(pid);
                db.update(a.clone())?;
                println!("agent #{} prompt override -> #{}", agent, pid);
            }
            upsert_config(
                &db,
                "default_agent_id",
                &agent.to_string(),
                "int",
                "CLI selected default agent",
                "agent",
            )?;
            println!("default agent set to #{} ({})", a.id, a.name);
        }
        AgentCommand::Mcp { agent, mcp } => {
            // 校验 Agent 存在。
            db.read_by_id::<AgentInfo>(agent)?
                .with_context(|| format!("agent #{} not found", agent))?;

            // 全量替换该 Agent 的 MCP 授权集合。
            db.conn
                .execute("DELETE FROM agent_mcp_rel WHERE agent_id = ?1", params![agent])?;
            for tool_id in &mcp {
                db.conn.execute(
                    "INSERT INTO agent_mcp_rel (agent_id, mcp_tool_id) VALUES (?1, ?2)",
                    params![agent, tool_id],
                )?;
            }
            println!(
                "agent #{} now mounts {} MCP tool(s): {:?}",
                agent,
                mcp.len(),
                mcp
            );
        }
        AgentCommand::Coordinator { agent } => {
            // 校验 Agent 存在后，记录为协调者（TUI 加载该 Agent 时升级为调度器）。
            let a = db
                .read_by_id::<AgentInfo>(agent)?
                .with_context(|| format!("agent #{} not found", agent))?;
            upsert_config(
                &db,
                "coordinator_agent_id",
                &agent.to_string(),
                "int",
                "CLI selected coordinator agent",
                "agent",
            )?;
            println!("coordinator agent set to #{} ({})", a.id, a.name);
        }
    }
    Ok(())
}

// ============================================================
// chat
// ============================================================

fn chat(args: ChatArgs) -> Result<()> {
    match args.action {
        Some(ChatCommand::Clear) => {
            // 上下文缓存为进程内状态，清除不删除数据库历史记录。
            println!("cleared current session context cache (history records kept)");
            Ok(())
        }
        Some(ChatCommand::List) => {
            let db = DatabaseManager::new()?;
            let rows = list_sessions(&db)?;
            println!("{:<5}{:<28}{:<8}{:<8}{}", "id", "chat_uuid", "llm", "msgs", "title");
            if rows.is_empty() {
                println!("(no sessions)");
            }
            for (s, msgs) in rows {
                println!(
                    "{:<5}{:<28}{:<8}{:<8}{}",
                    s.id,
                    s.chat_uuid,
                    s.llm_id,
                    msgs,
                    s.title.as_deref().unwrap_or("-"),
                );
            }
            Ok(())
        }
        Some(ChatCommand::Rename { id, title }) => {
            let db = DatabaseManager::new()?;
            if rename_session(&db, id, &title)? {
                println!("renamed session #{} -> '{}'", id, title);
            } else {
                anyhow::bail!("session #{} not found", id);
            }
            Ok(())
        }
        Some(ChatCommand::Delete { id, yes }) => {
            let db = DatabaseManager::new()?;
            if !yes {
                println!(
                    "this will permanently delete session #{} and all its messages; re-run with -y to confirm",
                    id
                );
                return Ok(());
            }
            let (sessions, messages) = delete_session(&db, id)?;
            if sessions == 0 {
                anyhow::bail!("session #{} not found", id);
            }
            println!("deleted session #{} and {} message(s)", id, messages);
            Ok(())
        }
        None => {
            // 仅带 --chat-uuid 的场景由 run_with 拦截以启动 TUI 续跑；
            // 若直接分发到此，给出提示。
            match args.chat_uuid {
                Some(uuid) => {
                    println!("to resume session '{}' interactively, run without extra flags", uuid);
                    Ok(())
                }
                None => {
                    println!("usage: Omega chat --chat-uuid <uuid> | Omega chat <list|rename|delete|clear>");
                    Ok(())
                }
            }
        }
    }
}

// ============================================================
// memory / security / performance —— 策略类配置覆盖
// ============================================================

fn memory(args: MemoryArgs) -> Result<()> {
    let db = DatabaseManager::new()?;
    let value = args.mem_type.as_str();
    upsert_config(&db, "memory_type", value, "string", "CLI memory loading scope", "memory")?;
    println!("memory loading scope set to '{}'", value);
    Ok(())
}

fn security(args: SecurityArgs) -> Result<()> {
    let db = DatabaseManager::new()?;
    let value = args.mode.as_str();
    upsert_config(&db, "security_mode", value, "string", "CLI MCP security policy", "security")?;
    println!("MCP security policy set to '{}'", value);
    Ok(())
}

fn performance(args: PerformanceArgs) -> Result<()> {
    let db = DatabaseManager::new()?;
    let value = args.mode.as_str();
    upsert_config(&db, "performance_mode", value, "string", "CLI performance profile", "performance")?;
    println!("performance profile set to '{}'", value);
    Ok(())
}

// ============================================================
// arena
// ============================================================

async fn arena(action: ArenaCommand) -> Result<()> {
    let svc = ArenaService::new()?;
    match action {
        ArenaCommand::Vote {
            agent,
            score,
            comment,
            chat_uuid,
            round_uuid,
            acc,
            rel,
            flu,
            helpful,
        } => {
            // 收集非空多维度评分（短键 → 1-5）。
            let mut dimensions = BTreeMap::new();
            if let Some(v) = acc {
                dimensions.insert("acc".to_string(), v);
            }
            if let Some(v) = rel {
                dimensions.insert("rel".to_string(), v);
            }
            if let Some(v) = flu {
                dimensions.insert("flu".to_string(), v);
            }
            if let Some(v) = helpful {
                dimensions.insert("helpful".to_string(), v);
            }
            let dims_summary: Vec<String> = dimensions
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            let input = VoteInput {
                agent_id: agent,
                score,
                comment,
                chat_uuid: chat_uuid.unwrap_or_default(),
                round_uuid,
                dimensions,
            };
            let id = svc.record_vote(input)?;
            println!("recorded vote #{} for agent #{}: {}/5", id, agent, score);
            if !dims_summary.is_empty() {
                println!("dimensions: {}", dims_summary.join(", "));
            }
            // 展示重算后的平均分。
            let avg = svc
                .ranking()?
                .into_iter()
                .find(|r| r.agent_id == agent)
                .and_then(|r| r.avg_score);
            match avg {
                Some(a) => println!("agent #{} avg_score -> {:.1}", agent, a),
                None => println!("agent #{} avg_score -> -", agent),
            }
        }
        ArenaCommand::List => {
            let ranks = svc.ranking()?;
            if ranks.is_empty() {
                println!("(no agents)");
                return Ok(());
            }
            println!("{:<5}{:<24}{:<12}{:<8}", "id", "name", "avg_score", "votes");
            for r in ranks {
                let score = match r.avg_score {
                    Some(s) => format!("{:.1}", s),
                    None => "-".to_string(),
                };
                println!(
                    "{:<5}{:<24}{:<12}{:<8}",
                    r.agent_id, r.name, score, r.vote_count
                );
            }
        }
        ArenaCommand::Report { agent } => {
            // 聚合评估数据，复用被评 Agent 自身的供应商/模型配置生成报告。
            let data = svc.report_data(agent)?;
            let cfg = SessionStore::new()?.load_agent_config(agent)?;
            let provider_config = ProviderConfig {
                kind: parse_provider_kind(&cfg.provider_name).unwrap_or(ProviderKind::Compatible),
                api_key: cfg.api_key.clone().unwrap_or_default(),
                base_url: cfg.base_url.clone(),
                model: cfg.model_code.clone(),
            };
            let reporter =
                PromptReportAgent::new(provider_config, cfg.model_code.clone(), cfg.temperature);
            let text = reporter.generate(&data).await?;
            println!("{}", text);
            let path = report::save_report(agent, &text)?;
            println!("report saved to {}", path.display());
        }
    }
    Ok(())
}

// ============================================================
// prompt (§3.6 版本对比与回滚)
// ============================================================

fn prompt(action: PromptCommand) -> Result<()> {
    let db = DatabaseManager::new()?;
    match action {
        PromptCommand::List => {
            let mut rows = db.read_all::<PromptInfo>()?;
            // 按名称、再按可解析版本、最后按 id 排序。
            rows.sort_by(|a, b| {
                a.name
                    .cmp(&b.name)
                    .then_with(|| version_key(a).cmp(&version_key(b)))
                    .then(a.id.cmp(&b.id))
            });
            println!("{:<5}{:<24}{:<12}{:<8}{}", "id", "name", "version", "role", "category");
            if rows.is_empty() {
                println!("(no prompts)");
            }
            for p in rows {
                println!(
                    "{:<5}{:<24}{:<12}{:<8}{}",
                    p.id,
                    p.name,
                    p.version.as_deref().unwrap_or("-"),
                    p.role.as_deref().unwrap_or("-"),
                    p.category.as_deref().unwrap_or("-"),
                );
            }
        }
        PromptCommand::Show { id } => {
            let p = db
                .read_by_id::<PromptInfo>(id)?
                .with_context(|| format!("prompt #{} not found", id))?;
            println!("id          : {}", p.id);
            println!("name        : {}", p.name);
            println!("version     : {}", p.version.as_deref().unwrap_or("-"));
            println!("role        : {}", p.role.as_deref().unwrap_or("-"));
            println!("category    : {}", p.category.as_deref().unwrap_or("-"));
            println!("tags        : {}", p.tags.as_deref().unwrap_or("-"));
            println!("version_desc: {}", p.version_desc.as_deref().unwrap_or("-"));
            println!("---- content ----");
            println!("{}", p.content);
        }
        PromptCommand::Diff { from, to } => {
            let a = db
                .read_by_id::<PromptInfo>(from)?
                .with_context(|| format!("prompt #{} not found", from))?;
            let b = db
                .read_by_id::<PromptInfo>(to)?
                .with_context(|| format!("prompt #{} not found", to))?;
            println!(
                "diff: #{} {} ({}) -> #{} {} ({})",
                a.id,
                a.name,
                a.version.as_deref().unwrap_or("-"),
                b.id,
                b.name,
                b.version.as_deref().unwrap_or("-"),
            );
            for line in diff_lines(&a.content, &b.content) {
                println!("{}", line);
            }
        }
        PromptCommand::Rollback { id, desc } => {
            let target = db
                .read_by_id::<PromptInfo>(id)?
                .with_context(|| format!("prompt #{} not found", id))?;
            let (new_id, new_version) = rollback_prompt(&db, &target, desc)?;
            println!(
                "rolled back '{}' to content of #{} as new version {} (id #{})",
                target.name, target.id, new_version, new_id
            );
        }
        PromptCommand::Validate { id } => {
            let p = db
                .read_by_id::<PromptInfo>(id)?
                .with_context(|| format!("prompt #{} not found", id))?;
            // 已知键 = 该提示词的占位符 + 内置 {AgentList}/{MCPList}。
            let store = SessionStore::new()?;
            let placeholders = store.load_prompt_placeholders(id)?;
            let mut known: HashSet<String> =
                placeholders.iter().map(|(k, _, _)| k.clone()).collect();
            known.insert("AgentList".to_string());
            known.insert("MCPList".to_string());

            let report = validate_prompt(&p.content, &known);
            println!(
                "prompt #{} '{}' ({}) validation:",
                p.id,
                p.name,
                p.version.as_deref().unwrap_or("-")
            );
            println!(
                "  placeholders: {}",
                if report.keys.is_empty() {
                    "(none)".to_string()
                } else {
                    report.keys.join(", ")
                }
            );
            if report.is_ok() {
                println!("  result: OK");
            } else {
                if !report.unresolved.is_empty() {
                    println!("  unresolved: {}", report.unresolved.join(", "));
                }
                if !report.malformed.is_empty() {
                    println!("  malformed : {}", report.malformed.join(" "));
                }
                if report.unclosed {
                    println!("  unclosed  : yes");
                }
                println!("  result: FAILED");
            }
        }
        PromptCommand::Render { id, agent } => {
            let p = db
                .read_by_id::<PromptInfo>(id)?
                .with_context(|| format!("prompt #{} not found", id))?;
            let store = SessionStore::new()?;
            let placeholders = store.load_prompt_placeholders(id)?;
            // {AgentList}/{MCPList} 仅在指定 Agent 上下文时填充，否则为空。
            let (agents, tools) = match agent {
                Some(aid) => {
                    let agents = store.load_callable_agents(aid)?;
                    let tools = store
                        .load_agent_mcp_tools(aid)?
                        .into_iter()
                        .map(|t| t.name)
                        .collect();
                    (agents, tools)
                }
                None => (Vec::new(), Vec::new()),
            };
            let vars = build_vars(&agents, &tools, &placeholders);
            println!("{}", render(&p.content, &vars));
        }
    }
    Ok(())
}

/// 以 `target` 的内容生成同名新版本（追加，保留全部历史），返回（新 id，新版本号）。
///
/// 版本号取同名现有版本集合的最大可解析版本的补丁位 +1；不可解析则回退 `v1.0.0`。
fn rollback_prompt(
    db: &DatabaseManager,
    target: &PromptInfo,
    desc: Option<String>,
) -> Result<(i64, String)> {
    let siblings = db.find_where::<PromptInfo>("name = ?1", &[&target.name])?;
    let versions: Vec<String> = siblings.iter().filter_map(|p| p.version.clone()).collect();
    let new_version = next_version(&versions);
    let note = desc.unwrap_or_else(|| {
        format!(
            "rollback to #{} ({})",
            target.id,
            target.version.as_deref().unwrap_or("-")
        )
    });
    let new_id = db.create(PromptInfo {
        id: 0,
        name: target.name.clone(),
        content: target.content.clone(),
        description: target.description.clone(),
        category: target.category.clone(),
        role: target.role.clone(),
        version: Some(new_version.clone()),
        version_desc: Some(note),
        tags: target.tags.clone(),
        create_time: Utc::now(),
        update_time: Utc::now(),
        remark: Some("auto-rollback".to_string()),
    })?;
    Ok((new_id, new_version))
}

/// 解析形如 `v1.2.3` / `1.2.3` 的版本号为可比较三元组；无法解析返回 `None`。
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let s = v.trim().trim_start_matches(['v', 'V']);
    let mut parts = s.split('.');
    let major = parts.next()?.trim().parse::<u32>().ok()?;
    let minor = parts.next()?.trim().parse::<u32>().ok()?;
    let patch = parts.next()?.trim().parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// 排序键：可解析版本用其三元组，不可解析退化为 `(0,0,0)`（排在最前）。
fn version_key(p: &PromptInfo) -> (u32, u32, u32) {
    p.version.as_deref().and_then(parse_version).unwrap_or((0, 0, 0))
}

/// 基于现有版本集合计算下一个补丁版本号；集合为空或均不可解析时返回 `v1.0.0`。
fn next_version(existing: &[String]) -> String {
    match existing.iter().filter_map(|s| parse_version(s)).max() {
        None => "v1.0.0".to_string(),
        Some((ma, mi, pa)) => format!("v{}.{}.{}", ma, mi, pa + 1),
    }
}

/// 行级 diff（基于 LCS），返回带 `  `（未变）/`- `（删除）/`+ `（新增）前缀的行。
fn diff_lines(old: &str, new: &str) -> Vec<String> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let (n, m) = (a.len(), b.len());

    // dp[i][j] = a[i..] 与 b[j..] 的 LCS 长度。
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(format!("  {}", a[i]));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(format!("- {}", a[i]));
            i += 1;
        } else {
            out.push(format!("+ {}", b[j]));
            j += 1;
        }
    }
    while i < n {
        out.push(format!("- {}", a[i]));
        i += 1;
    }
    while j < m {
        out.push(format!("+ {}", b[j]));
        j += 1;
    }
    out
}

// ============================================================
// system_config helpers
// ============================================================

/// 写入（或更新）一个 `system_config` 键值。
fn upsert_config(
    db: &DatabaseManager,
    key: &str,
    value: &str,
    key_type: &str,
    description: &str,
    tag: &str,
) -> Result<()> {
    let mut existing = db.find_where::<SystemConfig>("key = ?1", &[&key])?;
    if let Some(cfg) = existing.first_mut() {
        cfg.value = value.to_string();
        db.update(cfg.clone())?;
    } else {
        let cfg = SystemConfig {
            key: key.to_string(),
            value: value.to_string(),
            key_type: Some(key_type.to_string()),
            description: Some(description.to_string()),
            tag: Some(tag.to_string()),
            sort: 0,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: None,
        };
        db.create(cfg)?;
    }
    Ok(())
}

/// 读取一个 `system_config` 键值（不存在返回 `None`）。
fn read_config(db: &DatabaseManager, key: &str) -> Result<Option<String>> {
    let found = db.find_where::<SystemConfig>("key = ?1", &[&key])?;
    Ok(found.into_iter().next().map(|c| c.value))
}

// ============================================================
// model auto-sync (§3.4)
// ============================================================

/// 将供应商返回的模型标识同步入 `llm_model`（按 `provider_id + model_code` 幂等）。
///
/// 仅新增缺失项，已存在的模型保持不变（模型基础参数不支持自动覆写）。
/// 返回（新增数，已存在跳过数）；空白标识被忽略。
fn sync_models(db: &DatabaseManager, provider_id: i32, codes: &[String]) -> Result<(usize, usize)> {
    let mut added = 0usize;
    let mut skipped = 0usize;
    for raw in codes {
        let code = raw.trim();
        if code.is_empty() {
            continue;
        }
        let code = code.to_string();
        let existing = db.find_where::<LLMModel>(
            "provider_id = ?1 AND model_code = ?2",
            &[&provider_id, &code],
        )?;
        if existing.first().is_some() {
            skipped += 1;
            continue;
        }
        db.create(LLMModel {
            id: 0,
            provider_id,
            avatar: None,
            name: code.clone(),
            model_code: code,
            max_tokens: 0,
            max_response_tokens: 0,
            weight: 0,
            call_count: 0,
            status: 1,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: Some("auto-synced".to_string()),
        })?;
        added += 1;
    }
    Ok((added, skipped))
}

/// `model delete` 的前置校验结果。
enum ModelDeletable {
    /// 模型不存在。
    NotFound,
    /// 被 `refs` 个 Agent 引用，不可删除。
    Blocked { refs: usize },
    /// 可删除，附带待删模型行。
    Ready(LLMModel),
}

/// 校验模型是否可删除：不存在 / 被 Agent 引用 / 可删除。
fn check_model_deletable(db: &DatabaseManager, code: &str) -> Result<ModelDeletable> {
    let found = db.find_where::<LLMModel>("model_code = ?1", &[&code])?;
    let Some(m) = found.into_iter().next() else {
        return Ok(ModelDeletable::NotFound);
    };
    let refs = db.find_where::<AgentInfo>("llm_id = ?1", &[&m.id])?.len();
    if refs > 0 {
        Ok(ModelDeletable::Blocked { refs })
    } else {
        Ok(ModelDeletable::Ready(m))
    }
}

// ============================================================
// chat session helpers (§3.9)
// ============================================================

/// 列出所有会话及各自消息数（按 id 升序）。
fn list_sessions(db: &DatabaseManager) -> Result<Vec<(ChatSession, usize)>> {
    let mut sessions = db.read_all::<ChatSession>()?;
    sessions.sort_by_key(|s| s.id);

    // 一次性统计每个 chat_uuid 的消息数。
    let mut stmt = db
        .conn
        .prepare("SELECT chat_uuid, COUNT(*) FROM chat_message GROUP BY chat_uuid")?;
    let counts: HashMap<String, usize> = stmt
        .query_map(params![], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions
        .into_iter()
        .map(|s| {
            let msgs = counts.get(&s.chat_uuid).copied().unwrap_or(0);
            (s, msgs)
        })
        .collect())
}

/// 重命名会话标题；返回是否找到并更新。
fn rename_session(db: &DatabaseManager, id: i32, title: &str) -> Result<bool> {
    let Some(mut s) = db.read_by_id::<ChatSession>(id)? else {
        return Ok(false);
    };
    s.title = Some(title.to_string());
    db.update(s)?;
    Ok(true)
}

/// 物理删除会话及其全部消息；返回（删除的会话数，删除的消息数）。
fn delete_session(db: &DatabaseManager, id: i32) -> Result<(usize, usize)> {
    let Some(s) = db.read_by_id::<ChatSession>(id)? else {
        return Ok((0, 0));
    };
    let messages = db
        .conn
        .execute("DELETE FROM chat_message WHERE chat_uuid = ?1", params![s.chat_uuid])?;
    let sessions = db
        .conn
        .execute("DELETE FROM chat_session WHERE id = ?1", params![id])?;
    Ok((sessions, messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Model;
    use crate::db::models::chat_message::ChatMessage;

    /// 构造仅含所需表的内存库，验证 upsert/read 配置往返。
    fn temp_db() -> DatabaseManager {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(SystemConfig::create_table_sql()).unwrap();
        // DatabaseManager::new 会打开磁盘库；测试直接用已有连接构造。
        DatabaseManager { conn }
    }

    #[test]
    fn upsert_then_read_config_roundtrip() {
        let db = temp_db();
        assert_eq!(read_config(&db, "security_mode").unwrap(), None);

        upsert_config(&db, "security_mode", "strict", "string", "d", "security").unwrap();
        assert_eq!(read_config(&db, "security_mode").unwrap().as_deref(), Some("strict"));

        // 再次写入应更新而非重复插入。
        upsert_config(&db, "security_mode", "bypass", "string", "d", "security").unwrap();
        assert_eq!(read_config(&db, "security_mode").unwrap().as_deref(), Some("bypass"));
        let all = db.read_all::<SystemConfig>().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn sync_models_inserts_new_and_is_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(LLMModel::create_table_sql()).unwrap();
        let db = DatabaseManager { conn };

        let codes = vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()];
        assert_eq!(sync_models(&db, 1, &codes).unwrap(), (2, 0));
        // 再次同步应全部跳过（幂等）。
        assert_eq!(sync_models(&db, 1, &codes).unwrap(), (0, 2));
        // 不同 provider_id 视为不同模型。
        assert_eq!(sync_models(&db, 2, &codes).unwrap(), (2, 0));
        // 空白项被忽略。
        assert_eq!(sync_models(&db, 1, &["   ".to_string()]).unwrap(), (0, 0));

        let all = db.read_all::<LLMModel>().unwrap();
        assert_eq!(all.len(), 4);
        let synced = all.iter().find(|m| m.provider_id == 1 && m.model_code == "gpt-4o").unwrap();
        assert_eq!(synced.name, "gpt-4o");
        assert_eq!(synced.status, 1);
        assert_eq!(synced.remark.as_deref(), Some("auto-synced"));
    }

    fn make_prompt(name: &str, version: &str, content: &str) -> PromptInfo {
        PromptInfo {
            id: 0,
            name: name.to_string(),
            content: content.to_string(),
            description: None,
            category: None,
            role: Some("System".to_string()),
            version: Some(version.to_string()),
            version_desc: None,
            tags: None,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: None,
        }
    }

    #[test]
    fn parse_version_handles_prefix_and_garbage() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("v0.9"), None);
        assert_eq!(parse_version("nightly"), None);
    }

    #[test]
    fn next_version_bumps_patch_of_max() {
        assert_eq!(next_version(&[]), "v1.0.0");
        assert_eq!(next_version(&["v1.0.0".to_string()]), "v1.0.1");
        assert_eq!(
            next_version(&[
                "v1.0.0".to_string(),
                "v1.2.9".to_string(),
                "v1.1.0".to_string()
            ]),
            "v1.2.10"
        );
        // 均不可解析时回退。
        assert_eq!(next_version(&["nightly".to_string()]), "v1.0.0");
    }

    #[test]
    fn diff_lines_marks_context_add_remove() {
        let out = diff_lines("a\nb\nc", "a\nB\nc\nd");
        assert!(out.contains(&"  a".to_string()));
        assert!(out.contains(&"- b".to_string()));
        assert!(out.contains(&"+ B".to_string()));
        assert!(out.contains(&"  c".to_string()));
        assert!(out.contains(&"+ d".to_string()));
        // 无差异时全为上下文行。
        let same = diff_lines("x\ny", "x\ny");
        assert_eq!(same, vec!["  x".to_string(), "  y".to_string()]);
    }

    #[test]
    fn rollback_prompt_appends_new_version_with_target_content() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(PromptInfo::create_table_sql()).unwrap();
        let db = DatabaseManager { conn };
        db.create(make_prompt("coder", "v1.0.0", "old content")).unwrap();
        db.create(make_prompt("coder", "v1.1.0", "new content")).unwrap();

        // 回滚到 v1.0.0（id #1）。
        let target = db.read_by_id::<PromptInfo>(1).unwrap().unwrap();
        let (new_id, new_version) = rollback_prompt(&db, &target, None).unwrap();
        assert_eq!(new_version, "v1.1.1");

        let created = db.read_by_id::<PromptInfo>(new_id as i32).unwrap().unwrap();
        assert_eq!(created.content, "old content");
        assert_eq!(created.name, "coder");
        assert!(created.version_desc.unwrap().contains("rollback"));
        assert_eq!(created.remark.as_deref(), Some("auto-rollback"));
        // 历史保留：共 3 行。
        assert_eq!(db.read_all::<PromptInfo>().unwrap().len(), 3);
    }

    #[test]
    fn rollback_prompt_uses_custom_desc() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(PromptInfo::create_table_sql()).unwrap();
        let db = DatabaseManager { conn };
        db.create(make_prompt("writer", "v2.0.0", "body")).unwrap();
        let target = db.read_by_id::<PromptInfo>(1).unwrap().unwrap();
        let (_, ver) = rollback_prompt(&db, &target, Some("revert bug".to_string())).unwrap();
        assert_eq!(ver, "v2.0.1");
        let created = db.read_by_id::<PromptInfo>(2).unwrap().unwrap();
        assert_eq!(created.version_desc.as_deref(), Some("revert bug"));
    }

    fn make_session(uuid: &str, title: Option<&str>, llm_id: i32) -> ChatSession {
        ChatSession {
            id: 0,
            chat_uuid: uuid.to_string(),
            title: title.map(|t| t.to_string()),
            llm_id,
            is_deleted: 0,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: None,
        }
    }

    fn make_message(uuid: &str) -> ChatMessage {
        ChatMessage {
            id: 0,
            chat_uuid: uuid.to_string(),
            round_uuid: None,
            role: "user".to_string(),
            content: "hi".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            duration: 0,
            model_config: None,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: None,
        }
    }

    fn make_model(code: &str) -> LLMModel {
        LLMModel {
            id: 0,
            provider_id: 1,
            avatar: None,
            name: code.to_string(),
            model_code: code.to_string(),
            max_tokens: 0,
            max_response_tokens: 0,
            weight: 0,
            call_count: 0,
            status: 1,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark: None,
        }
    }

    fn session_db() -> DatabaseManager {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(ChatSession::create_table_sql()).unwrap();
        conn.execute_batch(ChatMessage::create_table_sql()).unwrap();
        DatabaseManager { conn }
    }

    #[test]
    fn list_sessions_counts_messages_per_uuid() {
        let db = session_db();
        db.create(make_session("u1", Some("first"), 1)).unwrap();
        db.create(make_session("u2", None, 2)).unwrap();
        // u1 两条消息，u2 一条。
        db.create(make_message("u1")).unwrap();
        db.create(make_message("u1")).unwrap();
        db.create(make_message("u2")).unwrap();

        let rows = list_sessions(&db).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.chat_uuid, "u1");
        assert_eq!(rows[0].1, 2);
        assert_eq!(rows[1].0.chat_uuid, "u2");
        assert_eq!(rows[1].1, 1);
    }

    #[test]
    fn rename_session_updates_title_or_reports_missing() {
        let db = session_db();
        db.create(make_session("u1", None, 1)).unwrap();
        assert!(rename_session(&db, 1, "planning").unwrap());
        assert_eq!(
            db.read_by_id::<ChatSession>(1).unwrap().unwrap().title.as_deref(),
            Some("planning")
        );
        // 不存在的会话。
        assert!(!rename_session(&db, 99, "x").unwrap());
    }

    #[test]
    fn delete_session_removes_rows_and_messages() {
        let db = session_db();
        db.create(make_session("u1", None, 1)).unwrap();
        db.create(make_session("u2", None, 1)).unwrap();
        db.create(make_message("u1")).unwrap();
        db.create(make_message("u1")).unwrap();

        assert_eq!(delete_session(&db, 1).unwrap(), (1, 2));
        // u1 已删，u2 保留。
        assert!(db.read_by_id::<ChatSession>(1).unwrap().is_none());
        assert!(db.read_by_id::<ChatSession>(2).unwrap().is_some());
        // 再删不存在的会话返回 (0,0)。
        assert_eq!(delete_session(&db, 1).unwrap(), (0, 0));
    }

    #[test]
    fn check_model_deletable_guards_references() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(LLMModel::create_table_sql()).unwrap();
        conn.execute_batch(AgentInfo::create_table_sql()).unwrap();
        let db = DatabaseManager { conn };
        // #1 used 被 Agent 引用，#2 free 无引用。
        db.create(make_model("used")).unwrap();
        db.create(make_model("free")).unwrap();
        db.conn
            .execute(
                "INSERT INTO agent_info (id, llm_id, name, status) VALUES (1, 1, 'A', 1)",
                params![],
            )
            .unwrap();

        assert!(matches!(
            check_model_deletable(&db, "missing").unwrap(),
            ModelDeletable::NotFound
        ));
        assert!(matches!(
            check_model_deletable(&db, "used").unwrap(),
            ModelDeletable::Blocked { refs: 1 }
        ));
        match check_model_deletable(&db, "free").unwrap() {
            ModelDeletable::Ready(m) => assert_eq!(m.model_code, "free"),
            _ => panic!("expected Ready"),
        }
    }
}
