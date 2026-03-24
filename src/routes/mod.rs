mod ai;
mod collections;
mod idea;
mod pages;
mod patent;
mod search;
mod settings;
mod upload;

pub use ai::*;
pub use collections::*;
pub use idea::*;
pub use pages::*;
pub use patent::*;
pub use search::*;
pub use settings::*;
pub use upload::*;

use crate::{ai::AiClient, db::Database};
use std::sync::{Arc, RwLock};

/// Shared application configuration (replaces env::set_var).
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub serpapi_key: String,
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
    /// Load from environment (at startup).
    pub fn from_env() -> Self {
        let mut fallbacks = Vec::new();

        // Load fallback providers from FALLBACK_AI_* env vars
        // Format: FALLBACK_AI_1_URL, FALLBACK_AI_1_KEY, FALLBACK_AI_1_MODEL, FALLBACK_AI_1_NAME
        for i in 1..=5 {
            let url = std::env::var(format!("FALLBACK_AI_{}_URL", i)).unwrap_or_default();
            let key = std::env::var(format!("FALLBACK_AI_{}_KEY", i)).unwrap_or_default();
            let model = std::env::var(format!("FALLBACK_AI_{}_MODEL", i)).unwrap_or_default();
            let name = std::env::var(format!("FALLBACK_AI_{}_NAME", i))
                .unwrap_or_else(|_| format!("Fallback-{}", i));
            if !url.is_empty() && !key.is_empty() && !model.is_empty() {
                fallbacks.push(AiFallback { name, base_url: url, api_key: key, model });
            }
        }

        Self {
            serpapi_key: std::env::var("SERPAPI_KEY").unwrap_or_default(),
            ai_base_url: std::env::var("AI_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".into()),
            ai_api_key: std::env::var("AI_API_KEY").unwrap_or_else(|_| "ollama".into()),
            ai_model: std::env::var("AI_MODEL").unwrap_or_else(|_| "qwen2.5:7b".into()),
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
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Arc<RwLock<AppConfig>>,
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
