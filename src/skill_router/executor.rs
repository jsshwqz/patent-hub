use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    time::SystemTime,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    security::Security,
    types::{ExecutionContext, ExecutionResponse, RouterPaths, SkillDefinition},
};

pub struct Executor;

impl Executor {
    /// Validate that a skill's permissions and entrypoint are safe to execute.
    pub fn validate_permissions(skill: &SkillDefinition, paths: &RouterPaths) -> Result<()> {
        Security::validate(skill, paths)?;

        if skill.metadata.permissions.process_exec {
            return Err(anyhow!(
                "skill '{}' requires process_exec which is denied by policy",
                skill.metadata.name
            ));
        }
        if skill.metadata.permissions.filesystem_write {
            return Err(anyhow!(
                "skill '{}' requires filesystem_write which is denied by policy",
                skill.metadata.name
            ));
        }

        Ok(())
    }

    /// Execute a skill and return the response.
    pub fn execute(
        skill: &SkillDefinition,
        context: &ExecutionContext,
        paths: &RouterPaths,
    ) -> Result<ExecutionResponse> {
        Self::validate_permissions(skill, paths)?;

        let response = if skill.metadata.entrypoint.starts_with("builtin:") {
            Self::execute_builtin(skill, context)?
        } else {
            return Err(anyhow!(
                "only builtin entrypoints are supported; got '{}'",
                skill.metadata.entrypoint
            ));
        };

        Self::append_log(skill, context, &response, paths);

        Ok(response)
    }

    fn execute_builtin(
        skill: &SkillDefinition,
        context: &ExecutionContext,
    ) -> Result<ExecutionResponse> {
        let builtin = skill
            .metadata
            .entrypoint
            .strip_prefix("builtin:")
            .unwrap_or("placeholder");

        let result = match builtin {
            "yaml_parse" => Self::builtin_yaml_parse(context),
            "json_parse" => Self::builtin_json_parse(context),
            "toml_parse" => Self::builtin_toml_parse(context),
            "csv_parse" => Self::builtin_csv_parse(context),
            "markdown_render" => Self::builtin_markdown_render(context),
            "text_diff" => Self::builtin_text_diff(context),
            "text_embed" => Self::builtin_text_embed(context),
            "text_summarize" => Self::builtin_text_summarize(context),
            "text_classify" => Self::builtin_text_classify(context),
            "text_extract" => Self::builtin_text_extract(context),
            "text_translate" => Self::builtin_text_translate(context),
            "web_search" => Self::builtin_web_search(context),
            "http_fetch" => Self::builtin_http_fetch(context),
            "echo" => Self::builtin_echo(context),
            _ => Self::builtin_placeholder(skill, context),
        };

        Ok(ExecutionResponse {
            status: "ok".to_string(),
            result,
            artifacts: Value::Object(Default::default()),
            error: None,
        })
    }

    // ── Data parsing builtins ────────────────────────────────────────────────

    fn builtin_yaml_parse(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        let mut result = serde_json::Map::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() {
                    result.insert(key.to_string(), Value::String(value.to_string()));
                }
            }
        }
        json!({ "capability": "yaml_parse", "parsed": result })
    }

    fn builtin_json_parse(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("{}");
        match serde_json::from_str::<Value>(text) {
            Ok(parsed) => json!({ "capability": "json_parse", "parsed": parsed }),
            Err(e) => json!({ "capability": "json_parse", "error": e.to_string() }),
        }
    }

    fn builtin_toml_parse(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        // Simple TOML key=value parser (handles flat TOML)
        let mut result = serde_json::Map::new();
        let mut current_section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let full_key = if current_section.is_empty() {
                    key.trim().to_string()
                } else {
                    format!("{}.{}", current_section, key.trim())
                };
                let value = value.trim().trim_matches('"');
                result.insert(full_key, Value::String(value.to_string()));
            }
        }
        json!({ "capability": "toml_parse", "parsed": result })
    }

    fn builtin_csv_parse(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        let mut lines = text.lines();
        let headers: Vec<String> = lines
            .next()
            .unwrap_or("")
            .split(',')
            .map(|h| h.trim().trim_matches('"').to_string())
            .collect();
        let rows: Vec<Vec<String>> = lines
            .map(|line| {
                line.split(',')
                    .map(|cell| cell.trim().trim_matches('"').to_string())
                    .collect()
            })
            .collect();
        json!({ "capability": "csv_parse", "headers": headers, "rows": rows })
    }

    fn builtin_markdown_render(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        let mut sections: Vec<Value> = Vec::new();
        let mut current_heading = String::new();
        let mut current_content = Vec::new();

        for line in text.lines() {
            if line.starts_with('#') {
                if !current_heading.is_empty() || !current_content.is_empty() {
                    sections.push(json!({
                        "heading": current_heading,
                        "content": current_content.join("\n"),
                    }));
                }
                current_heading = line.trim_start_matches('#').trim().to_string();
                current_content.clear();
            } else {
                current_content.push(line.to_string());
            }
        }
        if !current_heading.is_empty() || !current_content.is_empty() {
            sections.push(json!({
                "heading": current_heading,
                "content": current_content.join("\n"),
            }));
        }
        json!({ "capability": "markdown_render", "sections": sections })
    }

    // ── Text processing builtins ─────────────────────────────────────────────

    fn builtin_text_diff(context: &ExecutionContext) -> Value {
        let a = context.context["a"].as_str().unwrap_or("");
        let b = context.context["b"].as_str().unwrap_or("");
        let lines_a: Vec<&str> = a.lines().collect();
        let lines_b: Vec<&str> = b.lines().collect();
        let added: Vec<&&str> = lines_b.iter().filter(|l| !lines_a.contains(l)).collect();
        let removed: Vec<&&str> = lines_a.iter().filter(|l| !lines_b.contains(l)).collect();
        json!({
            "capability": "text_diff",
            "added": added.len(),
            "removed": removed.len(),
            "added_lines": added,
            "removed_lines": removed,
        })
    }

    fn builtin_text_embed(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        // Simple term-frequency bag-of-words
        let mut tf: HashMap<String, usize> = HashMap::new();
        for word in text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '\'' && c <= '\u{7F}')
            .filter(|w| w.len() >= 2)
        {
            *tf.entry(word.to_string()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = tf.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let top: Vec<_> = sorted.into_iter().take(50).collect();
        json!({ "capability": "text_embed", "vector": top })
    }

    fn builtin_text_summarize(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        match Self::call_ai(&format!(
            "请简要总结以下内容（不超过3句话）：\n\n{}",
            &text[..text.len().min(3000)]
        )) {
            Ok(summary) => json!({ "capability": "text_summarize", "output": summary }),
            Err(e) => json!({ "capability": "text_summarize", "error": e.to_string() }),
        }
    }

    fn builtin_text_classify(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        match Self::call_ai(&format!(
            "请将以下文本分类，返回一个类别标签（如：技术、商业、法律、医学、科学等），\
             只返回标签名，不要其他内容：\n\n{}",
            &text[..text.len().min(2000)]
        )) {
            Ok(label) => json!({ "capability": "text_classify", "output": label.trim() }),
            Err(e) => json!({ "capability": "text_classify", "error": e.to_string() }),
        }
    }

    fn builtin_text_extract(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        match Self::call_ai(&format!(
            "请从以下文本中提取关键实体（人名、组织、技术术语、日期、地点），\
             以JSON数组格式返回，每个实体包含type和value字段：\n\n{}",
            &text[..text.len().min(3000)]
        )) {
            Ok(entities) => {
                let parsed = serde_json::from_str::<Value>(&entities)
                    .unwrap_or_else(|_| json!({"raw": entities}));
                json!({ "capability": "text_extract", "output": parsed })
            }
            Err(e) => json!({ "capability": "text_extract", "error": e.to_string() }),
        }
    }

    fn builtin_text_translate(context: &ExecutionContext) -> Value {
        let text = context.context["text"].as_str().unwrap_or("");
        match Self::call_ai(&format!(
            "请将以下文本翻译为英文（如果已是英文则翻译为中文），只返回翻译结果：\n\n{}",
            &text[..text.len().min(3000)]
        )) {
            Ok(translated) => json!({ "capability": "text_translate", "output": translated }),
            Err(e) => json!({ "capability": "text_translate", "error": e.to_string() }),
        }
    }

    // ── Network builtins ─────────────────────────────────────────────────────

    fn builtin_web_search(context: &ExecutionContext) -> Value {
        let query = context.context["query"]
            .as_str()
            .or_else(|| context.context["text"].as_str())
            .unwrap_or("");

        let api_key = std::env::var("SERPAPI_KEY").unwrap_or_default();
        if api_key.is_empty() || api_key == "your-serpapi-key-here" {
            return json!({
                "capability": "web_search",
                "error": "SERPAPI_KEY not configured",
                "results": [],
            });
        }

        let url = format!(
            "https://serpapi.com/search.json?engine=google&q={}&num=5&api_key={}",
            urlencoding::encode(query),
            api_key
        );

        match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .and_then(|c| c.get(&url).send())
        {
            Ok(resp) => {
                if let Ok(body) = resp.json::<Value>() {
                    let results: Vec<Value> = body["organic_results"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .take(5)
                        .map(|r| {
                            json!({
                                "title": r["title"].as_str().unwrap_or(""),
                                "snippet": r["snippet"].as_str().unwrap_or(""),
                                "link": r["link"].as_str().unwrap_or(""),
                            })
                        })
                        .collect();
                    json!({ "capability": "web_search", "results": results })
                } else {
                    json!({ "capability": "web_search", "error": "Failed to parse response" })
                }
            }
            Err(e) => json!({ "capability": "web_search", "error": e.to_string() }),
        }
    }

    fn builtin_http_fetch(context: &ExecutionContext) -> Value {
        let url = context.context["url"]
            .as_str()
            .or_else(|| context.context["text"].as_str())
            .unwrap_or("");

        if !url.starts_with("https://") {
            return json!({
                "capability": "http_fetch",
                "error": "Only HTTPS URLs are supported",
            });
        }

        match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .and_then(|c| c.get(url).send())
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().unwrap_or_default();
                json!({
                    "capability": "http_fetch",
                    "status": status,
                    "body": &body[..body.len().min(5000)],
                })
            }
            Err(e) => json!({ "capability": "http_fetch", "error": e.to_string() }),
        }
    }

    // ── Utility builtins ─────────────────────────────────────────────────────

    fn builtin_echo(context: &ExecutionContext) -> Value {
        json!({
            "task": context.task,
            "capability": context.capability,
            "context": context.context,
        })
    }

    fn builtin_placeholder(skill: &SkillDefinition, context: &ExecutionContext) -> Value {
        json!({
            "capability": context.capability,
            "task": context.task,
            "skill": skill.metadata.name,
            "note": "placeholder builtin — no real processing performed",
        })
    }

    // ── AI helper (blocking, for use in synchronous executor) ────────────────

    fn call_ai(prompt: &str) -> Result<String> {
        let base_url = std::env::var("AI_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let api_key = std::env::var("AI_API_KEY").unwrap_or_else(|_| "ollama".to_string());
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| "qwen2.5:7b".to_string());

        let body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.3,
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let resp = client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()?
            .json::<Value>()?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| resp["result"].as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            Err(anyhow!("Empty AI response"))
        } else {
            Ok(content)
        }
    }

    fn append_log(
        skill: &SkillDefinition,
        context: &ExecutionContext,
        response: &ExecutionResponse,
        paths: &RouterPaths,
    ) {
        let _ = fs::create_dir_all(&paths.state_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&paths.executions_log)
        {
            let entry = json!({
                "timestamp": SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "skill": skill.metadata.name,
                "capability": context.capability,
                "task": context.task,
                "status": response.status,
            });
            let _ = writeln!(file, "{}", serde_json::to_string(&entry).unwrap_or_default());
        }
    }
}
