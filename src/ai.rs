//! AI 多模型容灾客户端 / Multi-Provider AI Client with Failover
//!
//! 支持 6 种 AI 服务商自动切换：智谱 GLM、OpenRouter、Gemini、OpenAI、NVIDIA、DeepSeek。
//! Supports 6 AI providers with automatic failover.
//!
//! 功能 / Features:
//! - 摘要、对比、矩阵分析、权利要求分析、侵权评估 / Summary, comparison, claims analysis, risk assessment
//! - 创意深度分析 / Idea deep analysis
//! - 图片描述（视觉模型）/ Image description (vision model)
//! - 批量摘要 / Batch summarize

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// 单个 AI 服务商端点 / A single AI provider endpoint.
#[derive(Clone)]
struct AiProvider {
    name: String,
    base_url: String,
    api_key: String,
    model: String,
}

/// AI client with automatic failover across multiple providers.
pub struct AiClient {
    client: Client,
    primary: AiProvider,
    fallbacks: Vec<AiProvider>,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,
    content: String,
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

fn extract_chat_content(raw_text: &str) -> String {
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
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk backwards from max_bytes to find a valid char boundary
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
    async fn try_provider(
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
                            // "unexpected end of file" 等连接中断错误
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

    /// 全局超时上限，避免多 provider 叠加导致用户等待过久
    /// Global timeout cap to prevent multi-provider cascade from blocking too long
    const GLOBAL_TIMEOUT_SECS: u64 = 60;

    /// 带全局超时的 AI 调用入口 / Chat completion with global timeout across all providers
    async fn send_chat(&self, messages: Vec<Message>, temperature: f32) -> Result<String> {
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

    /// 内部实现：依次尝试 primary + fallbacks / Inner: try primary then fallbacks
    async fn send_chat_inner(&self, messages: Vec<Message>, temperature: f32) -> Result<String> {
        // 先尝试主 provider
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

        // 依次尝试 fallback / Try each fallback in order
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

    pub async fn chat(&self, user_msg: &str, context: Option<&str>) -> Result<String> {
        let mut messages = vec![Message {
            role: "system".into(),
            content: "你是一个专利分析助手。你擅长分析专利文献、解读权利要求、评估专利价值、\
                         进行技术趋势分析。请用中文回答，专业术语可以保留英文。"
                .into(),
        }];

        if let Some(ctx) = context {
            messages.push(Message {
                role: "system".into(),
                content: format!("以下是相关专利信息供参考：\n{ctx}"),
            });
        }

        messages.push(Message {
            role: "user".into(),
            content: user_msg.to_string(),
        });

        self.send_chat(messages, 0.7).await
    }

    /// 流式聊天：返回 SSE chunk 接收端 / Streaming chat returning an mpsc receiver of text chunks
    pub fn chat_stream(
        &self,
        user_msg: &str,
        context: Option<&str>,
    ) -> tokio::sync::mpsc::Receiver<String> {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(64);

        let mut messages = vec![Message {
            role: "system".into(),
            content: "你是一个专利分析助手。你擅长分析专利文献、解读权利要求、评估专利价值、\
                         进行技术趋势分析。请用中文回答，专业术语可以保留英文。"
                .into(),
        }];
        if let Some(ctx) = context {
            messages.push(Message {
                role: "system".into(),
                content: format!("以下是相关专利信息供参考：\n{ctx}"),
            });
        }
        messages.push(Message {
            role: "user".into(),
            content: user_msg.to_string(),
        });

        let provider = self.primary.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            // 全局超时保护，避免 AI 服务商不关连接导致 task 泄漏
            // Global timeout guard to prevent leaked tasks from hung connections
            let _ = tokio::time::timeout(
                Duration::from_secs(Self::GLOBAL_TIMEOUT_SECS),
                async {
                    let request_body = serde_json::json!({
                        "model": provider.model,
                        "messages": messages.iter().map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
                        "temperature": 0.7,
                        "stream": true,
                    });

                    let mut resp = match client
                        .post(format!("{}/chat/completions", provider.base_url))
                        .header("Authorization", format!("Bearer {}", provider.api_key))
                        .json(&request_body)
                        .send()
                        .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            let _ = tx.send(format!("[ERROR] {}", e)).await;
                            return;
                        }
                    };

                    if !resp.status().is_success() {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx.send(format!("[ERROR] HTTP {} — {}", status, safe_truncate(&body, 200))).await;
                        return;
                    }

                    // 逐块解析 SSE 流 / Parse SSE stream chunk by chunk
                    let mut buf = String::new();

                    loop {
                        match resp.chunk().await {
                            Ok(Some(chunk)) => {
                                buf.push_str(&String::from_utf8_lossy(&chunk));

                                while let Some(pos) = buf.find('\n') {
                                    let line = buf[..pos].trim().to_string();
                                    buf = buf[pos + 1..].to_string();

                                    if line == "data: [DONE]" {
                                        return;
                                    }
                                    if let Some(json_str) = line.strip_prefix("data: ") {
                                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                            if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                                                if !content.is_empty()
                                                    && tx.send(content.to_string()).await.is_err()
                                                {
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                },
            )
            .await;
        });

        rx
    }

    pub async fn analyze_idea(
        &self,
        title: &str,
        description: &str,
        web_findings: &str,
        patent_findings: &str,
    ) -> Result<String> {
        let prompt = format!(
            "## 用户的想法\n\
             **标题：** {title}\n\
             **描述：** {description}\n\n\
             ## 网络上的相关发现\n\
             {web_findings}\n\n\
             ## 相关专利\n\
             {patent_findings}\n\n\
             请从以下几个方面进行深入分析，用 Markdown 格式返回：\n\n\
             ### 1. 新颖性评估\n\
             - 与已有方案的相似度（给出 0-100 的新颖性评分，100=完全原创）\n\
             - 哪些部分是已有的，哪些是创新的\n\n\
             ### 2. 已有方案分析\n\
             - 列出最相关的已有方案/产品/专利\n\
             - 分析它们的优缺点\n\n\
             ### 3. 差异化方向\n\
             - 与已有方案的关键差异\n\
             - 可以进一步拉开差距的方向\n\n\
             ### 4. 优化建议\n\
             - 技术实现路径建议\n\
             - 可以增强竞争力的功能点\n\
             - 潜在的商业化方向\n\n\
             ### 5. 风险提示\n\
             - 可能的技术壁垒\n\
             - 潜在的知识产权风险\n\
             - 市场竞争风险\n\n\
             最后请在分析最开头用一行给出新颖性评分，格式严格为：\n\
             **新颖性评分：XX/100**"
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一个专业的创新分析师和技术顾问。你会客观地评估用户想法的新颖性，\
                         对比已有方案，并提供建设性的改进建议。回答要全面、有深度、实用。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.7).await
    }

    /// Send a vision request to describe an image (using multimodal API like GLM-4V).
    pub async fn describe_image(&self, image_data_url: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.primary.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "请详细描述这张图片中的技术内容。如果包含文字，请提取所有文字。\
                                 如果是技术图纸、流程图、结构图或专利附图，请详细描述其技术方案和结构特征。"
                    },
                    {
                        "type": "image_url",
                        "image_url": { "url": image_data_url }
                    }
                ]
            }],
            "temperature": 0.3
        });

        let max_retries = 2;
        let mut last_err = None;
        for attempt in 0..max_retries {
            if attempt > 0 {
                let delay = Duration::from_secs(3);
                tokio::time::sleep(delay).await;
            }

            match self
                .client
                .post(format!("{}/chat/completions", self.primary.base_url))
                .header("Authorization", format!("Bearer {}", self.primary.api_key))
                .json(&request_body)
                .send()
                .await
            {
                Ok(resp) => {
                    let raw_text = resp.text().await?;
                    let content = extract_chat_content(&raw_text);
                    if content.starts_with("AI 错误") && attempt < max_retries - 1 {
                        last_err = Some(anyhow::anyhow!("{}", content));
                        continue;
                    }
                    return Ok(content);
                }
                Err(e) => {
                    last_err = Some(e.into());
                    if attempt < max_retries - 1 {
                        continue;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Image analysis failed")))
    }

    pub async fn summarize_patent(
        &self,
        patent_title: &str,
        abstract_text: &str,
        claims: &str,
    ) -> Result<String> {
        let prompt = format!(
            "请对以下专利进行全面分析摘要：\n\n\
             标题：{patent_title}\n\n\
             摘要：{abstract_text}\n\n\
             权利要求（前部分）：{claims_preview}\n\n\
             请从以下几个方面分析：\n\
             1. 技术领域\n\
             2. 核心技术方案\n\
             3. 创新点\n\
             4. 应用场景\n\
             5. 关键权利要求解读",
            claims_preview = safe_truncate(claims, 2000)
        );
        self.chat(&prompt, None).await
    }

    /// Analyze patent claims: identify independent vs dependent, extract scope elements.
    pub async fn analyze_claims(&self, patent_title: &str, claims: &str) -> Result<String> {
        let prompt = format!(
            "请对以下专利的权利要求进行深度分析：\n\n\
             专利标题：{patent_title}\n\n\
             权利要求全文：\n{claims_text}\n\n\
             请按以下格式分析（使用 Markdown 表格）：\n\n\
             ### 1. 权利要求结构总览\n\
             列出每条权利要求的编号、类型（独立/从属）、所从属的权利要求号\n\n\
             ### 2. 独立权利要求分析\n\
             对每条独立权利要求：\n\
             - 保护范围要素（技术特征列表）\n\
             - 保护范围宽度评估（宽/中/窄）\n\
             - 可能的规避设计方向\n\n\
             ### 3. 从属权利要求层级\n\
             用缩进或树形结构展示权利要求之间的从属关系\n\n\
             ### 4. 关键技术特征\n\
             提取最核心的限定性技术特征（决定保护范围的关键要素）\n\n\
             ### 5. 保护强度评估\n\
             综合评估该专利权利要求的保护强度（强/中/弱），并说明原因",
            claims_text = safe_truncate(claims, 4000)
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深专利代理人和知识产权律师。你擅长解读专利权利要求书，\
                         分析保护范围，识别关键技术特征。请用专业、严谨的语言分析。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Assess infringement risk: compare a product/tech description against multiple patents.
    pub async fn assess_infringement(
        &self,
        product_description: &str,
        patents_info: &str,
    ) -> Result<String> {
        let prompt = format!(
            "## 待评估的产品/技术方案\n{product}\n\n\
             ## 对比专利列表\n{patents}\n\n\
             请对每个专利逐一进行侵权风险评估，按以下格式输出（使用 Markdown 表格）：\n\n\
             ### 侵权风险评估矩阵\n\
             | 专利号 | 风险等级 | 关键风险点 | 规避建议 |\n\
             |--------|----------|------------|----------|\n\n\
             风险等级说明：\n\
             - **高风险**: 产品技术方案与专利权利要求高度重合\n\
             - **中风险**: 部分技术特征重合，需进一步分析\n\
             - **低风险**: 技术方案存在明显差异\n\
             - **无风险**: 不在专利保护范围内\n\n\
             ### 详细分析\n\
             对每个高/中风险专利，详细说明：\n\
             1. 哪些技术特征与专利权利要求对应\n\
             2. 字面侵权还是等同侵权的可能性\n\
             3. 具体的规避设计建议\n\n\
             ### 综合建议\n\
             整体风险评估和应对策略建议",
            product = safe_truncate(product_description, 2000),
            patents = safe_truncate(patents_info, 4000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深知识产权律师和专利侵权分析专家。你擅长评估产品的专利侵权风险，\
                         对比技术方案与专利权利要求的对应关系。请客观、专业地分析，并提供可操作的建议。".into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Compare multiple patents across multiple dimensions.
    pub async fn compare_multiple(&self, patents_info: &str) -> Result<String> {
        let prompt = format!(
            "请对以下多个专利进行多维度对比分析：\n\n{patents}\n\n\
             请按以下格式输出（使用 Markdown 表格）：\n\n\
             ### 1. 基本信息对比\n\
             | 维度 | 专利1 | 专利2 | ... |\n\
             |------|-------|-------|-----|\n\
             | 技术领域 | | | |\n\
             | 核心问题 | | | |\n\
             | 申请人 | | | |\n\n\
             ### 2. 技术方案对比\n\
             | 维度 | 专利1 | 专利2 | ... |\n\
             |------|-------|-------|-----|\n\
             | 核心方案 | | | |\n\
             | 创新点 | | | |\n\
             | 技术路线 | | | |\n\n\
             ### 3. 优缺点分析\n\
             | 专利 | 优点 | 缺点 | 应用场景 |\n\n\
             ### 4. 综合评价\n\
             - 技术演进趋势\n\
             - 最具创新性的方案\n\
             - 互补性分析",
            patents = safe_truncate(patents_info, 6000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位专利技术分析专家，擅长对比分析多个专利的技术方案，\
                         识别技术演进趋势和创新差异。请用结构化的表格形式呈现分析结果。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.5).await
    }

    /// Batch summarize multiple patents concurrently.
    pub async fn batch_summarize(
        &self,
        patents: &[(String, String, String)],
    ) -> Vec<(String, Result<String>)> {
        let mut results = Vec::new();
        for (id, title, abstract_text) in patents {
            let result = self
                .chat(
                    &format!(
                        "请用2-3句话简要总结这个专利的核心技术方案：\n标题：{}\n摘要：{}",
                        title,
                        safe_truncate(abstract_text, 500)
                    ),
                    None,
                )
                .await;
            results.push((id.clone(), result));
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::extract_chat_content;

    #[test]
    fn extracts_openai_message_content() {
        let raw = r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}]}"#;
        assert_eq!(extract_chat_content(raw), "hello");
    }

    #[test]
    fn extracts_provider_result_field() {
        let raw = r#"{"result":"provider text"}"#;
        assert_eq!(extract_chat_content(raw), "provider text");
    }

    #[test]
    fn formats_provider_error_message() {
        let raw = r#"{"error":{"message":"bad request"}}"#;
        assert_eq!(extract_chat_content(raw), "AI 错误：bad request");
    }
}
