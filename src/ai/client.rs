//! AI 多模型容灾客户端核心 / Core AI client with multi-provider failover

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// 单个 AI 服务商端点 / A single AI provider endpoint.
#[derive(Clone)]
pub(super) struct AiProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// AI client with automatic failover across multiple providers.
#[derive(Clone)]
pub struct AiClient {
    pub(super) client: Client,
    pub(super) primary: AiProvider,
    pub(super) fallbacks: Vec<AiProvider>,
}

#[derive(Serialize)]
pub(super) struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(super) struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

// Flexible response parser for different AI providers
#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    data: Option<Value>,    // Zhipu format
    result: Option<String>, // Some providers use this
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ResponseMessage>,
    delta: Option<ResponseMessage>, // Streaming format
}

pub(crate) fn extract_chat_content(raw_text: &str) -> String {
    if let Ok(resp) = serde_json::from_str::<ChatResponse>(raw_text) {
        if let Some(choices) = resp.choices {
            for choice in choices {
                if let Some(content) = choice
                    .message
                    .and_then(|message| message.content)
                    .or_else(|| choice.delta.and_then(|delta| delta.content))
                {
                    return content;
                }
            }
        }

        if let Some(data) = resp.data {
            if let Some(content) = extract_content_from_data(&data) {
                return content;
            }
        }

        if let Some(result) = resp.result {
            return result;
        }
    }

    if let Ok(json) = serde_json::from_str::<Value>(raw_text) {
        if let Some(content) = extract_content_from_data(&json) {
            return content;
        }
        if let Some(err) = json["error"]["message"].as_str() {
            return format!("AI 错误：{}", err);
        }
        if let Some(msg) = json["msg"].as_str() {
            return format!("AI 错误：{}", msg);
        }
    }

    format!(
        "AI 响应解析失败，原始响应：{}",
        raw_text.chars().take(200).collect::<String>()
    )
}

/// Safely truncate a UTF-8 string to at most `max_bytes` bytes
/// without splitting multi-byte characters.
pub(super) fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

fn extract_content_from_data(data: &Value) -> Option<String> {
    if let Some(choices) = data["choices"].as_array() {
        for choice in choices {
            if let Some(content) = choice["message"]["content"].as_str() {
                return Some(content.to_string());
            }
            if let Some(content) = choice["delta"]["content"].as_str() {
                return Some(content.to_string());
            }
        }
    }

    if let Some(content) = data["data"]["content"].as_str() {
        return Some(content.to_string());
    }

    if let Some(content) = data["data"]["choices"][0]["message"]["content"].as_str() {
        return Some(content.to_string());
    }

    None
}

impl AiClient {
    /// Create from explicit config values (preferred).
    pub fn with_config(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
            primary: AiProvider {
                name: "primary".to_string(),
                base_url: base_url.to_string(),
                api_key: api_key.to_string(),
                model: model.to_string(),
            },
            fallbacks: Vec::new(),
        }
    }

    /// Add a fallback AI provider.
    pub fn add_fallback(&mut self, base_url: &str, api_key: &str, model: &str, name: &str) {
        self.fallbacks.push(AiProvider {
            name: name.to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        });
    }

    /// Try a single provider with retries.
    pub(super) async fn try_provider(
        &self,
        provider: &AiProvider,
        messages: &[Message],
        temperature: f32,
    ) -> Result<String> {
        let request_body = ChatRequest {
            model: provider.model.clone(),
            messages: messages.to_vec(),
            temperature,
        };

        let max_retries = 2;
        let mut last_err = None;
        for attempt in 0..max_retries {
            if attempt > 0 {
                let delay = Duration::from_secs(2);
                tokio::time::sleep(delay).await;
            }

            match self
                .client
                .post(format!("{}/chat/completions", provider.base_url))
                .header("Authorization", format!("Bearer {}", provider.api_key))
                .json(&request_body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();

                    if status.as_u16() == 429 {
                        let raw_text = resp.text().await.unwrap_or_default();
                        tracing::warn!(
                            "[{}] rate limited (429): {}",
                            provider.name,
                            safe_truncate(&raw_text, 100)
                        );
                        return Err(anyhow::anyhow!("AI 频率限制，请稍后再试"));
                    }

                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        let raw_text = resp.text().await.unwrap_or_default();
                        tracing::warn!(
                            "[{}] auth error ({}): {}",
                            provider.name,
                            status.as_u16(),
                            safe_truncate(&raw_text, 200)
                        );
                        return Err(anyhow::anyhow!(
                            "AI API Key 无效或已过期。请到「设置」页面检查 API Key 配置。"
                        ));
                    }

                    let raw_text = match resp.text().await {
                        Ok(text) => text,
                        Err(e) => {
                            if attempt < max_retries - 1 {
                                last_err = Some(anyhow::anyhow!("AI 响应读取中断: {}", e));
                                continue;
                            }
                            return Err(anyhow::anyhow!(
                                "AI 响应读取失败（连接中断）。可能原因：\n\
                                 1. API Key 无效或余额不足\n\
                                 2. 网络不稳定\n\
                                 3. AI 服务暂时不可用\n\
                                 请到「设置」检查 AI 配置。"
                            ));
                        }
                    };
                    let content = extract_chat_content(&raw_text);

                    if status.is_server_error() && attempt < max_retries - 1 {
                        last_err = Some(anyhow::anyhow!("Server error {}", status));
                        continue;
                    }

                    if content.starts_with("AI 错误") && attempt < max_retries - 1 {
                        last_err = Some(anyhow::anyhow!("{}", content));
                        continue;
                    }

                    return Ok(content);
                }
                Err(e) => {
                    if e.is_connect() && provider.base_url.contains("localhost") {
                        return Err(anyhow::anyhow!(
                            "AI 未配置。请打开「设置」页面，配置云端 AI 服务（如智谱 GLM、OpenRouter 等）。\
                             当前默认连接本地 Ollama (localhost:11434)，手机端不可用。"
                        ));
                    }
                    if attempt < max_retries - 1 && (e.is_timeout() || e.is_connect()) {
                        last_err = Some(e.into());
                        continue;
                    }
                    return Err(e.into());
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Provider {} failed", provider.name)))
    }

    /// 全局超时上限
    pub(super) const GLOBAL_TIMEOUT_SECS: u64 = 120;

    /// 带全局超时的 AI 调用入口
    pub(super) async fn send_chat(
        &self,
        messages: Vec<Message>,
        temperature: f32,
    ) -> Result<String> {
        match tokio::time::timeout(
            Duration::from_secs(Self::GLOBAL_TIMEOUT_SECS),
            self.send_chat_inner(messages, temperature),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!(
                "AI 调用超时（全局上限 {}s）。请检查网络或更换 AI 服务商。",
                Self::GLOBAL_TIMEOUT_SECS
            )),
        }
    }

    /// 内部实现：依次尝试 primary + fallbacks
    async fn send_chat_inner(&self, messages: Vec<Message>, temperature: f32) -> Result<String> {
        match self
            .try_provider(&self.primary, &messages, temperature)
            .await
        {
            Ok(content) => return Ok(content),
            Err(e) => {
                if self.fallbacks.is_empty() {
                    return Err(e);
                }
                tracing::warn!("[{}] failed: {}, trying fallbacks...", self.primary.name, e);
            }
        }

        let mut last_err = anyhow::anyhow!("All providers failed");
        for fallback in &self.fallbacks {
            tracing::info!("[failover] trying {}...", fallback.name);
            match self.try_provider(fallback, &messages, temperature).await {
                Ok(content) => {
                    tracing::info!("[failover] {} succeeded", fallback.name);
                    return Ok(content);
                }
                Err(e) => {
                    tracing::warn!("[failover] {} failed: {}", fallback.name, e);
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }
}
