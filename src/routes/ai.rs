use super::AppState;
use crate::patent::*;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde_json::json;
use std::convert::Infallible;
use reqwest::Client;

/// Quick web search: SerpAPI → Sogou free fallback. Returns formatted context string.
async fn quick_web_search(query: &str, serpapi_key: &str) -> Option<String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let has_serp = !serpapi_key.is_empty() && serpapi_key != "your-serpapi-key-here";
    let mut results: Vec<(String, String, String)> = Vec::new(); // (title, snippet, link)

    if has_serp {
        if let Ok(resp) = client
            .get("https://serpapi.com/search.json")
            .query(&[
                ("q", query),
                ("api_key", serpapi_key),
                ("num", "5"),
                ("hl", "zh-cn"),
            ])
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(items) = json["organic_results"].as_array() {
                    for r in items.iter().take(5) {
                        results.push((
                            r["title"].as_str().unwrap_or("").to_string(),
                            r["snippet"].as_str().unwrap_or("").to_string(),
                            r["link"].as_str().unwrap_or("").to_string(),
                        ));
                    }
                }
            }
        }
    }

    // Sogou free fallback
    if results.is_empty() {
        let encoded = urlencoding::encode(query);
        let url = format!("https://www.sogou.com/web?query={}", encoded);
        if let Ok(resp) = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
        {
            if let Ok(html) = resp.text().await {
                // Simple extraction of results from Sogou HTML
                for cap in html.split("vrTitle").skip(1).take(5) {
                    if let Some(title_start) = cap.find('>') {
                        let after = &cap[title_start + 1..];
                        if let Some(title_end) = after.find("</") {
                            let title = after[..title_end]
                                .replace("<em>", "").replace("</em>", "")
                                .replace("<!--", "").replace("-->", "")
                                .trim().to_string();
                            // Extract snippet
                            let snippet = if let Some(abs_start) = cap.find("strAbstract") {
                                let abs = &cap[abs_start..];
                                if let Some(s) = abs.find('>') {
                                    let a = &abs[s + 1..];
                                    if let Some(e) = a.find("</") {
                                        a[..e].replace("<em>", "").replace("</em>", "").trim().to_string()
                                    } else { String::new() }
                                } else { String::new() }
                            } else { String::new() };
                            if !title.is_empty() {
                                results.push((title, snippet, String::new()));
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        return None;
    }

    let mut context = String::from("【联网搜索结果】\n");
    for (i, (title, snippet, link)) in results.iter().enumerate() {
        context.push_str(&format!("{}. {}\n", i + 1, title));
        if !snippet.is_empty() {
            context.push_str(&format!("   {}\n", snippet));
        }
        if !link.is_empty() {
            context.push_str(&format!("   {}\n", link));
        }
    }
    Some(context)
}

/// Estimate token count (CJK ~1.5 tok/char, ASCII ~0.25 tok/char)
fn estimate_tokens(s: &str) -> usize {
    let mut t = 0usize;
    for ch in s.chars() {
        t += if ch > '\u{2E80}' { 3 } else { 1 };
    }
    t / 2 + 1
}

/// When history is too long, summarize early messages via AI and combine with recent ones.
/// Returns (condensed_history, whether summarization was applied).
async fn compress_history(
    ai: &crate::ai::AiClient,
    history: Vec<(String, String)>,
    max_tokens: usize,
) -> Vec<(String, String)> {
    let total: usize = history.iter().map(|(_, c)| estimate_tokens(c)).sum();
    if total <= max_tokens || history.len() <= 10 {
        return history;
    }

    // Keep the last 8 messages (4 rounds) intact, summarize everything before that
    let keep_recent = 8.min(history.len());
    let split_at = history.len() - keep_recent;
    let (old_part, recent_part) = history.split_at(split_at);

    // Build text of old conversation for summarization
    let mut old_text = String::new();
    for (role, content) in old_part {
        let label = if role == "user" { "用户" } else { "助手" };
        old_text.push_str(&format!("{}：{}\n", label, content));
    }

    // Ask AI to compress — use a short, focused prompt
    let summary = ai.chat(
        &format!(
            "请将以下对话历史压缩为一段简洁的摘要（保留所有关键信息、结论、数据和用户偏好，不超过500字）：\n\n{}",
            old_text
        ),
        None,
    ).await;

    match summary {
        Ok(summary_text) => {
            let mut result = Vec::with_capacity(1 + recent_part.len());
            result.push((
                "system".to_string(),
                format!("【前期对话摘要】{}", summary_text),
            ));
            result.extend_from_slice(recent_part);
            result
        }
        Err(_) => {
            // Summarization failed, just send all and let the model handle it
            history
        }
    }
}

/// POST /api/ai/chat/stream — SSE 流式 AI 聊天 / Streaming AI chat via SSE
pub async fn api_ai_chat_stream(
    State(s): State<AppState>,
    Json(req): Json<AiChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let ai = s.config.read().unwrap().ai_client();
    let ctx = req
        .patent_id
        .as_ref()
        .and_then(|pid| s.db.get_patent(pid).ok().flatten())
        .map(|p| {
            let claims_preview: String = p.claims.chars().take(3000).collect();
            format!(
                "Patent: {}\nTitle: {}\nAbstract: {}\nClaims: {}",
                p.patent_number, p.title, p.abstract_text, claims_preview
            )
        });

    let mut rx = ai.chat_stream(&req.message, ctx.as_deref());

    let stream = async_stream::stream! {
        while let Some(chunk) = rx.recv().await {
            if chunk.starts_with("[ERROR]") {
                yield Ok(Event::default().event("error").data(chunk));
                break;
            }
            yield Ok(Event::default().data(chunk));
        }
        yield Ok(Event::default().event("done").data("[DONE]"));
    };

    Sse::new(stream)
}

pub async fn api_ai_chat(
    State(s): State<AppState>,
    Json(req): Json<AiChatRequest>,
) -> Json<AiResponse> {
    let ai = s.config.read().unwrap().ai_client();
    let serpapi_key = s.config.read().unwrap().serpapi_key.clone();

    // Optional web search: fetch real-time info before AI response
    let web_context = if req.web_search {
        quick_web_search(&req.message, &serpapi_key).await
    } else {
        None
    };

    let ctx = req
        .patent_id
        .as_ref()
        .and_then(|pid| s.db.get_patent(pid).ok().flatten())
        .map(|p| {
            let claims_preview: String = p.claims.chars().take(3000).collect();
            format!(
                "Patent: {}\nTitle: {}\nAbstract: {}\nClaims: {}",
                p.patent_number, p.title, p.abstract_text, claims_preview
            )
        });

    // Build system prompt with optional web search results
    let base_prompt = ctx.as_deref().unwrap_or("你是一个技术研发助手，擅长专利分析、技术方案评估和可行性验证。请用中文回答。");
    let system_prompt = match &web_context {
        Some(web) => format!("{}\n\n以下是联网搜索到的最新资料，请结合这些信息回答用户问题：\n{}", base_prompt, web),
        None => base_prompt.to_string(),
    };

    let result = if req.history.is_empty() {
        let ctx_with_web = match &web_context {
            Some(web) => Some(format!("{}\n{}", ctx.as_deref().unwrap_or(""), web)),
            None => ctx,
        };
        ai.chat(&req.message, ctx_with_web.as_deref()).await
    } else {
        let mut history = req.history;
        history.push(("user".to_string(), req.message));
        // 超过 ~8000 token 时自动压缩早期对话为摘要
        let history = compress_history(&ai, history, 8000).await;
        ai.chat_with_history(&system_prompt, history, 0.7).await
    };

    match result {
        Ok(content) => Json(AiResponse { content }),
        Err(e) => Json(AiResponse {
            content: format!("AI error: {e}"),
        }),
    }
}

pub async fn api_ai_summarize(
    State(s): State<AppState>,
    Json(req): Json<FetchPatentRequest>,
) -> Json<AiResponse> {
    let ai = s.config.read().unwrap().ai_client();
    match s.db.get_patent(&req.patent_number) {
        Ok(Some(p)) => match ai
            .summarize_patent(&p.title, &p.abstract_text, &p.claims)
            .await
        {
            Ok(content) => Json(AiResponse { content }),
            Err(e) => Json(AiResponse {
                content: format!("AI error: {e}"),
            }),
        },
        _ => Json(AiResponse {
            content: "Patent not found".into(),
        }),
    }
}

pub async fn api_ai_compare(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<AiResponse> {
    let id1 = req["patent_id1"].as_str().unwrap_or("");
    let id2 = req["patent_id2"].as_str().unwrap_or("");

    let p1 = match s.db.get_patent(id1) {
        Ok(Some(p)) => p,
        _ => {
            return Json(AiResponse {
                content: format!("专利1「{}」未找到。请确认该专利已通过搜索页收录到本地库。", id1),
            })
        }
    };
    let p2 = match s.db.get_patent(id2) {
        Ok(Some(p)) => p,
        _ => {
            return Json(AiResponse {
                content: format!("专利2「{}」未找到。请确认该专利已通过搜索页收录到本地库。", id2),
            })
        }
    };

    let ai = s.config.read().unwrap().ai_client();
    let p1_abstract: String = p1.abstract_text.chars().take(500).collect();
    let p1_claims: String = p1.claims.chars().take(1000).collect();
    let p2_abstract: String = p2.abstract_text.chars().take(500).collect();
    let p2_claims: String = p2.claims.chars().take(1000).collect();

    let prompt = format!(
        "请对比分析以下两个专利的异同：\n\n\
         【专利1】\n\
         专利号：{}\n标题：{}\n申请人：{}\n\
         摘要：{}\n权利要求（前部分）：{}\n\n\
         【专利2】\n\
         专利号：{}\n标题：{}\n申请人：{}\n\
         摘要：{}\n权利要求（前部分）：{}\n\n\
         请从以下方面对比：\n\
         1. 技术领域是否相同\n\
         2. 解决的技术问题对比\n\
         3. 技术方案的异同点\n\
         4. 创新点对比\n\
         5. 保护范围对比\n\
         6. 是否存在侵权风险（初步判断）",
        p1.patent_number,
        p1.title,
        p1.applicant,
        p1_abstract,
        p1_claims,
        p2.patent_number,
        p2.title,
        p2.applicant,
        p2_abstract,
        p2_claims
    );

    match ai.chat(&prompt, None).await {
        Ok(content) => Json(AiResponse { content }),
        Err(e) => Json(AiResponse {
            content: format!("AI error: {e}"),
        }),
    }
}

pub async fn api_ai_analyze_results(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let query = req["query"].as_str().unwrap_or("");
    let patents = req["patents"].as_array();

    if query.is_empty() || patents.is_none() {
        return Json(json!({"error": "缺少查询词或专利数据"}));
    }
    let patents = patents.unwrap();

    let mut patent_list = String::new();
    for (i, p) in patents.iter().enumerate().take(10) {
        let title = p["title"].as_str().unwrap_or("");
        let abstract_text = p["abstract_text"].as_str().unwrap_or("");
        let applicant = p["applicant"].as_str().unwrap_or("");
        let preview: String = abstract_text.chars().take(100).collect();
        patent_list.push_str(&format!(
            "{}. 标题：{}\n   申请人：{}\n   摘要：{}\n\n",
            i + 1,
            title,
            applicant,
            preview
        ));
    }

    let prompt = format!(
        "你是一个专利分析专家和研发创新顾问。用户正在研究「{}」方向。\n\n\
         以下是搜索到的相关专利列表：\n{}\n\n\
         请完成以下分析（用JSON格式返回）：\n\n\
         1. **语义相关性评分**：对每条专利给出0-100的语义相关性评分\n\
         2. **技术趋势**：这些专利反映了什么技术发展趋势\n\
         3. **技术空白**：哪些方向还没有被充分覆盖\n\
         4. **创新建议**：针对用户的研究方向，给出2-3个具体的创新切入点\n\n\
         请严格按以下JSON格式返回（不要包含其他文字）：\n\
         {{\n\
           \"scores\": [{{\"index\": 1, \"score\": 85, \"reason\": \"简短原因\"}}, ...],\n\
           \"trend\": \"技术趋势分析文字\",\n\
           \"gaps\": \"技术空白分析文字\",\n\
           \"suggestions\": [\"建议1\", \"建议2\", \"建议3\"]\n\
         }}",
        query, patent_list
    );

    let ai = s.config.read().unwrap().ai_client();
    match ai.chat(&prompt, None).await {
        Ok(content) => {
            let trimmed = content.trim();
            let json_str = if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.rfind('}') {
                    &trimmed[start..=end]
                } else {
                    trimmed
                }
            } else {
                trimmed
            };

            match serde_json::from_str::<serde_json::Value>(json_str) {
                Ok(parsed) => Json(json!({"status": "ok", "analysis": parsed})),
                Err(_) => Json(json!({"status": "ok", "analysis": {"raw": content}})),
            }
        }
        Err(e) => Json(json!({"error": format!("AI分析失败: {}。请在设置页面配置AI服务。", e)})),
    }
}

/// Claims scope analysis
pub async fn api_ai_claims_analysis(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let patent_id = req["patent_id"].as_str().unwrap_or("");
    let patent = match s.db.get_patent(patent_id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };

    if patent.claims.trim().is_empty() || patent.claims.trim().len() < 10 {
        return Json(json!({"error": "该专利没有权利要求数据，请先获取完整专利信息"}));
    }

    let ai = s.config.read().unwrap().ai_client();
    match ai.analyze_claims(&patent.title, &patent.claims).await {
        Ok(content) => Json(json!({"status": "ok", "analysis": content})),
        Err(e) => Json(json!({"error": format!("分析失败: {}", e)})),
    }
}

/// Infringement risk assessment
pub async fn api_ai_risk_assessment(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let product_desc = req["product_description"].as_str().unwrap_or("").trim();
    let patent_ids = req["patent_ids"].as_array();

    if product_desc.is_empty() {
        return Json(json!({"error": "请输入产品/技术方案描述"}));
    }

    let ids = match patent_ids {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return Json(json!({"error": "请选择至少一个专利"})),
    };

    if ids.is_empty() || ids.len() > 10 {
        return Json(json!({"error": "请选择 1-10 个专利进行评估"}));
    }

    let mut patents_info = String::new();
    let mut not_found: Vec<String> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        if let Ok(Some(p)) = s.db.get_patent(id) {
            let claims_preview: String = p.claims.chars().take(800).collect();
            patents_info.push_str(&format!(
                "### 专利 {} - {}\n专利号：{}\n申请人：{}\n摘要：{}\n权利要求：{}\n\n",
                i + 1,
                p.title,
                p.patent_number,
                p.applicant,
                p.abstract_text,
                claims_preview
            ));
        } else {
            not_found.push(id.to_string());
        }
    }

    if patents_info.is_empty() {
        return Json(json!({"error": format!(
            "未找到指定的专利「{}」。请确认这些专利已通过搜索页收录到本地库（支持专利号或内部 ID）。",
            not_found.join(", ")
        )}));
    }

    let ai = s.config.read().unwrap().ai_client();
    match ai.assess_infringement(product_desc, &patents_info).await {
        Ok(content) => Json(json!({"status": "ok", "analysis": content})),
        Err(e) => Json(json!({"error": format!("评估失败: {}", e)})),
    }
}

/// Multi-patent comparison matrix
pub async fn api_ai_compare_matrix(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let patent_ids = req["patent_ids"].as_array();

    let ids = match patent_ids {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return Json(json!({"error": "请选择至少2个专利"})),
    };

    if ids.len() < 2 || ids.len() > 5 {
        return Json(json!({"error": "请选择 2-5 个专利进行对比"}));
    }

    let mut patents_info = String::new();
    for (i, id) in ids.iter().enumerate() {
        if let Ok(Some(p)) = s.db.get_patent(id) {
            let claims_preview: String = p.claims.chars().take(600).collect();
            patents_info.push_str(&format!(
                "### 专利 {}\n专利号：{}\n标题：{}\n申请人：{}\n摘要：{}\n权利要求：{}\n\n",
                i + 1,
                p.patent_number,
                p.title,
                p.applicant,
                p.abstract_text,
                claims_preview
            ));
        }
    }

    let ai = s.config.read().unwrap().ai_client();
    match ai.compare_multiple(&patents_info).await {
        Ok(content) => Json(json!({"status": "ok", "analysis": content})),
        Err(e) => Json(json!({"error": format!("对比失败: {}", e)})),
    }
}

/// Batch summarize patents
pub async fn api_ai_batch_summarize(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let patent_ids = req["patent_ids"].as_array();

    let ids = match patent_ids {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return Json(json!({"error": "请选择专利"})),
    };

    if ids.is_empty() || ids.len() > 20 {
        return Json(json!({"error": "请选择 1-20 个专利"}));
    }

    let mut patents_data: Vec<(String, String, String)> = Vec::new();
    // Keep patent_number + title for response enrichment
    let mut patent_meta: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    for id in &ids {
        if let Ok(Some(p)) = s.db.get_patent(id) {
            patent_meta.insert(p.id.clone(), (p.patent_number.clone(), p.title.clone()));
            patents_data.push((p.id.clone(), p.title.clone(), p.abstract_text.clone()));
        }
    }

    let ai = s.config.read().unwrap().ai_client();
    let results = ai.batch_summarize(&patents_data).await;

    let summaries: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(id, result)| {
            let (pn, title) = patent_meta
                .get(&id)
                .cloned()
                .unwrap_or_default();
            match result {
                Ok(summary) => json!({"id": id, "patent_number": pn, "title": title, "summary": summary}),
                Err(e) => json!({"id": id, "patent_number": pn, "title": title, "error": format!("{}", e)}),
            }
        })
        .collect();

    Json(json!({"status": "ok", "summaries": summaries}))
}
