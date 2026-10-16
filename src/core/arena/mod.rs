//! 竞技场投票
//!
//! 对应《系统功能设计与数据库设计文档》3.8 Agent 竞技场模块：用户可对单轮会话、
//! 单 Agent 输出进行**星级打分（1-5）+ 文字评价**，评价数据落 `agent_arena`，并反向
//! 聚合更新 `agent_info.avg_score`（0-5 均值）与 `call_count`。
//!
//! 设计说明：
//! - 投票经 CLI（`Omega arena vote` / `arena list`）与 TUI（Ctrl+1..5）落库；
//! - 多维度评估（准确性/相关性/流畅性/有用性，各 1-5）序列化为 JSON 存入
//!   `agent_arena.remark`，`vote_type` 仍为综合星级，不改表结构；
//! - [`report`] 子模块提供 LLM 驱动的 Prompt 优化报告智能体（3.8.3 数据闭环），
//!   经 `Omega arena report` 触发，聚合数据见 [`ArenaService::report_data`]。

pub mod report;

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::params;

use crate::core::db::DatabaseManager;
use crate::db::models::agent_arena::AgentArena;
use crate::db::models::agent_info::AgentInfo;

/// 多维度评估定义：(短键, 中文标签)。序列化入 `agent_arena.remark` JSON。
pub const DIMENSIONS: &[(&str, &str)] = &[
    ("acc", "准确性"),
    ("rel", "相关性"),
    ("flu", "流畅性"),
    ("helpful", "有用性"),
];

/// 一次投票的输入。
#[derive(Debug, Clone)]
pub struct VoteInput {
    /// 被投票的 Agent ID（`agent_info.id`）。
    pub agent_id: i32,
    /// 星级评分，取值 1-5。
    pub score: i8,
    /// 文字评价（可选）。
    pub comment: Option<String>,
    /// 关联会话 UUID（`agent_arena.chat_uuid` NOT NULL，未提供时为空串）。
    pub chat_uuid: String,
    /// 关联单轮消息 UUID（可选）。
    pub round_uuid: Option<String>,
    /// 多维度评分（短键 → 1-5），序列化存入 `agent_arena.remark`；空表示未评多维度。
    pub dimensions: BTreeMap<String, i8>,
}

/// 排行榜中一个 Agent 的聚合评分。
#[derive(Debug, Clone)]
pub struct AgentRank {
    pub agent_id: i32,
    pub name: String,
    /// 平均星级评分（0-5，无有效投票时为 `None`）。
    pub avg_score: Option<f32>,
    /// 有效投票数。
    pub vote_count: i64,
}

/// 某 Agent 的评估聚合数据，供 Prompt 优化报告智能体分析。
#[derive(Debug, Clone)]
pub struct ReportData {
    pub agent_id: i32,
    pub agent_name: String,
    /// 当前系统提示词（`prompt_info.content`）。
    pub system_prompt: String,
    /// 综合平均星级（0-5）。
    pub avg_score: Option<f32>,
    /// 有效投票总数。
    pub vote_count: i64,
    /// 星级分布，下标 0..4 对应 1..5 星的票数。
    pub distribution: [i64; 5],
    /// 多维度均值：(维度键, 均值, 样本数)。
    pub dimensions: Vec<(String, f32, i64)>,
    /// 近期文字评价（最多 50 条，按时间倒序）。
    pub comments: Vec<String>,
}

/// 竞技场服务：投票落库、评分聚合、排行榜。
pub struct ArenaService {
    db: DatabaseManager,
}

impl ArenaService {
    /// 打开应用数据库构造服务。
    pub fn new() -> Result<Self> {
        Ok(Self {
            db: DatabaseManager::new()?,
        })
    }

    /// 测试专用：用已有 [`DatabaseManager`]（例如内存库）构造。
    #[cfg(test)]
    fn from_db(db: DatabaseManager) -> Self {
        Self { db }
    }

    /// 记录一条星级投票，并同步重算该 Agent 的 `avg_score`、递增 `call_count`。
    ///
    /// 返回新写入的 `agent_arena` 行 id。评分越界或 Agent 不存在时报错。
    pub fn record_vote(&self, v: VoteInput) -> Result<i64> {
        if !(1..=5).contains(&v.score) {
            bail!("score must be within 1-5");
        }
        // 多维度评分（可选）：逐维校验后序列化为 remark JSON；空则 remark = None。
        let remark = if v.dimensions.is_empty() {
            None
        } else {
            for (k, val) in &v.dimensions {
                if !(1..=5).contains(val) {
                    bail!("dimension '{}' must be within 1-5", k);
                }
            }
            Some(serde_json::to_string(&v.dimensions)?)
        };
        let mut agent = self
            .db
            .read_by_id::<AgentInfo>(v.agent_id)?
            .with_context(|| format!("agent #{} not found", v.agent_id))?;

        let row = AgentArena {
            id: 0,
            chat_uuid: v.chat_uuid,
            round_uuid: v.round_uuid,
            agent_id: v.agent_id,
            vote_type: v.score,
            comment: v.comment,
            status: 1,
            create_time: Utc::now(),
            update_time: Utc::now(),
            remark,
        };
        let id = self.db.create(row)?;

        // 聚合重算平均分并累计调用次数（每次投票视作一次被评价的调用）。
        let (avg, _count) = self.score_stats(v.agent_id)?;
        agent.avg_score = avg;
        agent.call_count += 1;
        self.db.update(agent)?;

        Ok(id)
    }

    /// 计算某 Agent 的有效投票平均分（保留 1 位小数）与票数；无票返回 `(None, 0)`。
    fn score_stats(&self, agent_id: i32) -> Result<(Option<f32>, i64)> {
        Ok(self.db.conn.query_row(
            "SELECT AVG(vote_type), COUNT(*) FROM agent_arena WHERE agent_id = ?1 AND status = 1",
            params![agent_id],
            |row| {
                let avg: Option<f64> = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((avg.map(round1), count))
            },
        )?)
    }

    /// 幂等重算并写回某 Agent 的 `avg_score`（供手动纠正）。返回重算后的均值。
    pub fn recompute(&self, agent_id: i32) -> Result<Option<f32>> {
        let mut agent = self
            .db
            .read_by_id::<AgentInfo>(agent_id)?
            .with_context(|| format!("agent #{} not found", agent_id))?;
        let (avg, _count) = self.score_stats(agent_id)?;
        agent.avg_score = avg;
        self.db.update(agent)?;
        Ok(avg)
    }

    /// 按平均评分降序返回全部 Agent 的排行榜（未评分者置底）。
    pub fn ranking(&self) -> Result<Vec<AgentRank>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT a.id, a.name, a.avg_score, \
                    (SELECT COUNT(*) FROM agent_arena r WHERE r.agent_id = a.id AND r.status = 1) \
             FROM agent_info a \
             ORDER BY a.avg_score DESC, a.id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AgentRank {
                agent_id: row.get(0)?,
                name: row.get(1)?,
                avg_score: row.get(2)?,
                vote_count: row.get(3)?,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 汇总某 Agent 的评估数据（星级分布、多维度均值、文字评价、当前提示词），
    /// 供 Prompt 优化报告智能体分析。Agent 不存在时报错。
    pub fn report_data(&self, agent_id: i32) -> Result<ReportData> {
        let agent = self
            .db
            .read_by_id::<AgentInfo>(agent_id)?
            .with_context(|| format!("agent #{} not found", agent_id))?;

        // 当前系统提示词（prompt_id → prompt_info.content）。
        let system_prompt = match agent.prompt_id {
            Some(pid) => self
                .db
                .conn
                .query_row(
                    "SELECT content FROM prompt_info WHERE id = ?1",
                    params![pid],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_default(),
            None => String::new(),
        };

        // 星级分布（1..5），忽略越界值。
        let mut distribution = [0i64; 5];
        {
            let mut stmt = self.db.conn.prepare(
                "SELECT vote_type, COUNT(*) FROM agent_arena \
                 WHERE agent_id = ?1 AND status = 1 GROUP BY vote_type",
            )?;
            let rows = stmt.query_map(params![agent_id], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
            })?;
            for r in rows {
                let (vt, cnt) = r?;
                if (1..=5).contains(&vt) {
                    distribution[(vt - 1) as usize] = cnt;
                }
            }
        }
        let vote_count: i64 = distribution.iter().sum();

        let dimensions = self.dimension_stats(agent_id)?;

        // 近期非空文字评价（最多 50 条，按时间倒序）。
        let mut comments = Vec::new();
        {
            let mut stmt = self.db.conn.prepare(
                "SELECT comment FROM agent_arena \
                 WHERE agent_id = ?1 AND status = 1 AND comment IS NOT NULL AND comment != '' \
                 ORDER BY id DESC LIMIT 50",
            )?;
            let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
            for r in rows {
                comments.push(r?);
            }
        }

        Ok(ReportData {
            agent_id,
            agent_name: agent.name,
            system_prompt,
            avg_score: agent.avg_score,
            vote_count,
            distribution,
            dimensions,
            comments,
        })
    }

    /// 解析各行 `remark` 中的多维度评分，按 [`DIMENSIONS`] 顺序返回 (键, 均值, 样本数)。
    ///
    /// 非法 JSON 行跳过；未知维度键按字典序追加在已声明维度之后。
    fn dimension_stats(&self, agent_id: i32) -> Result<Vec<(String, f32, i64)>> {
        let mut acc: BTreeMap<String, (i64, i64)> = BTreeMap::new(); // key -> (sum, count)
        {
            let mut stmt = self.db.conn.prepare(
                "SELECT remark FROM agent_arena \
                 WHERE agent_id = ?1 AND status = 1 AND remark IS NOT NULL",
            )?;
            let rows = stmt.query_map(params![agent_id], |row| row.get::<_, String>(0))?;
            for r in rows {
                let raw = r?;
                if let Ok(map) = serde_json::from_str::<BTreeMap<String, i8>>(&raw) {
                    for (k, val) in map {
                        let e = acc.entry(k).or_insert((0, 0));
                        e.0 += val as i64;
                        e.1 += 1;
                    }
                }
            }
        }

        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (key, _label) in DIMENSIONS {
            if let Some((s, c)) = acc.get(*key) {
                out.push((key.to_string(), round1(*s as f64 / *c as f64), *c));
                seen.insert(*key);
            }
        }
        for (k, (s, c)) in &acc {
            if !seen.contains(k.as_str()) {
                out.push((k.clone(), round1(*s as f64 / *c as f64), *c));
            }
        }
        Ok(out)
    }
}

/// 四舍五入到 1 位小数。
fn round1(x: f64) -> f32 {
    ((x * 10.0).round() / 10.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Model;

    /// 构造含 `agent_info`+`agent_arena` 表的内存库，并插入 `n` 个 Agent（id 从 1 起）。
    fn test_service(agents: usize) -> ArenaService {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(AgentInfo::create_table_sql()).unwrap();
        conn.execute_batch(AgentArena::create_table_sql()).unwrap();
        for i in 1..=agents {
            conn.execute(
                "INSERT INTO agent_info (id, llm_id, name, status) VALUES (?1, 1, ?2, 1)",
                params![i as i32, format!("Agent-{}", i)],
            )
            .unwrap();
        }
        ArenaService::from_db(DatabaseManager { conn })
    }

    fn vote(agent_id: i32, score: i8) -> VoteInput {
        VoteInput {
            agent_id,
            score,
            comment: None,
            chat_uuid: String::new(),
            round_uuid: None,
            dimensions: BTreeMap::new(),
        }
    }

    fn dims(pairs: &[(&str, i8)]) -> BTreeMap<String, i8> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn avg_of(svc: &ArenaService, agent_id: i32) -> Option<f32> {
        svc.db
            .read_by_id::<AgentInfo>(agent_id)
            .unwrap()
            .unwrap()
            .avg_score
    }

    fn call_count_of(svc: &ArenaService, agent_id: i32) -> i64 {
        svc.db
            .read_by_id::<AgentInfo>(agent_id)
            .unwrap()
            .unwrap()
            .call_count
    }

    #[test]
    fn record_vote_inserts_and_updates_avg() {
        let svc = test_service(1);
        let id = svc.record_vote(vote(1, 5)).unwrap();
        assert!(id > 0);

        let rows = svc
            .db
            .find_where::<AgentArena>("agent_id = ?1", &[&1i32])
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vote_type, 5);
        assert_eq!(rows[0].status, 1);

        assert_eq!(avg_of(&svc, 1), Some(5.0));
        assert_eq!(call_count_of(&svc, 1), 1);
    }

    #[test]
    fn avg_rounds_to_one_decimal() {
        let svc = test_service(1);
        svc.record_vote(vote(1, 5)).unwrap();
        svc.record_vote(vote(1, 4)).unwrap();
        assert_eq!(avg_of(&svc, 1), Some(4.5));
        assert_eq!(call_count_of(&svc, 1), 2);
    }

    #[test]
    fn invalid_score_rejected() {
        let svc = test_service(1);
        assert!(svc.record_vote(vote(1, 0)).is_err());
        assert!(svc.record_vote(vote(1, 6)).is_err());
        // 越界投票不应写入任何行。
        let rows = svc
            .db
            .find_where::<AgentArena>("agent_id = ?1", &[&1i32])
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn nonexistent_agent_rejected() {
        let svc = test_service(1);
        let err = svc.record_vote(vote(99, 5)).unwrap_err();
        assert!(err.to_string().contains("agent #99 not found"));
    }

    #[test]
    fn comment_and_round_uuid_persisted() {
        let svc = test_service(1);
        svc.record_vote(VoteInput {
            agent_id: 1,
            score: 3,
            comment: Some("ok".to_string()),
            chat_uuid: "c-1".to_string(),
            round_uuid: Some("r-1".to_string()),
            dimensions: BTreeMap::new(),
        })
        .unwrap();
        let rows = svc
            .db
            .find_where::<AgentArena>("agent_id = ?1", &[&1i32])
            .unwrap();
        assert_eq!(rows[0].comment.as_deref(), Some("ok"));
        assert_eq!(rows[0].chat_uuid, "c-1");
        assert_eq!(rows[0].round_uuid.as_deref(), Some("r-1"));
    }

    #[test]
    fn ranking_orders_by_score_desc() {
        let svc = test_service(3);
        // Agent-1 均分 3，Agent-2 均分 5，Agent-3 无投票。
        svc.record_vote(vote(1, 3)).unwrap();
        svc.record_vote(vote(2, 5)).unwrap();
        svc.record_vote(vote(2, 5)).unwrap();

        let ranks = svc.ranking().unwrap();
        assert_eq!(ranks.len(), 3);
        // 最高分在前。
        assert_eq!(ranks[0].agent_id, 2);
        assert_eq!(ranks[0].avg_score, Some(5.0));
        assert_eq!(ranks[0].vote_count, 2);
        // 次高。
        assert_eq!(ranks[1].agent_id, 1);
        assert_eq!(ranks[1].avg_score, Some(3.0));
        assert_eq!(ranks[1].vote_count, 1);
        // 未评分置底。
        assert_eq!(ranks[2].agent_id, 3);
        assert_eq!(ranks[2].avg_score, None);
        assert_eq!(ranks[2].vote_count, 0);
    }

    #[test]
    fn recompute_is_idempotent() {
        let svc = test_service(1);
        svc.record_vote(vote(1, 4)).unwrap();
        // 手动置空后重算应恢复。
        let mut a = svc.db.read_by_id::<AgentInfo>(1).unwrap().unwrap();
        a.avg_score = None;
        svc.db.update(a).unwrap();
        assert_eq!(avg_of(&svc, 1), None);
        assert_eq!(svc.recompute(1).unwrap(), Some(4.0));
        assert_eq!(avg_of(&svc, 1), Some(4.0));
    }

    #[test]
    fn record_vote_serializes_dimensions_to_remark() {
        let svc = test_service(1);
        svc.record_vote(VoteInput {
            agent_id: 1,
            score: 4,
            comment: None,
            chat_uuid: "c-1".to_string(),
            round_uuid: None,
            dimensions: dims(&[("acc", 5), ("rel", 4)]),
        })
        .unwrap();
        let rows = svc
            .db
            .find_where::<AgentArena>("agent_id = ?1", &[&1i32])
            .unwrap();
        assert_eq!(rows[0].vote_type, 4);
        let remark = rows[0].remark.clone().unwrap();
        let parsed: BTreeMap<String, i8> = serde_json::from_str(&remark).unwrap();
        assert_eq!(parsed.get("acc"), Some(&5));
        assert_eq!(parsed.get("rel"), Some(&4));
    }

    #[test]
    fn record_vote_rejects_out_of_range_dimension() {
        let svc = test_service(1);
        let r = svc.record_vote(VoteInput {
            agent_id: 1,
            score: 4,
            comment: None,
            chat_uuid: String::new(),
            round_uuid: None,
            dimensions: dims(&[("acc", 6)]),
        });
        assert!(r.is_err());
        // 越界维度不应写入任何行。
        let rows = svc
            .db
            .find_where::<AgentArena>("agent_id = ?1", &[&1i32])
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn report_data_aggregates_distribution_dims_comments() {
        let svc = test_service(1);
        svc.record_vote(VoteInput {
            agent_id: 1,
            score: 5,
            comment: Some("great".to_string()),
            chat_uuid: String::new(),
            round_uuid: None,
            dimensions: dims(&[("acc", 5), ("rel", 5)]),
        })
        .unwrap();
        svc.record_vote(VoteInput {
            agent_id: 1,
            score: 3,
            comment: Some("ok".to_string()),
            chat_uuid: String::new(),
            round_uuid: None,
            dimensions: dims(&[("acc", 3), ("rel", 5)]),
        })
        .unwrap();
        svc.record_vote(VoteInput {
            agent_id: 1,
            score: 3,
            comment: None,
            chat_uuid: String::new(),
            round_uuid: None,
            dimensions: BTreeMap::new(),
        })
        .unwrap();

        let data = svc.report_data(1).unwrap();
        assert_eq!(data.agent_name, "Agent-1");
        assert_eq!(data.vote_count, 3);
        // 分布：5 星 1 票，3 星 2 票。
        assert_eq!(data.distribution[4], 1);
        assert_eq!(data.distribution[2], 2);
        // acc 均值 (5+3)/2=4.0（样本 2）；rel 均值 5.0。
        let acc = data.dimensions.iter().find(|d| d.0 == "acc").unwrap();
        assert_eq!(acc.1, 4.0);
        assert_eq!(acc.2, 2);
        let rel = data.dimensions.iter().find(|d| d.0 == "rel").unwrap();
        assert_eq!(rel.1, 5.0);
        // 评论按 id DESC，仅 2 条非空。
        assert_eq!(data.comments.len(), 2);
        assert_eq!(data.comments[0], "ok");
        assert_eq!(data.comments[1], "great");
    }
}
