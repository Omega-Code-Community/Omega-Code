//! 记忆系统
//!
//! 对应《系统功能设计与数据库设计文档》3.10 记忆模块。分两类记忆：
//! - **用户记忆**（`user`）：长期编码习惯、对话偏好、使用行为，全局生效；正文存于
//!   `~/.omega/memory/<uuid>.md`（[`crate::platform::get_platform_app_dir`]）。
//! - **项目记忆**（`project`）：绑定单一项目上下文，抑制幻觉、固定业务边界；正文存于
//!   `<workspace>/.omega/memory/<uuid>.md`，仅在 Workspace 模式（`project_dir` 存在）可用。
//!
//! 存储采用「数据库 + 本地文件」双存储：`memory_info` 表仅存**索引**
//! （memory_uuid / memory_type / keywords / is_vector=0 / vector_id=NULL / status），
//! **正文写文件**。FAISS/LanceDB 向量检索、上下文压缩、过期清理属设计明确的
//! 「前期不实现」范围，本模块不涉及。
//!
//! 加载作用域来自 `system_config.memory_type`（由 `Omega memory --type` 写入），映射为
//! [`MemoryScope`]；未设置时为 [`MemoryScope::None`]（记忆关闭）。
//!
//! [`MemoryExtractor`] 提供自动抽取：一轮对话结束后用独立 provider 调 LLM 总结可长期
//! 沉淀的信息，去重后经 [`MemoryStore::save`] 写入。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use log::warn;
use rand::RngExt;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::core::db::Model;
use crate::core::providers::{
    create_provider, CompletionRequest, Message, ProviderConfig, Role,
};
use crate::db::models::memory_info::MemoryInfo;
use crate::platform::{get_database_path, get_platform_app_dir};

/// 单次注入提示词的最大记忆条数。
pub const MAX_INJECT: usize = 20;
/// 注入提示词段的最大字符数（防止提示词膨胀）。
pub const MAX_INJECT_CHARS: usize = 4000;

// ============================================================
// 作用域与领域对象
// ============================================================

/// 记忆加载作用域（对应 `system_config.memory_type`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    /// 仅加载用户记忆。
    User,
    /// 仅加载项目记忆。
    Project,
    /// 加载用户 + 项目记忆。
    All,
    /// 关闭记忆功能。
    None,
}

impl MemoryScope {
    /// 从字符串解析作用域；无法识别的值一律视为 [`MemoryScope::None`]（安全关闭）。
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "user" => MemoryScope::User,
            "project" => MemoryScope::Project,
            "all" => MemoryScope::All,
            _ => MemoryScope::None,
        }
    }

    /// 稳定小写标识。
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryScope::User => "user",
            MemoryScope::Project => "project",
            MemoryScope::All => "all",
            MemoryScope::None => "none",
        }
    }

    /// 该作用域是否允许加载/写入指定 `memory_type`（"user" / "project"）。
    pub fn allows_type(&self, memory_type: &str) -> bool {
        match self {
            MemoryScope::All => memory_type == "user" || memory_type == "project",
            MemoryScope::User => memory_type == "user",
            MemoryScope::Project => memory_type == "project",
            MemoryScope::None => false,
        }
    }
}

/// 一条记忆的领域对象：索引字段 + 文件正文。
#[derive(Debug, Clone)]
pub struct MemoryRecord {
    /// 记忆唯一标识（同时作为正文文件名 `<uuid>.md`）。
    pub memory_uuid: String,
    /// 记忆类型："user" / "project"。
    pub memory_type: String,
    /// 检索关键词。
    pub keywords: Vec<String>,
    /// 记忆正文（来自文件）。
    pub content: String,
}

/// 归一化关键词：去空白、去空项、排序、去重后以逗号连接（用于存储与去重比较）。
fn normalize_keywords(keywords: &[String]) -> String {
    let mut v: Vec<String> = keywords
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v.join(",")
}

/// 拆分存储的关键词串为向量。
fn split_keywords(s: Option<String>) -> Vec<String> {
    match s {
        Some(x) => x
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect(),
        None => Vec::new(),
    }
}

/// 生成记忆唯一标识（无外部 uuid 依赖）。
fn new_memory_uuid() -> String {
    let ts = chrono::Utc::now().timestamp();
    let rand_part: u32 = rand::rng().random_range(0..u32::MAX);
    format!("m-{:x}-{:08x}", ts, rand_part)
}

// ============================================================
// MemoryStore
// ============================================================

/// 记忆存储：`memory_info` 索引（SQLite）+ 正文文件（本地目录）。
///
/// 内部用 [`Mutex<Connection>`] 保证 `Send + Sync`，可被 Agent 在异步任务中共享。
pub struct MemoryStore {
    conn: Mutex<Connection>,
    /// 用户记忆正文目录（`~/.omega/memory`）。
    user_dir: PathBuf,
    /// 项目记忆正文目录（`<workspace>/.omega/memory`）；`None` 表示非 Workspace 模式。
    project_dir: Option<PathBuf>,
}

impl MemoryStore {
    /// 打开记忆存储。
    ///
    /// `project_root` 为 `Some` 时启用项目记忆（目录 `<root>/.omega/memory`），
    /// 传入 Workspace 根路径；`None`（single 模式）时仅支持用户记忆。
    pub fn new(project_root: Option<PathBuf>) -> Result<Self> {
        let path = get_database_path()?;
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;
        Self::ensure_table(&conn)?;
        let user_dir = get_platform_app_dir()?.join("memory");
        let project_dir = project_root.map(|p| p.join(".omega").join("memory"));
        Ok(Self {
            conn: Mutex::new(conn),
            user_dir,
            project_dir,
        })
    }

    /// 测试专用：用已有连接与目录构造。
    #[cfg(test)]
    fn from_parts(conn: Connection, user_dir: PathBuf, project_dir: Option<PathBuf>) -> Self {
        Self::ensure_table(&conn).unwrap();
        Self {
            conn: Mutex::new(conn),
            user_dir,
            project_dir,
        }
    }

    /// 确保 `memory_info` 表存在（启动时已建，此处幂等兜底以支持独立使用）。
    fn ensure_table(conn: &Connection) -> Result<()> {
        conn.execute_batch(MemoryInfo::create_table_sql())?;
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow!("MemoryStore mutex poisoned"))
    }

    /// 指定记忆类型的正文目录。项目记忆在 `project_dir` 缺失时报错。
    fn dir_for(&self, memory_type: &str) -> Result<PathBuf> {
        match memory_type {
            "project" => self
                .project_dir
                .clone()
                .ok_or_else(|| anyhow!("project memory unavailable in single mode")),
            _ => Ok(self.user_dir.clone()),
        }
    }

    /// 读取某条记忆的正文文件。
    fn read_content(&self, memory_type: &str, memory_uuid: &str) -> Result<String> {
        let path = self
            .dir_for(memory_type)?
            .join(format!("{}.md", memory_uuid));
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read memory file: {}", path.display()))
    }

    /// 保存（或更新）一条记忆：写正文文件 + upsert `memory_info` 索引行。
    ///
    /// 以 `memory_uuid` 为幂等键：已存在则更新 keywords/status/update_time，否则插入。
    pub fn save(&self, rec: &MemoryRecord) -> Result<()> {
        let dir = self.dir_for(&rec.memory_type)?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create memory dir: {}", dir.display()))?;
        let path = dir.join(format!("{}.md", rec.memory_uuid));
        std::fs::write(&path, &rec.content)
            .with_context(|| format!("failed to write memory file: {}", path.display()))?;

        let keywords = normalize_keywords(&rec.keywords);
        let conn = self.lock()?;
        let existing: Option<i32> = conn
            .query_row(
                "SELECT id FROM memory_info WHERE memory_uuid = ?1",
                params![rec.memory_uuid],
                |row| row.get(0),
            )
            .optional()?;
        match existing {
            Some(id) => {
                conn.execute(
                    "UPDATE memory_info SET keywords = ?1, status = 1, \
                     update_time = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![keywords, id],
                )?;
            }
            None => {
                conn.execute(
                    "INSERT INTO memory_info \
                     (memory_uuid, memory_type, is_vector, vector_id, keywords, status) \
                     VALUES (?1, ?2, 0, NULL, ?3, 1)",
                    params![rec.memory_uuid, rec.memory_type, keywords],
                )?;
            }
        }
        Ok(())
    }

    /// 按作用域加载记忆（正文来自文件），按更新时间倒序，上限 [`MAX_INJECT`] 条。
    ///
    /// 正文文件缺失的记忆跳过并告警（不阻断加载）。[`MemoryScope::None`] 返回空。
    pub fn load(&self, scope: MemoryScope) -> Result<Vec<MemoryRecord>> {
        if scope == MemoryScope::None {
            return Ok(Vec::new());
        }
        let where_sql = match scope {
            MemoryScope::User => "status = 1 AND memory_type = 'user'",
            MemoryScope::Project => "status = 1 AND memory_type = 'project'",
            _ => "status = 1",
        };
        let sql = format!(
            "SELECT memory_uuid, memory_type, keywords FROM memory_info \
             WHERE {} ORDER BY update_time DESC, id DESC LIMIT ?1",
            where_sql
        );
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![MAX_INJECT as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut out = Vec::new();
        for row in rows {
            let (memory_uuid, memory_type, kw) = row?;
            match self.read_content(&memory_type, &memory_uuid) {
                Ok(content) => out.push(MemoryRecord {
                    memory_uuid,
                    memory_type,
                    keywords: split_keywords(kw),
                    content,
                }),
                Err(e) => warn!("skip memory ({}): {}", memory_uuid, e),
            }
        }
        Ok(out)
    }

    /// 列出记忆（等价于 [`MemoryStore::load`]，含正文，供展示/调试）。
    pub fn list(&self, scope: MemoryScope) -> Result<Vec<MemoryRecord>> {
        self.load(scope)
    }

    /// 物理删除一条记忆（文件 + 索引行）。返回是否删除了索引行。
    pub fn delete(&self, memory_uuid: &str) -> Result<bool> {
        let conn = self.lock()?;
        let memory_type: Option<String> = conn
            .query_row(
                "SELECT memory_type FROM memory_info WHERE memory_uuid = ?1",
                params![memory_uuid],
                |row| row.get(0),
            )
            .optional()?;
        let removed = conn.execute(
            "DELETE FROM memory_info WHERE memory_uuid = ?1",
            params![memory_uuid],
        )?;
        if let Some(mt) = memory_type {
            if let Ok(dir) = self.dir_for(&mt) {
                let path = dir.join(format!("{}.md", memory_uuid));
                let _ = std::fs::remove_file(path);
            }
        }
        Ok(removed > 0)
    }

    /// 按关键词模糊检索记忆（`keywords LIKE %kw%`），受作用域约束。
    pub fn search(&self, keyword: &str, scope: MemoryScope) -> Result<Vec<MemoryRecord>> {
        if scope == MemoryScope::None || keyword.trim().is_empty() {
            return Ok(Vec::new());
        }
        let type_sql = match scope {
            MemoryScope::User => " AND memory_type = 'user'",
            MemoryScope::Project => " AND memory_type = 'project'",
            _ => "",
        };
        let sql = format!(
            "SELECT memory_uuid, memory_type, keywords FROM memory_info \
             WHERE status = 1 AND keywords LIKE ?1{} ORDER BY update_time DESC, id DESC",
            type_sql
        );
        let like = format!("%{}%", keyword.trim());
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![like], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut out = Vec::new();
        for row in rows {
            let (memory_uuid, memory_type, kw) = row?;
            match self.read_content(&memory_type, &memory_uuid) {
                Ok(content) => out.push(MemoryRecord {
                    memory_uuid,
                    memory_type,
                    keywords: split_keywords(kw),
                    content,
                }),
                Err(e) => warn!("skip memory ({}): {}", memory_uuid, e),
            }
        }
        Ok(out)
    }

    /// 去重启发式：是否已存在同类型且关键词集合完全相同的有效记忆。
    pub fn exists_same(&self, memory_type: &str, keywords: &[String]) -> bool {
        let kw = normalize_keywords(keywords);
        if kw.is_empty() {
            return false;
        }
        let conn = match self.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.query_row(
            "SELECT 1 FROM memory_info WHERE status = 1 AND memory_type = ?1 AND keywords = ?2 LIMIT 1",
            params![memory_type, kw],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
    }

    /// 渲染注入 Agent 系统提示词的记忆段；无记忆或作用域为 None 时返回空串。
    pub fn render_context(&self, scope: MemoryScope) -> Result<String> {
        let records = self.load(scope)?;
        if records.is_empty() {
            return Ok(String::new());
        }
        let mut user_lines: Vec<String> = Vec::new();
        let mut project_lines: Vec<String> = Vec::new();
        for r in records {
            let line = format!("- [{}] {}", r.keywords.join(", "), r.content.trim());
            if r.memory_type == "project" {
                project_lines.push(line);
            } else {
                user_lines.push(line);
            }
        }

        let mut s = format!("# Memory (scope={})\n", scope.as_str());
        if !user_lines.is_empty() {
            s.push_str("## User Memory\n");
            for l in &user_lines {
                s.push_str(l);
                s.push('\n');
            }
        }
        if !project_lines.is_empty() {
            s.push_str("## Project Memory\n");
            for l in &project_lines {
                s.push_str(l);
                s.push('\n');
            }
        }

        if s.chars().count() > MAX_INJECT_CHARS {
            s = s.chars().take(MAX_INJECT_CHARS).collect();
        }
        Ok(s)
    }
}

// ============================================================
// 自动抽取
// ============================================================

/// 从对话中抽取到的一条候选记忆（解析自 LLM 输出）。
#[derive(Debug, Clone)]
pub struct ExtractedMemory {
    /// 记忆类型："user" / "project"。
    pub memory_type: String,
    /// 记忆正文。
    pub content: String,
    /// 关键词。
    pub keywords: Vec<String>,
}

/// 记忆抽取指令。
pub const EXTRACTION_PROMPT: &str = "\
You are a memory extraction assistant for a local coding agent.
Review the conversation round below and extract ONLY information worth remembering LONG-TERM.

Extract two kinds of memories:
- \"user\": durable user coding habits, tooling/style preferences, or communication preferences.
- \"project\": durable project facts, constraints, conventions, or business boundaries.

Rules:
- Ignore one-off task details, transient state, secrets/credentials, and anything sensitive.
- Keep each memory concise and self-contained (one fact per entry).
- If nothing is worth remembering, output an empty array.

Output ONLY a fenced code block tagged ```memory containing a JSON array. Each element:
{\"type\": \"user\" | \"project\", \"content\": \"<text>\", \"keywords\": [\"<kw>\", ...]}

Example:
```memory
[{\"type\": \"user\", \"content\": \"Prefers Rust with anyhow for errors\", \"keywords\": [\"rust\", \"error-handling\"]}]
```
";

/// 把角色转为文本标识。
fn role_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// 把一组消息渲染为抽取用的对话文本。
fn render_transcript(messages: &[Message]) -> String {
    let mut s = String::new();
    for m in messages {
        s.push_str(role_str(&m.role));
        s.push_str(": ");
        s.push_str(m.content.trim());
        s.push('\n');
    }
    s
}

/// 解析 LLM 输出中的 ```memory 围栏 JSON 数组为候选记忆列表。
///
/// 取**最后一个** ```memory 围栏块；无围栏或 JSON 非法时返回空 Vec（不抛错）。
/// 仅接受 `type` 为 "user"/"project" 且 `content` 非空的元素。
pub fn parse_extracted_memories(text: &str) -> Vec<ExtractedMemory> {
    const MARKER: &str = "```memory";
    let Some(start) = text.rfind(MARKER) else {
        return Vec::new();
    };
    let after = &text[start + MARKER.len()..];
    let Some(end) = after.find("```") else {
        return Vec::new();
    };
    let json_str = after[..end].trim();
    let value: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for item in arr {
        let memory_type = item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if memory_type != "user" && memory_type != "project" {
            continue;
        }
        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if content.trim().is_empty() {
            continue;
        }
        let keywords = item
            .get("keywords")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|k| k.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        out.push(ExtractedMemory {
            memory_type,
            content,
            keywords,
        });
    }
    out
}

/// 记忆抽取器：一轮对话结束后用独立 provider 调 LLM 总结记忆并落库。
///
/// 使用与 Agent 相同的供应商/模型配置，但构造**独立** provider（不借用 Agent 的
/// `Box<dyn Provider>`），以便在 `tokio::spawn` 的独立任务中运行、不阻塞对话。
pub struct MemoryExtractor {
    provider_config: ProviderConfig,
    model: String,
    temperature: Option<f32>,
    store: Arc<MemoryStore>,
    scope: MemoryScope,
}

impl MemoryExtractor {
    /// 构造抽取器。
    pub fn new(
        provider_config: ProviderConfig,
        model: String,
        temperature: Option<f32>,
        store: Arc<MemoryStore>,
        scope: MemoryScope,
    ) -> Self {
        Self {
            provider_config,
            model,
            temperature,
            store,
            scope,
        }
    }

    /// 从一轮对话中抽取并写入记忆，返回实际写入条数。
    ///
    /// 作用域为 None 或无消息时直接返回 0。抽取到的候选按作用域过滤、跳过空内容与
    /// 已存在（同类型 + 同关键词集合）者，其余生成新 uuid 落库。
    pub async fn extract_from_round(&self, messages: &[Message]) -> Result<usize> {
        if self.scope == MemoryScope::None || messages.is_empty() {
            return Ok(0);
        }
        let provider = create_provider(self.provider_config.clone())?;
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: EXTRACTION_PROMPT.to_string(),
                },
                Message {
                    role: Role::User,
                    content: render_transcript(messages),
                },
            ],
            temperature: self.temperature,
            max_tokens: None,
            stream: false,
        };
        let response = provider.complete(request).await?;
        let candidates = parse_extracted_memories(&response.content);

        let mut written = 0usize;
        for c in candidates {
            if !self.scope.allows_type(&c.memory_type) {
                continue;
            }
            if self.store.exists_same(&c.memory_type, &c.keywords) {
                continue;
            }
            let rec = MemoryRecord {
                memory_uuid: new_memory_uuid(),
                memory_type: c.memory_type,
                keywords: c.keywords,
                content: c.content,
            };
            self.store.save(&rec)?;
            written += 1;
        }
        Ok(written)
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("omega-mem-{}-{}", tag, new_memory_uuid()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn test_store(with_project: bool) -> MemoryStore {
        let conn = Connection::open_in_memory().unwrap();
        let project_dir = if with_project { Some(temp_dir("proj")) } else { None };
        MemoryStore::from_parts(conn, temp_dir("user"), project_dir)
    }

    fn rec(uuid: &str, ty: &str, kws: &[&str], content: &str) -> MemoryRecord {
        MemoryRecord {
            memory_uuid: uuid.to_string(),
            memory_type: ty.to_string(),
            keywords: kws.iter().map(|s| s.to_string()).collect(),
            content: content.to_string(),
        }
    }

    #[test]
    fn scope_parse_and_allows() {
        assert_eq!(MemoryScope::parse("USER"), MemoryScope::User);
        assert_eq!(MemoryScope::parse(" project "), MemoryScope::Project);
        assert_eq!(MemoryScope::parse("all"), MemoryScope::All);
        assert_eq!(MemoryScope::parse("bogus"), MemoryScope::None);
        assert_eq!(MemoryScope::parse(""), MemoryScope::None);

        assert!(MemoryScope::All.allows_type("user"));
        assert!(MemoryScope::All.allows_type("project"));
        assert!(MemoryScope::User.allows_type("user"));
        assert!(!MemoryScope::User.allows_type("project"));
        assert!(MemoryScope::Project.allows_type("project"));
        assert!(!MemoryScope::None.allows_type("user"));
        assert_eq!(MemoryScope::All.as_str(), "all");
    }

    #[test]
    fn save_load_roundtrip_user() {
        let store = test_store(false);
        store
            .save(&rec("u1", "user", &["rust", "errors"], "Prefers anyhow"))
            .unwrap();
        // 正文文件写入用户目录。
        assert!(store.user_dir.join("u1.md").exists());

        let loaded = store.load(MemoryScope::User).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].memory_uuid, "u1");
        assert_eq!(loaded[0].content, "Prefers anyhow");
        let mut kws = loaded[0].keywords.clone();
        kws.sort();
        assert_eq!(kws, vec!["errors", "rust"]);
    }

    #[test]
    fn scope_filters_records() {
        let store = test_store(true);
        store.save(&rec("u1", "user", &["a"], "user mem")).unwrap();
        store
            .save(&rec("p1", "project", &["b"], "project mem"))
            .unwrap();

        assert_eq!(store.load(MemoryScope::User).unwrap().len(), 1);
        assert_eq!(store.load(MemoryScope::Project).unwrap().len(), 1);
        assert_eq!(store.load(MemoryScope::All).unwrap().len(), 2);
        assert_eq!(store.load(MemoryScope::None).unwrap().len(), 0);
    }

    #[test]
    fn project_save_without_dir_errors() {
        let store = test_store(false);
        let err = store
            .save(&rec("p1", "project", &["x"], "no project dir"))
            .unwrap_err();
        assert!(err.to_string().contains("project memory unavailable"));
    }

    #[test]
    fn delete_removes_file_and_row() {
        let store = test_store(false);
        store.save(&rec("u1", "user", &["a"], "mem")).unwrap();
        assert!(store.user_dir.join("u1.md").exists());

        assert!(store.delete("u1").unwrap());
        assert!(!store.user_dir.join("u1.md").exists());
        assert_eq!(store.load(MemoryScope::User).unwrap().len(), 0);
        // 再次删除返回 false（无索引行）。
        assert!(!store.delete("u1").unwrap());
    }

    #[test]
    fn save_is_upsert_by_uuid() {
        let store = test_store(false);
        store.save(&rec("u1", "user", &["a"], "v1")).unwrap();
        store.save(&rec("u1", "user", &["b"], "v2")).unwrap();
        let loaded = store.load(MemoryScope::User).unwrap();
        assert_eq!(loaded.len(), 1, "upsert should not duplicate row");
        assert_eq!(loaded[0].content, "v2");
        assert_eq!(loaded[0].keywords, vec!["b"]);
    }

    #[test]
    fn search_by_keyword() {
        let store = test_store(false);
        store
            .save(&rec("u1", "user", &["rust", "tokio"], "async"))
            .unwrap();
        store.save(&rec("u2", "user", &["python"], "sync")).unwrap();

        let hits = store.search("tokio", MemoryScope::All).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].memory_uuid, "u1");

        assert_eq!(store.search("golang", MemoryScope::All).unwrap().len(), 0);
        // 作用域约束：project 作用域下搜不到 user 记忆。
        assert_eq!(store.search("tokio", MemoryScope::Project).unwrap().len(), 0);
    }

    #[test]
    fn exists_same_dedups_by_keyword_set() {
        let store = test_store(false);
        store.save(&rec("u1", "user", &["a", "b"], "mem")).unwrap();
        // 顺序不同的同集合应判为已存在。
        assert!(store.exists_same(
            "user",
            &["b".to_string(), "a".to_string()]
        ));
        assert!(!store.exists_same("user", &["a".to_string()]));
        assert!(!store.exists_same("project", &["a".to_string(), "b".to_string()]));
        assert!(!store.exists_same("user", &[]));
    }

    #[test]
    fn render_context_formats_and_guards() {
        let store = test_store(true);
        assert_eq!(store.render_context(MemoryScope::None).unwrap(), "");
        assert_eq!(store.render_context(MemoryScope::All).unwrap(), "");

        store.save(&rec("u1", "user", &["a"], "user mem")).unwrap();
        store
            .save(&rec("p1", "project", &["b"], "project mem"))
            .unwrap();
        let ctx = store.render_context(MemoryScope::All).unwrap();
        assert!(ctx.starts_with("# Memory (scope=all)"));
        assert!(ctx.contains("## User Memory"));
        assert!(ctx.contains("## Project Memory"));
        assert!(ctx.contains("user mem"));
        assert!(ctx.contains("project mem"));
    }

    #[test]
    fn parse_extracted_memories_valid() {
        let text = "Here you go:\n```memory\n[\
            {\"type\":\"user\",\"content\":\"likes rust\",\"keywords\":[\"rust\"]},\
            {\"type\":\"project\",\"content\":\"uses sqlite\",\"keywords\":[\"db\"]}\
            ]\n```";
        let out = parse_extracted_memories(text);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].memory_type, "user");
        assert_eq!(out[0].content, "likes rust");
        assert_eq!(out[0].keywords, vec!["rust"]);
        assert_eq!(out[1].memory_type, "project");
    }

    #[test]
    fn parse_extracted_memories_filters_invalid_entries() {
        let text = "```memory\n[\
            {\"type\":\"other\",\"content\":\"x\"},\
            {\"type\":\"user\",\"content\":\"   \"},\
            {\"type\":\"user\",\"content\":\"ok\",\"keywords\":[\"k\"]}\
            ]\n```";
        let out = parse_extracted_memories(text);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "ok");
    }

    #[test]
    fn parse_extracted_memories_falls_back_empty() {
        assert!(parse_extracted_memories("no fence here").is_empty());
        assert!(parse_extracted_memories("```memory\nnot json\n```").is_empty());
        assert!(parse_extracted_memories("```memory\n{\"type\":\"user\"}\n```").is_empty());
    }
}
