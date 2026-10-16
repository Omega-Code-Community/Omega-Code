//! 多 Agent 调度器（Coordinator / Tool / Worker 三层架构）
//!
//! 对应《调度器设计与实现》：Coordinator 作为「顶层协调者」负责拆分任务、调度 Worker
//! 子代理并行执行、汇总结果；本模块提供支撑调度层的四个工具（创建 / 续跑 / 停止 / 等待）
//! 以及 Worker 生命周期管理（进程内内存池）。
//!
//! 设计要点：
//! - **解耦**：调度层不直接依赖 [`Provider`](crate::core::providers::Provider) / `AiClient`，
//!   而是经 [`WorkerContext`] / [`WorkerFactory`] 抽象驱动 Worker。生产环境用真实 [`Agent`]
//!   实现（[`AgentWorkerFactory`]），测试用 echo 桩实现，从而调度逻辑可独立单测。
//! - **并发**：`create_worker` 后台异步起 Worker 立即返回 `worker_id`，`await_workers`
//!   阻塞收集标准化结果，实现多 Worker 并行。
//! - **可追溯**：Worker 各自建立独立会话（经 `chat_session`/`chat_message` 持久化），
//!   进程内池保留上下文以支持续跑（`resume_worker`）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

use crate::core::agent::{Agent, AgentEvent};
use crate::core::providers::{create_provider, ProviderConfig};
use crate::core::runtime::Runtime;
use crate::core::tools::{Tool, ToolRegistry};

/// `await_workers` 轮询间隔。
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// `await_workers` 总超时（防止个别 Worker 卡死导致协调者永久阻塞）。
pub const AWAIT_TIMEOUT: Duration = Duration::from_secs(600);

// ============================================================
// 类型
// ============================================================

/// Worker 执行状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus {
    /// 运行中。
    Running,
    /// 已成功完成。
    Completed,
    /// 执行失败。
    Failed,
    /// 被协调者中止。
    Stopped,
}

impl WorkerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Running => "running",
            WorkerStatus::Completed => "completed",
            WorkerStatus::Failed => "failed",
            WorkerStatus::Stopped => "stopped",
        }
    }

    /// 是否处于终止态（不再变化，`await_workers` 可停止等待）。
    pub fn is_terminal(&self) -> bool {
        !matches!(self, WorkerStatus::Running)
    }
}

/// Worker 上报的标准化结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerReport {
    /// Worker 唯一标识。
    pub worker_id: String,
    /// 任务标识（取自 Worker 上报，缺省回退 `worker_id`）。
    pub task_id: String,
    /// 业务状态："success" / "failed"。
    pub status: String,
    /// 结果摘要。
    pub summary: String,
    /// 具体产出（文件路径 / commit 哈希 / 关键发现等）。
    pub artifacts: Vec<String>,
}

/// Worker 执行上下文抽象：执行一条自包含指令并返回最终产出文本。
///
/// 实现内部保留历史，以支持 [`ResumeWorkerTool`] 复用上下文续跑。
#[async_trait]
pub trait WorkerContext: Send + Sync {
    /// 执行一条指令，返回 Worker 的最终产出文本（应含标准化 `result` 块）。
    async fn run_instruction(&self, instruction: String) -> Result<String>;
}

/// Worker 工厂：按子任务类型创建一个独立的 [`WorkerContext`]。
pub trait WorkerFactory: Send + Sync {
    fn create(&self, task_type: &str) -> Arc<dyn WorkerContext>;
}

/// 池中一个 Worker 的运行态句柄。
struct WorkerHandle {
    status: WorkerStatus,
    ctx: Arc<dyn WorkerContext>,
    report: Option<WorkerReport>,
    abort: Option<AbortHandle>,
}

/// Worker 池：进程内内存态，管理所有 Worker 的生命周期与结果。
///
/// 内部用 [`std::sync::Mutex`] 保护，临界区短小、**不跨 `await` 持有**。
#[derive(Default)]
pub struct WorkerPool {
    inner: std::sync::Mutex<HashMap<String, WorkerHandle>>,
}

impl WorkerPool {
    fn guard(&self) -> std::sync::MutexGuard<'_, HashMap<String, WorkerHandle>> {
        // 中毒时恢复内部数据，避免因单次 panic 使整个池不可用。
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 登记一个新 Worker（初始为 Running）。
    pub fn insert(&self, id: String, ctx: Arc<dyn WorkerContext>) {
        self.guard().insert(
            id,
            WorkerHandle {
                status: WorkerStatus::Running,
                ctx,
                report: None,
                abort: None,
            },
        );
    }

    /// 回填后台任务的 abort 句柄（供 `stop_worker` 中止）。
    pub fn set_abort(&self, id: &str, abort: AbortHandle) {
        if let Some(h) = self.guard().get_mut(id) {
            h.abort = Some(abort);
        }
    }

    /// 标记 Worker 终止并写入报告。
    pub fn finish(&self, id: &str, status: WorkerStatus, report: Option<WorkerReport>) {
        if let Some(h) = self.guard().get_mut(id) {
            h.status = status;
            h.report = report;
            h.abort = None;
        }
    }

    /// 置回 Running（续跑前调用），清空旧报告。
    pub fn set_running(&self, id: &str) {
        if let Some(h) = self.guard().get_mut(id) {
            h.status = WorkerStatus::Running;
            h.report = None;
        }
    }

    /// 中止 Worker：触发 abort 并置为 Stopped；保留 ctx 以便后续续跑。
    /// 返回该 Worker 是否存在。
    pub fn stop(&self, id: &str) -> bool {
        let mut g = self.guard();
        match g.get_mut(id) {
            Some(h) => {
                if let Some(a) = h.abort.take() {
                    a.abort();
                }
                h.status = WorkerStatus::Stopped;
                true
            }
            None => false,
        }
    }

    /// 取出 Worker 的执行上下文（用于续跑，复用历史）。
    pub fn get_ctx(&self, id: &str) -> Option<Arc<dyn WorkerContext>> {
        self.guard().get(id).map(|h| h.ctx.clone())
    }

    /// 当前所有 Worker 的 id。
    pub fn ids(&self) -> Vec<String> {
        self.guard().keys().cloned().collect()
    }

    /// 指定的所有 id 是否均已终止（不存在的 id 视为无需等待）。
    pub fn all_terminal(&self, ids: &[String]) -> bool {
        let g = self.guard();
        ids.iter().all(|id| match g.get(id) {
            Some(h) => h.status.is_terminal(),
            None => true,
        })
    }

    /// 收集指定 id 的状态与报告快照（跳过不存在的 id）。
    pub fn snapshot(&self, ids: &[String]) -> Vec<(String, WorkerStatus, Option<WorkerReport>)> {
        let g = self.guard();
        ids.iter()
            .filter_map(|id| {
                g.get(id)
                    .map(|h| (id.clone(), h.status.clone(), h.report.clone()))
            })
            .collect()
    }
}

// ============================================================
// 生产实现：基于真实 Agent 的 Worker
// ============================================================

/// 生产环境的 Worker 工厂：用协调者的供应商/模型配置构造独立 [`Agent`]。
pub struct AgentWorkerFactory {
    /// 供应商配置（复用协调者 Agent）。
    pub provider_config: ProviderConfig,
    /// 工作目录（Workspace 路径）。
    pub working_dir: PathBuf,
    /// 采样温度。
    pub temperature: Option<f32>,
    /// 核采样 top_p。
    pub top_p: Option<f32>,
    /// 单次输出上限。
    pub max_tokens: Option<u32>,
}

impl AgentWorkerFactory {
    fn build_agent(&self, task_type: &str) -> Result<Agent> {
        let provider = create_provider(self.provider_config.clone())?;
        let runtime = Arc::new(Runtime::new(self.working_dir.clone()));
        let tools = ToolRegistry::with_defaults(runtime);

        let key = if self.provider_config.api_key.is_empty() {
            None
        } else {
            Some(self.provider_config.api_key.clone())
        };

        let mut agent = Agent::new(
            provider,
            self.provider_config.model.clone(),
            WORKER_PROMPT.to_string(),
            tools,
        )
        .with_provider_meta(
            self.provider_config.kind.as_str(),
            self.provider_config.base_url.clone(),
            key,
        )
        .with_sampling(self.temperature, self.top_p, self.max_tokens);

        // 每个 Worker 建立独立会话，便于复盘（尽力而为，失败仅告警）。
        if let Err(e) = agent.enable_persistence_auto(Some(task_type)) {
            log::warn!("worker persistence init failed: {e}");
        }
        Ok(agent)
    }
}

impl WorkerFactory for AgentWorkerFactory {
    fn create(&self, task_type: &str) -> Arc<dyn WorkerContext> {
        match self.build_agent(task_type) {
            Ok(agent) => Arc::new(AgentWorkerContext {
                agent: tokio::sync::Mutex::new(agent),
            }),
            // 构造失败不 panic：返回一个运行期报错的上下文，交由调度层标记 Failed。
            Err(e) => Arc::new(FailedWorkerContext {
                error: e.to_string(),
            }),
        }
    }
}

/// 基于 [`Agent`] 的 Worker 上下文。
pub struct AgentWorkerContext {
    agent: tokio::sync::Mutex<Agent>,
}

#[async_trait]
impl WorkerContext for AgentWorkerContext {
    async fn run_instruction(&self, instruction: String) -> Result<String> {
        let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();
        let mut agent = self.agent.lock().await;
        // 无界通道不会阻塞发送：先驱动整轮，再收集累积的助手文本。
        agent.run_turn(&instruction, tx).await?;
        let mut acc = String::new();
        while let Some(ev) = rx.recv().await {
            if let AgentEvent::Token(t) = ev {
                acc.push_str(&t);
            }
        }
        Ok(acc)
    }
}

/// 构造失败时的兜底上下文：运行即报错。
struct FailedWorkerContext {
    error: String,
}

#[async_trait]
impl WorkerContext for FailedWorkerContext {
    async fn run_instruction(&self, _instruction: String) -> Result<String> {
        anyhow::bail!("worker unavailable: {}", self.error)
    }
}

// ============================================================
// 提示词
// ============================================================

/// 追加到协调者 Agent 系统提示词后的调度器说明。
pub const COORDINATOR_PROMPT: &str = "\n\n# Coordinator Role (Multi-Agent Scheduler)\n\
You are the top-level Coordinator. You DO NOT execute concrete tasks yourself; you decompose the \
user's software-engineering request into self-contained subtasks, orchestrate Worker sub-agents \
through the scheduler tools, and aggregate their results.\n\n\
Pipeline: research -> implement -> verify.\n\n\
Scheduler tools:\n\
- `create_worker` {instruction, task_type?}: start a Worker asynchronously; returns worker_id \
immediately. Use it for parallelizable work.\n\
- `await_workers` {ids?}: block until the given (or all) workers finish; returns their standardized \
reports. ALWAYS await before aggregating.\n\
- `resume_worker` {worker_id, instruction}: give an existing worker a new instruction, reusing its \
context (use for fixes after a failure).\n\
- `stop_worker` {worker_id}: terminate a misguided worker (it can be resumed later).\n\n\
Rules:\n\
- Instructions MUST be self-contained: include file paths, exact operations, and completion \
criteria. Workers cannot see this conversation.\n\
- Parallelism: read-only tasks (research/query) run in parallel; write tasks are serialized per \
file (never let two workers edit the same file concurrently).\n\
- Aggregation is YOUR job: never ask a worker to inspect another worker's output. Wait for all \
parallel subtasks via `await_workers`, then integrate the reports yourself.\n\
- On worker failure, prefer `resume_worker` with a corrective instruction reusing its error \
context; after repeated failures, stop and report to the user.\n\
- Trivial actions (reading one file, running one command) do not need a worker.\n\n\
Each worker ends with a fenced ```result JSON block: \
{\"task_id\",\"status\":\"success|failed\",\"summary\",\"artifacts\":[...]}. `await_workers` returns \
these reports for you to summarize.\n";

/// Worker 子代理的系统提示词。
const WORKER_PROMPT: &str = "You are a Worker sub-agent in a multi-agent scheduler. You receive a \
single self-contained instruction and must complete it independently using the available file \
tools (read_file/write_file/list_dir) and any mounted MCP tools. You have NO access to the \
coordinator's or the user's conversation; rely only on the instruction.\n\n\
Do the work requested, then END your reply with exactly one fenced code block tagged `result` \
containing JSON:\n\
```result\n\
{\"task_id\": \"<echo the task id from the instruction, or your worker id>\", \"status\": \
\"success\", \"summary\": \"<concise outcome>\", \"artifacts\": [\"<file paths / commit hashes / \
key findings>\"]}\n\
```\n\
If you cannot complete the instruction, set status to \"failed\" and explain why in summary. Do \
not fabricate results.";

// ============================================================
// 结果解析
// ============================================================

/// 从 Worker 产出文本解析标准化报告（取最后一个 ```result 围栏块）。
///
/// 无有效围栏块时回退：`status="success"`、`summary` 为文本截断（前 500 字）。
pub fn parse_worker_report(worker_id: &str, text: &str) -> WorkerReport {
    const MARKER: &str = "```result";
    if let Some(start) = text.rfind(MARKER) {
        let after = &text[start + MARKER.len()..];
        if let Some(end) = after.find("```") {
            let json_str = after[..end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                let raw_status = v
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("success");
                let status = if raw_status == "failed" {
                    "failed".to_string()
                } else {
                    "success".to_string()
                };
                return WorkerReport {
                    worker_id: worker_id.to_string(),
                    task_id: v
                        .get("task_id")
                        .and_then(|s| s.as_str())
                        .unwrap_or(worker_id)
                        .to_string(),
                    status,
                    summary: v
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    artifacts: v
                        .get("artifacts")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                };
            }
        }
    }

    let summary: String = text.chars().take(500).collect();
    WorkerReport {
        worker_id: worker_id.to_string(),
        task_id: worker_id.to_string(),
        status: "success".to_string(),
        summary,
        artifacts: vec![],
    }
}

/// 生成一个 Worker 唯一标识。
fn new_worker_id() -> String {
    use rand::RngExt;
    let n: u32 = rand::rng().random_range(0..u32::MAX);
    format!("w-{:08x}", n)
}

/// 后台运行一个 Worker 指令并把结果写回池（`create_worker`/`resume_worker` 共用）。
async fn run_and_finish(
    pool: Arc<WorkerPool>,
    worker_id: String,
    ctx: Arc<dyn WorkerContext>,
    instruction: String,
) {
    let outcome = ctx.run_instruction(instruction).await;
    let (status, report) = match outcome {
        Ok(text) => {
            let report = parse_worker_report(&worker_id, &text);
            let status = if report.status == "failed" {
                WorkerStatus::Failed
            } else {
                WorkerStatus::Completed
            };
            (status, Some(report))
        }
        Err(e) => (
            WorkerStatus::Failed,
            Some(WorkerReport {
                worker_id: worker_id.clone(),
                task_id: worker_id.clone(),
                status: "failed".to_string(),
                summary: format!("worker execution error: {e}"),
                artifacts: vec![],
            }),
        ),
    };
    pool.finish(&worker_id, status, report);
}

// ============================================================
// 调度工具
// ============================================================

/// `create_worker`：异步启动一个新 Worker，立即返回 `worker_id`。
pub struct CreateWorkerTool {
    pool: Arc<WorkerPool>,
    factory: Arc<dyn WorkerFactory>,
}

#[async_trait]
impl Tool for CreateWorkerTool {
    fn name(&self) -> &str {
        "create_worker"
    }

    fn description(&self) -> &str {
        "Start a new Worker sub-agent asynchronously to execute a self-contained instruction. \
         Returns a worker_id immediately (does not block). Args: {\"instruction\": string, \
         \"task_type\"?: string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "instruction": { "type": "string", "description": "Self-contained subtask instruction (file paths, operations, completion criteria)" },
                "task_type": { "type": "string", "description": "Subtask type: research / implement / verify / generic" }
            },
            "required": ["instruction"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let instruction = args
            .get("instruction")
            .and_then(|v| v.as_str())
            .context("create_worker requires an 'instruction' string argument")?
            .to_string();
        let task_type = args
            .get("task_type")
            .and_then(|v| v.as_str())
            .unwrap_or("generic");

        let worker_id = new_worker_id();
        let ctx = self.factory.create(task_type);
        self.pool.insert(worker_id.clone(), ctx.clone());

        let join = tokio::spawn(run_and_finish(
            self.pool.clone(),
            worker_id.clone(),
            ctx,
            instruction,
        ));
        self.pool.set_abort(&worker_id, join.abort_handle());

        Ok(json!({ "worker_id": worker_id, "status": "running" }).to_string())
    }
}

/// `await_workers`：阻塞等待指定（或全部）Worker 终止并汇总报告。
pub struct AwaitWorkersTool {
    pool: Arc<WorkerPool>,
}

#[async_trait]
impl Tool for AwaitWorkersTool {
    fn name(&self) -> &str {
        "await_workers"
    }

    fn description(&self) -> &str {
        "Block until the given workers (or all workers when 'ids' is omitted) reach a terminal \
         state, then return their aggregated reports. Args: {\"ids\"?: string[]}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ids": { "type": "array", "items": { "type": "string" }, "description": "Worker ids to await; omit to await all" }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let requested: Vec<String> = args
            .get("ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let targets = if requested.is_empty() {
            self.pool.ids()
        } else {
            requested
        };

        if targets.is_empty() {
            return Ok(json!({ "workers": [], "complete": true, "note": "no workers to await" })
                .to_string());
        }

        let deadline = Instant::now() + AWAIT_TIMEOUT;
        loop {
            if self.pool.all_terminal(&targets) || Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }

        let complete = self.pool.all_terminal(&targets);
        let workers: Vec<Value> = self
            .pool
            .snapshot(&targets)
            .into_iter()
            .map(|(id, status, report)| {
                json!({
                    "worker_id": id,
                    "status": status.as_str(),
                    "report": report.map(|r| serde_json::to_value(r).unwrap_or(Value::Null)).unwrap_or(Value::Null),
                })
            })
            .collect();

        Ok(json!({ "workers": workers, "complete": complete }).to_string())
    }
}

/// `resume_worker`：给已有 Worker 下发新指令，复用其上下文续跑。
pub struct ResumeWorkerTool {
    pool: Arc<WorkerPool>,
}

#[async_trait]
impl Tool for ResumeWorkerTool {
    fn name(&self) -> &str {
        "resume_worker"
    }

    fn description(&self) -> &str {
        "Send a new instruction to an existing worker, reusing its prior context (history/errors). \
         Runs asynchronously. Args: {\"worker_id\": string, \"instruction\": string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "worker_id": { "type": "string", "description": "Target worker id" },
                "instruction": { "type": "string", "description": "New self-contained instruction (e.g. a corrective fix)" }
            },
            "required": ["worker_id", "instruction"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let worker_id = args
            .get("worker_id")
            .and_then(|v| v.as_str())
            .context("resume_worker requires a 'worker_id' string argument")?
            .to_string();
        let instruction = args
            .get("instruction")
            .and_then(|v| v.as_str())
            .context("resume_worker requires an 'instruction' string argument")?
            .to_string();

        let ctx = self
            .pool
            .get_ctx(&worker_id)
            .with_context(|| format!("worker '{}' not found", worker_id))?;

        self.pool.set_running(&worker_id);
        let join = tokio::spawn(run_and_finish(
            self.pool.clone(),
            worker_id.clone(),
            ctx,
            instruction,
        ));
        self.pool.set_abort(&worker_id, join.abort_handle());

        Ok(json!({ "worker_id": worker_id, "status": "resumed" }).to_string())
    }
}

/// `stop_worker`：中止一个 Worker（保留上下文，可后续续跑）。
pub struct StopWorkerTool {
    pool: Arc<WorkerPool>,
}

#[async_trait]
impl Tool for StopWorkerTool {
    fn name(&self) -> &str {
        "stop_worker"
    }

    fn description(&self) -> &str {
        "Terminate a running worker (e.g. wrong direction). The worker can be resumed later. \
         Args: {\"worker_id\": string}"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "worker_id": { "type": "string", "description": "Target worker id" }
            },
            "required": ["worker_id"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let worker_id = args
            .get("worker_id")
            .and_then(|v| v.as_str())
            .context("stop_worker requires a 'worker_id' string argument")?
            .to_string();

        if !self.pool.stop(&worker_id) {
            anyhow::bail!("worker '{}' not found", worker_id);
        }
        Ok(json!({ "worker_id": worker_id, "status": "stopped" }).to_string())
    }
}

/// 构建四个调度工具（共享同一个 [`WorkerPool`] 与 [`WorkerFactory`]）。
pub fn build_tools(
    pool: Arc<WorkerPool>,
    factory: Arc<dyn WorkerFactory>,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(CreateWorkerTool {
            pool: pool.clone(),
            factory,
        }),
        Arc::new(AwaitWorkersTool { pool: pool.clone() }),
        Arc::new(ResumeWorkerTool { pool: pool.clone() }),
        Arc::new(StopWorkerTool { pool }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// echo 桩上下文：记录收到的指令，按配置返回成功/失败的标准化结果。
    struct EchoWorkerContext {
        fail: bool,
        delay: Duration,
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl WorkerContext for EchoWorkerContext {
        async fn run_instruction(&self, instruction: String) -> Result<String> {
            self.calls.lock().unwrap().push(instruction.clone());
            tokio::time::sleep(self.delay).await;
            if self.fail {
                Ok("```result\n{\"status\":\"failed\",\"summary\":\"boom\"}\n```".to_string())
            } else {
                Ok(format!(
                    "```result\n{{\"task_id\":\"t1\",\"status\":\"success\",\"summary\":\"done: {}\",\"artifacts\":[\"a.txt\"]}}\n```",
                    instruction
                ))
            }
        }
    }

    struct EchoFactory {
        fail: bool,
        delay: Duration,
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl WorkerFactory for EchoFactory {
        fn create(&self, _task_type: &str) -> Arc<dyn WorkerContext> {
            Arc::new(EchoWorkerContext {
                fail: self.fail,
                delay: self.delay,
                calls: self.calls.clone(),
            })
        }
    }

    fn setup(fail: bool, delay: Duration) -> (Arc<WorkerPool>, Vec<Arc<dyn Tool>>, Arc<std::sync::Mutex<Vec<String>>>) {
        let pool = Arc::new(WorkerPool::default());
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let factory = Arc::new(EchoFactory {
            fail,
            delay,
            calls: calls.clone(),
        });
        let tools = build_tools(pool.clone(), factory);
        (pool, tools, calls)
    }

    fn tool<'a>(tools: &'a [Arc<dyn Tool>], name: &str) -> &'a Arc<dyn Tool> {
        tools.iter().find(|t| t.name() == name).expect("tool exists")
    }

    fn wid_of(create_out: &str) -> String {
        serde_json::from_str::<Value>(create_out)
            .unwrap()
            .get("worker_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn build_tools_registers_four_schedulers() {
        let (_pool, tools, _calls) = setup(false, Duration::ZERO);
        let mut names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["await_workers", "create_worker", "resume_worker", "stop_worker"]
        );
    }

    #[test]
    fn parse_worker_report_valid_block() {
        let text = "prefix\n```result\n{\"task_id\":\"T9\",\"status\":\"failed\",\"summary\":\"nope\",\"artifacts\":[\"x.rs\",\"y.rs\"]}\n```\ntrail";
        let rep = parse_worker_report("w-1", text);
        assert_eq!(rep.worker_id, "w-1");
        assert_eq!(rep.task_id, "T9");
        assert_eq!(rep.status, "failed");
        assert_eq!(rep.summary, "nope");
        assert_eq!(rep.artifacts, vec!["x.rs".to_string(), "y.rs".to_string()]);
    }

    #[test]
    fn parse_worker_report_falls_back_without_fence() {
        let rep = parse_worker_report("w-2", "just some plain text with no result block");
        assert_eq!(rep.worker_id, "w-2");
        assert_eq!(rep.task_id, "w-2");
        assert_eq!(rep.status, "success");
        assert!(rep.summary.contains("plain text"));
        assert!(rep.artifacts.is_empty());
    }

    #[tokio::test]
    async fn create_then_await_completes() {
        let (_pool, tools, calls) = setup(false, Duration::from_millis(5));
        let out = tool(&tools, "create_worker")
            .execute(json!({ "instruction": "research auth", "task_type": "research" }))
            .await
            .unwrap();
        let wid = wid_of(&out);

        let awaited = tool(&tools, "await_workers")
            .execute(json!({ "ids": [wid.clone()] }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&awaited).unwrap();
        assert_eq!(v.get("complete").unwrap().as_bool(), Some(true));
        let w0 = &v.get("workers").unwrap().as_array().unwrap()[0];
        assert_eq!(w0.get("status").unwrap().as_str(), Some("completed"));
        assert_eq!(
            w0.pointer("/report/status").and_then(|x| x.as_str()),
            Some("success")
        );
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn failed_worker_is_marked_failed() {
        let (_pool, tools, _calls) = setup(true, Duration::from_millis(5));
        let out = tool(&tools, "create_worker")
            .execute(json!({ "instruction": "do thing" }))
            .await
            .unwrap();
        let wid = wid_of(&out);

        let awaited = tool(&tools, "await_workers")
            .execute(json!({}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&awaited).unwrap();
        let w0 = &v.get("workers").unwrap().as_array().unwrap()[0];
        assert_eq!(w0.get("worker_id").unwrap().as_str(), Some(wid.as_str()));
        assert_eq!(w0.get("status").unwrap().as_str(), Some("failed"));
    }

    #[tokio::test]
    async fn stop_worker_marks_stopped() {
        // 长延时确保在 Worker 完成前中止。
        let (pool, tools, _calls) = setup(false, Duration::from_millis(500));
        let out = tool(&tools, "create_worker")
            .execute(json!({ "instruction": "long task" }))
            .await
            .unwrap();
        let wid = wid_of(&out);

        let stopped = tool(&tools, "stop_worker")
            .execute(json!({ "worker_id": wid.clone() }))
            .await
            .unwrap();
        assert!(stopped.contains("stopped"));

        let awaited = tool(&tools, "await_workers")
            .execute(json!({ "ids": [wid.clone()] }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&awaited).unwrap();
        let w0 = &v.get("workers").unwrap().as_array().unwrap()[0];
        assert_eq!(w0.get("status").unwrap().as_str(), Some("stopped"));

        // 未知 id 停止应报错。
        assert!(tool(&tools, "stop_worker")
            .execute(json!({ "worker_id": "nope" }))
            .await
            .is_err());
        let _ = pool;
    }

    #[tokio::test]
    async fn resume_worker_reuses_context() {
        let (_pool, tools, calls) = setup(false, Duration::from_millis(5));
        let out = tool(&tools, "create_worker")
            .execute(json!({ "instruction": "first" }))
            .await
            .unwrap();
        let wid = wid_of(&out);
        tool(&tools, "await_workers")
            .execute(json!({ "ids": [wid.clone()] }))
            .await
            .unwrap();

        // 续跑：复用同一 ctx，指令累积到共享 calls。
        tool(&tools, "resume_worker")
            .execute(json!({ "worker_id": wid.clone(), "instruction": "second" }))
            .await
            .unwrap();
        let awaited = tool(&tools, "await_workers")
            .execute(json!({ "ids": [wid.clone()] }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&awaited).unwrap();
        assert_eq!(v.get("complete").unwrap().as_bool(), Some(true));

        let log = calls.lock().unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], "first");
        assert_eq!(log[1], "second");

        // 未知 worker 续跑应报错。
        assert!(tool(&tools, "resume_worker")
            .execute(json!({ "worker_id": "ghost", "instruction": "x" }))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn await_empty_pool_returns_no_workers() {
        let (_pool, tools, _calls) = setup(false, Duration::ZERO);
        let out = tool(&tools, "await_workers").execute(json!({})).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v.get("workers").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(v.get("complete").unwrap().as_bool(), Some(true));
    }

    #[tokio::test]
    async fn create_worker_requires_instruction() {
        let (_pool, tools, _calls) = setup(false, Duration::ZERO);
        assert!(tool(&tools, "create_worker")
            .execute(json!({ "task_type": "research" }))
            .await
            .is_err());
    }

    #[test]
    fn pool_all_terminal_and_snapshot_skip_missing() {
        let pool = WorkerPool::default();
        let ctx: Arc<dyn WorkerContext> = Arc::new(FailedWorkerContext {
            error: "x".into(),
        });
        pool.insert("w-a".into(), ctx.clone());
        pool.insert("w-b".into(), ctx.clone());
        pool.finish(
            "w-a",
            WorkerStatus::Completed,
            Some(WorkerReport {
                worker_id: "w-a".into(),
                task_id: "w-a".into(),
                status: "success".into(),
                summary: "ok".into(),
                artifacts: vec![],
            }),
        );

        // w-a 终止、w-b 仍运行 -> 整体未终止。
        assert!(!pool.all_terminal(&["w-a".into(), "w-b".into()]));
        // 仅 w-a -> 终止；不存在的 id 视为终止。
        assert!(pool.all_terminal(&["w-a".into(), "ghost".into()]));

        let snap = pool.snapshot(&["w-a".into(), "ghost".into()]);
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, "w-a");
        assert_eq!(snap[0].1, WorkerStatus::Completed);
    }
}
