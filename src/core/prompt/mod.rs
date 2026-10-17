//! Prompt 占位符解析
//!
//! 对应《系统功能设计与数据库设计文档》四.5 `prompt_placeholder`：系统提示词中的 `{key}`
//! 占位符在加载 Agent 时被替换为实际值。内置两个约定占位符：
//! - `{AgentList}`：当前可调用的其它 Agent 名称列表；
//! - `{MCPList}`：当前 Agent 已授权的 MCP 工具名称列表；
//!
//! 其余占位符来自 `prompt_placeholder` 表（`key`→`value`，缺省回退 `default_value`）。
//! 未知/空/跨行/嵌套花括号的占位符**原样保留**，避免因模板书写差异破坏正文。

use std::collections::HashMap;
use std::collections::HashSet;

/// 将 `template` 中的 `{key}` 占位符按 `vars` 替换。
///
/// 扫描按 ASCII 花括号字节位切片（多字节 UTF-8 续字节均 `>= 0x80`，故不会误伤中文正文）：
/// - 命中 `vars` 的键：替换为其值；
/// - 未知键、空 `{}`、含换行或嵌套花括号：原样保留；
/// - 无闭合 `}`：原样输出剩余部分。
pub fn render(template: &str, vars: &HashMap<String, String>) -> String {
    let bytes = template.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;

    while i < bytes.len() {
        // 定位下一个 `{`（其前的正文原样拷贝）。
        let Some(rel) = bytes[i..].iter().position(|&b| b == b'{') else {
            out.push_str(&template[i..]);
            break;
        };
        let brace = i + rel;
        out.push_str(&template[i..brace]);

        let after = brace + 1;
        // 无闭合 `}`：原样输出剩余并结束。
        let Some(rel_end) = bytes[after..].iter().position(|&b| b == b'}') else {
            out.push_str(&template[brace..]);
            break;
        };
        let end = after + rel_end; // `}` 的字节下标
        let key = &template[after..end];

        let placeholder_valid =
            !key.is_empty() && !key.contains('\n') && !key.contains('{') && !key.contains('}');

        if placeholder_valid {
            match vars.get(key) {
                Some(val) => out.push_str(val),
                // 未知键：原样保留 `{key}`。
                None => out.push_str(&template[brace..=end]),
            }
        } else {
            // 非法占位符（空/跨行/嵌套）：原样保留。
            out.push_str(&template[brace..=end]);
        }

        i = end + 1;
    }

    out
}

/// 构建占位符变量表。
///
/// - `AgentList`/`MCPList` 渲染为 `- item` 多行列表（空集合则为空串）；
/// - 自定义占位符 `key → value.or(default_value).unwrap_or("")`（`value` 优先于 `default_value`）。
pub fn build_vars(
    callable_agents: &[String],
    mcp_tools: &[String],
    placeholders: &[(String, Option<String>, Option<String>)],
) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    vars.insert("AgentList".to_string(), render_list(callable_agents));
    vars.insert("MCPList".to_string(), render_list(mcp_tools));

    for (key, value, default_value) in placeholders {
        let resolved = value.clone().or_else(|| default_value.clone()).unwrap_or_default();
        vars.insert(key.clone(), resolved);
    }

    vars
}

/// 将条目渲染为 `- item` 多行列表；空集合返回空串。
fn render_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- {}", item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 占位符扫描结果（用于§3.6.5 语法合法性校验）。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PlaceholderScan {
    /// 合法占位符键（按出现顺序去重）。
    pub keys: Vec<String>,
    /// 非法花括号片段（空 `{}`、跨行、嵌套），原样保留。
    pub malformed: Vec<String>,
    /// 是否存在未闭合的 `{`。
    pub unclosed: bool,
}

/// 扫描模板中的占位符，规则与 [`render`] 一致。
///
/// - `{key}` 且 key 非空、不含换行/花括号 → 合法键（去重）；
/// - 空 `{}`、跨行、嵌套花括号 → 记为非法片段；
/// - 出现 `{` 但无后续 `}` → 记为未闭合。
pub fn scan_placeholders(template: &str) -> PlaceholderScan {
    let bytes = template.as_bytes();
    let mut scan = PlaceholderScan::default();
    let mut i = 0usize;

    while i < bytes.len() {
        let Some(rel) = bytes[i..].iter().position(|&b| b == b'{') else {
            break;
        };
        let brace = i + rel;
        let after = brace + 1;
        let Some(rel_end) = bytes[after..].iter().position(|&b| b == b'}') else {
            scan.unclosed = true;
            break;
        };
        let end = after + rel_end;
        let key = &template[after..end];
        let valid =
            !key.is_empty() && !key.contains('\n') && !key.contains('{') && !key.contains('}');
        if valid {
            if !scan.keys.iter().any(|k| k == key) {
                scan.keys.push(key.to_string());
            }
        } else {
            scan.malformed.push(template[brace..=end].to_string());
        }
        i = end + 1;
    }

    scan
}

/// 提示词校验报告（§3.6.5 语法 + 占位符完整性）。
#[derive(Debug, PartialEq, Eq)]
pub struct ValidationReport {
    /// 模板中出现的合法占位符键（去重）。
    pub keys: Vec<String>,
    /// 未在 `known` 中声明的键（完整性缺失）。
    pub unresolved: Vec<String>,
    /// 非法花括号片段。
    pub malformed: Vec<String>,
    /// 是否存在未闭合花括号。
    pub unclosed: bool,
}

impl ValidationReport {
    /// 无任何语法与完整性问题时为真。
    pub fn is_ok(&self) -> bool {
        self.unresolved.is_empty() && self.malformed.is_empty() && !self.unclosed
    }
}

/// 校验模板：扫描占位符，并以 `known`（已声明的占位符键集）判定完整性。
pub fn validate_prompt(template: &str, known: &HashSet<String>) -> ValidationReport {
    let scan = scan_placeholders(template);
    let unresolved = scan
        .keys
        .iter()
        .filter(|k| !known.contains(*k))
        .cloned()
        .collect();
    ValidationReport {
        keys: scan.keys,
        unresolved,
        malformed: scan.malformed,
        unclosed: scan.unclosed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn render_replaces_known_key() {
        let vars = vars_of(&[("Name", "Omega")]);
        assert_eq!(render("Hi {Name}!", &vars), "Hi Omega!");
    }

    #[test]
    fn render_keeps_unknown_key() {
        let vars = vars_of(&[("Name", "Omega")]);
        assert_eq!(render("{Name} and {Other}", &vars), "Omega and {Other}");
    }

    #[test]
    fn render_without_braces_is_unchanged() {
        let vars = vars_of(&[]);
        assert_eq!(render("plain text 中文正文", &vars), "plain text 中文正文");
    }

    #[test]
    fn render_handles_unclosed_brace() {
        let vars = vars_of(&[("Name", "Omega")]);
        assert_eq!(render("Hi {Name", &vars), "Hi {Name");
    }

    #[test]
    fn render_keeps_empty_and_nested_and_multiline() {
        let vars = vars_of(&[("a", "A")]);
        assert_eq!(render("{}", &vars), "{}");
        assert_eq!(render("{a{b}}", &vars), "{a{b}}");
        assert_eq!(render("{a\nb}", &vars), "{a\nb}");
    }

    #[test]
    fn render_preserves_chinese_around_placeholder() {
        let vars = vars_of(&[("名称", "欧米茄")]);
        assert_eq!(render("你好 {名称} 世界", &vars), "你好 欧米茄 世界");
    }

    #[test]
    fn build_vars_lists_and_custom() {
        let agents = vec!["Coder".to_string(), "Writer".to_string()];
        let tools = vec!["add".to_string()];
        let placeholders = vec![
            ("City".to_string(), Some("Beijing".to_string()), Some("N/A".to_string())),
            ("Lang".to_string(), None, Some("Rust".to_string())),
            ("Empty".to_string(), None, None),
        ];

        let vars = build_vars(&agents, &tools, &placeholders);
        assert_eq!(vars["AgentList"], "- Coder\n- Writer");
        assert_eq!(vars["MCPList"], "- add");
        // value 优先于 default_value。
        assert_eq!(vars["City"], "Beijing");
        // 无 value 回退 default_value。
        assert_eq!(vars["Lang"], "Rust");
        // 两者皆无为空串。
        assert_eq!(vars["Empty"], "");
    }

    #[test]
    fn build_vars_empty_lists_are_empty_string() {
        let vars = build_vars(&[], &[], &[]);
        assert_eq!(vars["AgentList"], "");
        assert_eq!(vars["MCPList"], "");
    }

    #[test]
    fn scan_collects_keys_dedup_and_malformed() {
        let scan = scan_placeholders("{A} {B} {A} {} {c{d}}");
        assert_eq!(scan.keys, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(scan.malformed, vec!["{}".to_string(), "{c{d}".to_string()]);
        assert!(!scan.unclosed);
    }

    #[test]
    fn scan_flags_unclosed_brace() {
        let scan = scan_placeholders("ok {A} then {B");
        assert_eq!(scan.keys, vec!["A".to_string()]);
        assert!(scan.unclosed);
    }

    #[test]
    fn scan_plain_text_has_nothing() {
        let scan = scan_placeholders("no placeholders 中文");
        assert!(scan.keys.is_empty());
        assert!(scan.malformed.is_empty());
        assert!(!scan.unclosed);
    }

    #[test]
    fn validate_prompt_reports_unresolved_and_syntax() {
        let mut known = HashSet::new();
        known.insert("AgentList".to_string());
        known.insert("City".to_string());

        // City/AgentList 已声明，Lang 未声明，另有一个非法片段。
        let report = validate_prompt("{City} {AgentList} {Lang} {}", &known);
        assert_eq!(report.unresolved, vec!["Lang".to_string()]);
        assert_eq!(report.malformed, vec!["{}".to_string()]);
        assert!(!report.is_ok());

        // 全部已声明且无非法片段时通过。
        let ok = validate_prompt("{City} {AgentList}", &known);
        assert!(ok.is_ok());
    }
}
