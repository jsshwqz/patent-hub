use super::AppState;
use axum::{extract::State, Json};
use serde_json::json;

pub async fn api_get_settings(State(s): State<AppState>) -> Json<serde_json::Value> {
    let config = s.config.read().unwrap();

    fn mask_api_key(key: &str) -> String {
        if key.is_empty() || key == "your-serpapi-key-here" {
            String::new()
        } else if key.len() <= 8 {
            "****".to_string()
        } else {
            format!("{}****{}", &key[..4], &key[key.len() - 4..])
        }
    }

    Json(json!({
        "serpapi_key": mask_api_key(&config.serpapi_key),
        "serpapi_key_configured": config.has_serpapi(),
        "ai_base_url": config.ai_base_url,
        "ai_api_key": mask_api_key(&config.ai_api_key),
        "ai_api_key_configured": !config.ai_api_key.is_empty(),
        "ai_model": config.ai_model,
    }))
}

pub async fn api_save_serpapi(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let api_key = req["api_key"].as_str().unwrap_or("").trim();

    if api_key.is_empty() {
        return Json(json!({"status": "error", "message": "API key is required"}));
    }
    if api_key.len() < 20 || api_key.len() > 200 {
        return Json(json!({"status": "error", "message": "Invalid API key format"}));
    }
    if !api_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Json(json!({"status": "error", "message": "API key contains invalid characters"}));
    }

    // 先更新内存配置（立即生效）
    s.config.write().unwrap().serpapi_key = api_key.to_string();
    // SQLite 持久化（主存储，Android 友好）
    if let Err(e) = s.db.set_setting("SERPAPI_KEY", api_key) {
        tracing::warn!("保存设置到数据库失败: {}", e);
    }
    // .env 持久化为可选（桌面端后备）
    let _ = update_env_file("SERPAPI_KEY", api_key);
    Json(json!({"status": "ok"}))
}

pub async fn api_save_ai(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let base_url = req["base_url"].as_str().unwrap_or("").trim();
    let api_key = req["api_key"].as_str().unwrap_or("").trim();
    let model = req["model"].as_str().unwrap_or("").trim();

    if base_url.is_empty() || api_key.is_empty() || model.is_empty() {
        return Json(json!({"status": "error", "message": "All fields are required"}));
    }
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Json(json!({"status": "error", "message": "URL must use HTTP or HTTPS protocol"}));
    }
    if api_key.len() < 8 || api_key.len() > 200 {
        return Json(json!({"status": "error", "message": "API key length must be between 8 and 200 characters"}));
    }
    if model.len() < 2 || model.len() > 100 {
        return Json(json!({"status": "error", "message": "Model name must be between 2 and 100 characters"}));
    }
    if !model
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':' || c == '/')
    {
        return Json(json!({"status": "error", "message": "Model name contains invalid characters"}));
    }

    // 先更新内存配置（立即生效）
    {
        let mut config = s.config.write().unwrap();
        config.ai_base_url = base_url.to_string();
        config.ai_api_key = api_key.to_string();
        config.ai_model = model.to_string();
    }

    // SQLite 持久化（主存储，Android 友好）
    for (k, v) in [("AI_BASE_URL", base_url), ("AI_API_KEY", api_key), ("AI_MODEL", model)] {
        if let Err(e) = s.db.set_setting(k, v) {
            tracing::warn!("保存设置 {} 到数据库失败: {}", k, e);
        }
    }
    // .env 持久化为可选（桌面端后备）
    let _ = update_env_file("AI_BASE_URL", base_url);
    let _ = update_env_file("AI_API_KEY", api_key);
    let _ = update_env_file("AI_MODEL", model);

    Json(json!({"status": "ok"}))
}

pub async fn api_import_patents(
    State(s): State<AppState>,
    Json(req): Json<crate::patent::ImportRequest>,
) -> Json<serde_json::Value> {
    let mut n = 0;
    for p in &req.patents {
        if s.db.insert_patent(p).is_ok() {
            n += 1;
        }
    }
    Json(json!({"status":"ok","imported":n}))
}

pub async fn api_save_fallbacks(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let fallbacks = match req["fallbacks"].as_array() {
        Some(arr) => arr,
        None => return Json(json!({"status":"error","message":"Missing fallbacks array"})),
    };

    let mut new_fallbacks = Vec::new();
    for (i, fb) in fallbacks.iter().enumerate() {
        let idx = i + 1;
        let name = fb["name"].as_str().unwrap_or("").to_string();
        let url = fb["url"].as_str().unwrap_or("").to_string();
        let key = fb["key"].as_str().unwrap_or("").to_string();
        let model = fb["model"].as_str().unwrap_or("").to_string();

        if url.is_empty() || key.is_empty() {
            continue;
        }

        // SQLite 持久化（主存储）
        for (suffix, val) in [("NAME", name.as_str()), ("URL", url.as_str()), ("KEY", key.as_str()), ("MODEL", model.as_str())] {
            let db_key = format!("FALLBACK_AI_{}_{}", idx, suffix);
            if let Err(e) = s.db.set_setting(&db_key, val) {
                tracing::warn!("保存设置 {} 到数据库失败: {}", db_key, e);
            }
        }
        // .env 持久化为可选（桌面端后备）
        let _ = update_env_file(&format!("FALLBACK_AI_{}_NAME", idx), &name);
        let _ = update_env_file(&format!("FALLBACK_AI_{}_URL", idx), &url);
        let _ = update_env_file(&format!("FALLBACK_AI_{}_KEY", idx), &key);
        let _ = update_env_file(&format!("FALLBACK_AI_{}_MODEL", idx), &model);

        new_fallbacks.push(super::AiFallback { name, base_url: url, api_key: key, model });
    }

    // Clear remaining slots
    for idx in (fallbacks.len() + 1)..=5 {
        // 清除 SQLite 中的旧槽位
        for suffix in ["NAME", "URL", "KEY", "MODEL"] {
            let db_key = format!("FALLBACK_AI_{}_{}", idx, suffix);
            let _ = s.db.set_setting(&db_key, "");
        }
        let _ = update_env_file(&format!("FALLBACK_AI_{}_URL", idx), "");
        let _ = update_env_file(&format!("FALLBACK_AI_{}_KEY", idx), "");
    }

    // Update in-memory config
    {
        let mut config = s.config.write().unwrap();
        config.ai_fallbacks = new_fallbacks;
    }

    Json(json!({"status":"ok","message":format!("已保存 {} 个备用 AI", fallbacks.len())}))
}

fn update_env_file(key: &str, value: &str) -> Result<(), String> {
    let env_path = ".env";
    let content = std::fs::read_to_string(env_path).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut found = false;

    for line in &mut lines {
        if line.starts_with(&format!("{}=", key)) {
            *line = format!("{}={}", key, value);
            found = true;
            break;
        }
    }

    if !found {
        lines.push(format!("{}={}", key, value));
    }

    std::fs::write(env_path, lines.join("\n"))
        .map_err(|e| format!("Failed to write .env file: {}", e))?;

    Ok(())
}
