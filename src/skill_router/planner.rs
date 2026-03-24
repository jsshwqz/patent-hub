use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
};

use anyhow::Result;
use serde_json::json;

use super::capability_registry::CapabilityRegistry;

// ── In-process LRU-style cache (task_hash → capability) ──────────────────────
// Capped at 256 entries; oldest entry evicted when full.

static INFER_CACHE: OnceLock<Mutex<InferCache>> = OnceLock::new();

const MAX_CACHE_SIZE: usize = 256;

struct InferCache {
    map: HashMap<u64, String>,
    order: std::collections::VecDeque<u64>,
}

impl InferCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: std::collections::VecDeque::new(),
        }
    }

    fn get(&self, key: u64) -> Option<&String> {
        self.map.get(&key)
    }

    fn insert(&mut self, key: u64, value: String) {
        if self.map.contains_key(&key) {
            return;
        }
        if self.order.len() >= MAX_CACHE_SIZE {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.map.insert(key, value);
        self.order.push_back(key);
    }
}

fn cache() -> &'static Mutex<InferCache> {
    INFER_CACHE.get_or_init(|| Mutex::new(InferCache::new()))
}

fn hash_task(task: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    task.to_ascii_lowercase().hash(&mut hasher);
    hasher.finish()
}

// ── Keyword → capability mapping ─────────────────────────────────────────────

const KEYWORD_MAP: &[(&[&str], &str)] = &[
    (&["yaml", "yml"],                        "yaml_parse"),
    (&["json", "parse json"],                 "json_parse"),
    (&["toml"],                               "toml_parse"),
    (&["csv", "spreadsheet"],                 "csv_parse"),
    (&["pdf"],                                "pdf_parse"),
    (&["markdown", "md"],                     "markdown_render"),
    (&["summarize", "summary", "tldr"],       "text_summarize"),
    (&["translate", "translation"],           "text_translate"),
    (&["classify", "categorize", "label"],    "text_classify"),
    (&["extract", "entities", "ner"],         "text_extract"),
    (&["diff", "compare text"],              "text_diff"),
    (&["embed", "embedding", "vector"],       "text_embed"),
    (&["search", "google", "web search"],     "web_search"),
    (&["fetch", "http", "url", "download"],   "http_fetch"),
    (&["describe image", "image"],            "image_describe"),
    (&["generate code", "write code"],        "code_generate"),
    (&["test code", "unit test"],             "code_test"),
    (&["lint", "review code", "code review"], "code_lint"),
];

pub struct Planner;

impl Planner {
    /// Infer capability from task text using keyword matching, validated against registry.
    pub fn infer_capability(task: &str, registry: &CapabilityRegistry) -> Result<Option<String>> {
        let key = hash_task(task);
        if let Ok(c) = cache().lock() {
            if let Some(cached) = c.get(key) {
                return Ok(Some(cached.to_string()));
            }
        }

        let inferred = Self::infer_via_keywords(task, registry);

        if let Some(ref cap) = inferred {
            if let Ok(mut c) = cache().lock() {
                c.insert(key, cap.clone());
            }
        }

        Ok(inferred)
    }

    /// Version used by SkillRouter that can persist auto-discovered capabilities.
    pub fn infer_capability_with_paths(
        task: &str,
        registry: &mut CapabilityRegistry,
        paths: &super::types::RouterPaths,
    ) -> Result<Option<String>> {
        let key = hash_task(task);
        if let Ok(c) = cache().lock() {
            if let Some(cached) = c.get(key) {
                return Ok(Some(cached.to_string()));
            }
        }

        // Try AI inference first, then fall back to keywords
        let result = if let Ok(Some(cap)) = Self::infer_via_ai(task, registry) {
            Some(cap)
        } else {
            Self::infer_via_keywords(task, registry)
        };

        // If nothing matched, try AI auto-discovery of a new capability
        let result = if result.is_none() {
            Self::discover_capability_via_ai(task, registry, paths)
        } else {
            result
        };

        if let Some(ref cap) = result {
            if let Ok(mut c) = cache().lock() {
                c.insert(key, cap.clone());
            }
        }

        Ok(result)
    }

    fn infer_via_keywords(task: &str, registry: &CapabilityRegistry) -> Option<String> {
        let lower = task.to_ascii_lowercase();
        for (keywords, capability) in KEYWORD_MAP {
            if keywords.iter().any(|kw| lower.contains(kw)) && registry.contains(capability) {
                return Some(capability.to_string());
            }
        }
        None
    }

    fn infer_via_ai(task: &str, registry: &CapabilityRegistry) -> Result<Option<String>> {
        let base_url = std::env::var("AI_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let api_key = std::env::var("AI_API_KEY").unwrap_or_else(|_| "ollama".to_string());
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "qwen2.5:7b".to_string());

        let capabilities: Vec<&str> = registry
            .definitions()
            .map(|d| d.name.as_str())
            .collect();
        let cap_list = capabilities.join(", ");

        let prompt = format!(
            "A user wants to: \"{task}\"\n\
             Available capabilities: [{cap_list}]\n\
             Which capability best matches? Return ONLY the snake_case name, nothing else.\n\
             If none match, return NONE."
        );

        let body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.0,
            "max_tokens": 32,
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()?;

        let resp = client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()?
            .json::<serde_json::Value>()?;

        let raw = resp["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| resp["result"].as_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        let name: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if name == "none" || name.is_empty() {
            return Ok(None);
        }

        if registry.contains(&name) {
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }

    fn discover_capability_via_ai(
        task: &str,
        registry: &mut CapabilityRegistry,
        paths: &super::types::RouterPaths,
    ) -> Option<String> {
        let base_url = std::env::var("AI_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let api_key = std::env::var("AI_API_KEY").unwrap_or_else(|_| "ollama".to_string());
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "qwen2.5:7b".to_string());

        let prompt = format!(
            "A user wants to: \"{task}\"\n\
             No existing capability matches. Propose a new capability name in snake_case \
             (e.g. xml_parse, audio_transcribe).\n\
             Return ONLY the snake_case name, nothing else."
        );

        let body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.0,
            "max_tokens": 16,
        });

        let discovered = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .ok()
            .and_then(|c| {
                c.post(format!("{}/chat/completions", base_url))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&body)
                    .send()
                    .ok()
            })
            .and_then(|r| r.json::<serde_json::Value>().ok())
            .and_then(|v| {
                let raw = v["choices"][0]["message"]["content"]
                    .as_str()
                    .or_else(|| v["result"].as_str())
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase();
                let name: String = raw
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && registry.validate_name(&name).is_ok() {
                    Some(name)
                } else {
                    None
                }
            });

        if let Some(ref name) = discovered {
            let _ = registry.persist_to_dir(name, task, &paths.capabilities_dir);
        }

        discovered
    }
}
