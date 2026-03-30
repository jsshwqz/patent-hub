//! API 路由层 / API Routes
//!
//! 所有 HTTP 端点的实现，按功能模块拆分。
//! All HTTP endpoint implementations, organized by feature module.
//!
//! - [`ai`] — AI 聊天、摘要、对比 / AI chat, summary, comparison
//! - [`search`] — 专利搜索 / Patent search
//! - [`idea`] — 创意验证 + 多轮对话 / Idea validation + multi-round chat
//! - [`patent`] — 专利详情 / Patent details
//! - [`collections`] — 收藏夹管理 / Collections management
//! - [`settings`] — 系统设置 / System settings
//! - [`ipc`] — IPC 分类 / IPC classification
//! - [`upload`] — 文件上传 / File upload
//! - [`pages`] — 页面渲染 / Page rendering

mod ai;
mod collections;
mod idea;
mod ipc;
mod pages;
mod patent;
mod search;
mod settings;
mod upload;

pub use ai::*;
pub use collections::*;
pub use idea::*;
pub use ipc::*;
pub use pages::*;
pub use patent::*;
pub use search::*;
pub use settings::*;
pub use upload::*;

use crate::{ai::AiClient, db::Database, pipeline::context::PipelineProgress};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;

/// Shared application configuration (replaces env::set_var).
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub serpapi_key: String,
    /// Bing Web Search API key (Azure Cognitive Services) — 国内可用，替代 SerpAPI
    pub bing_api_key: String,
    /// Lens.org Patent API key — 国内可用，替代 Google Patents
    pub lens_api_key: String,
    pub ai_base_url: String,
    pub ai_api_key: String,
    pub ai_model: String,
    // Fallback AI providers for automatic failover
    pub ai_fallbacks: Vec<AiFallback>,
}

#[derive(Debug, Clone)]
pub struct AiFallback {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl AppConfig {
    /// Load from environment only (without DB). Kept for tests/fallback.
    #[allow(dead_code)]
    pub fn from_env() -> Self {
        Self::from_db_and_env(None)
    }

    /// 从 SQLite 数据库加载设置，环境变量作为后备。
    /// SQLite 是主存储（Android 友好），.env 是次要存储（桌面端后备）。
    pub fn from_db_and_env(db: Option<&Database>) -> Self {
        // 先从数据库加载所有设置
        let db_settings = db
            .and_then(|d| d.get_all_settings().ok())
            .unwrap_or_default();

        // 辅助：优先取 DB 值，其次取环境变量，最后用默认值
        let get = |key: &str, default: &str| -> String {
            if let Some(v) = db_settings.get(key) {
                if !v.is_empty() {
                    return v.clone();
                }
            }
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        };

        let mut fallbacks = Vec::new();
        for i in 1..=5 {
            let url = get(&format!("FALLBACK_AI_{}_URL", i), "");
            let key = get(&format!("FALLBACK_AI_{}_KEY", i), "");
            let model = get(&format!("FALLBACK_AI_{}_MODEL", i), "");
            let name = get(
                &format!("FALLBACK_AI_{}_NAME", i),
                &format!("Fallback-{}", i),
            );
            if !url.is_empty() && !key.is_empty() && !model.is_empty() {
                fallbacks.push(AiFallback {
                    name,
                    base_url: url,
                    api_key: key,
                    model,
                });
            }
        }

        Self {
            serpapi_key: get("SERPAPI_KEY", ""),
            bing_api_key: get("BING_API_KEY", ""),
            lens_api_key: get("LENS_API_KEY", ""),
            ai_base_url: get("AI_BASE_URL", "http://localhost:11434/v1"),
            ai_api_key: get("AI_API_KEY", "ollama"),
            ai_model: get("AI_MODEL", "qwen2.5:7b"),
            ai_fallbacks: fallbacks,
        }
    }

    /// Build an AiClient from the current config (with fallback support).
    pub fn ai_client(&self) -> AiClient {
        let mut client = AiClient::with_config(&self.ai_base_url, &self.ai_api_key, &self.ai_model);
        for fb in &self.ai_fallbacks {
            client.add_fallback(&fb.base_url, &fb.api_key, &fb.model, &fb.name);
        }
        client
    }

    /// Whether SerpAPI is configured and usable.
    pub fn has_serpapi(&self) -> bool {
        !self.serpapi_key.is_empty() && self.serpapi_key != "your-serpapi-key-here"
    }

    /// Whether Bing Search API is configured (国内可用替代方案).
    pub fn has_bing(&self) -> bool {
        !self.bing_api_key.is_empty()
    }

    /// Whether Lens.org patent API is configured (国内可用替代方案).
    pub fn has_lens(&self) -> bool {
        !self.lens_api_key.is_empty()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Arc<RwLock<AppConfig>>,
    /// Pipeline progress channels for SSE streaming
    pub pipeline_channels: Arc<Mutex<HashMap<String, broadcast::Sender<PipelineProgress>>>>,
}

use crate::patent::SearchType;

pub(crate) fn parse_search_type(search_type: Option<&str>) -> Option<SearchType> {
    search_type.map(|t| match t {
        "applicant" => SearchType::Applicant,
        "inventor" => SearchType::Inventor,
        "patent_number" => SearchType::PatentNumber,
        "keyword" => SearchType::Keyword,
        _ => SearchType::Mixed,
    })
}

pub(crate) fn build_online_query(
    query: &str,
    search_type: Option<&SearchType>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> String {
    let q = query.trim().replace('"', "");
    let mut search_query = match search_type {
        Some(SearchType::Applicant) => format!("assignee:\"{}\"", q),
        Some(SearchType::Inventor) => format!("inventor:\"{}\"", q),
        Some(SearchType::PatentNumber) => format!("\"{}\"", q),
        _ => q,
    };
    if let Some(from) = date_from {
        if !from.is_empty() {
            search_query.push_str(&format!(" after:{from}"));
        }
    }
    if let Some(to) = date_to {
        if !to.is_empty() {
            search_query.push_str(&format!(" before:{to}"));
        }
    }
    search_query
}

/// HTML-escape to prevent XSS in template interpolation.
pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Escape a CSV field.
pub(crate) fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Recursively extract a named field from a JSON value (for EPO responses).
pub(crate) fn efld(json: &serde_json::Value, field: &str) -> String {
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            if k == field {
                return match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(a) => a
                        .iter()
                        .filter_map(|x| x.as_str().or_else(|| x["$"].as_str()))
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => v.to_string(),
                };
            }
            let r = efld(v, field);
            if !r.is_empty() {
                return r;
            }
        }
    } else if let Some(arr) = json.as_array() {
        for v in arr {
            let r = efld(v, field);
            if !r.is_empty() {
                return r;
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::build_online_query;
    use crate::patent::SearchType;

    #[test]
    fn online_query_uses_applicant_scope() {
        let q = build_online_query("Alice Zhang", Some(&SearchType::Applicant), None, None);
        assert_eq!(q, "assignee:\"Alice Zhang\"");
    }

    #[test]
    fn online_query_uses_inventor_scope_and_dates() {
        let q = build_online_query(
            "Alice Zhang",
            Some(&SearchType::Inventor),
            Some("2024-01-01"),
            Some("2024-12-31"),
        );
        assert_eq!(
            q,
            "inventor:\"Alice Zhang\" after:2024-01-01 before:2024-12-31"
        );
    }
}
