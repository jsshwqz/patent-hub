use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// AI client compatible with OpenAI API (works with Ollama, vLLM, etc.)
pub struct AiClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

impl AiClient {
    pub fn new() -> Self {
        let base_url = std::env::var("AI_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let api_key = std::env::var("AI_API_KEY")
            .unwrap_or_else(|_| "ollama".to_string());
        let model = std::env::var("AI_MODEL")
            .unwrap_or_else(|_| "qwen2.5:7b".to_string());

        Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
        }
    }

    pub async fn chat(&self, user_msg: &str, context: Option<&str>) -> Result<String> {
        let mut messages = vec![
            Message {
                role: "system".into(),
                content: "你是一个专利分析助手。你擅长分析专利文献、解读权利要求、评估专利价值、\
                         进行技术趋势分析。请用中文回答，专业术语可以保留英文。".into(),
            },
        ];

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

        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&ChatRequest {
                model: self.model.clone(),
                messages,
                temperature: 0.7,
            })
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(resp.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "AI 未返回有效回复".into()))
    }

    pub async fn summarize_patent(&self, patent_title: &str, abstract_text: &str, claims: &str) -> Result<String> {
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
            claims_preview = &claims[..claims.len().min(2000)]
        );
        self.chat(&prompt, None).await
    }
}
