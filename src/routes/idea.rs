use super::AppState;
use crate::patent::*;
use crate::pipeline::context::PipelineProgress;
use crate::pipeline::runner::PipelineRunner;
use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde_json::json;
use std::convert::Infallible;

pub async fn api_idea_submit(
    State(s): State<AppState>,
    Json(req): Json<IdeaSubmitRequest>,
) -> Json<serde_json::Value> {
    let title = req.title.trim().to_string();
    let description = req.description.trim().to_string();
    if title.is_empty() {
        return Json(json!({"status": "error", "message": "标题不能为空"}));
    }
    if title.chars().count() > 200 {
        return Json(json!({"status": "error", "message": "标题不能超过200字"}));
    }
    if description.is_empty() {
        return Json(json!({"status": "error", "message": "描述不能为空"}));
    }
    if description.chars().count() > 10000 {
        return Json(json!({"status": "error", "message": "描述不能超过10000字"}));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let idea = Idea {
        id: id.clone(),
        title: title.clone(),
        description: description.clone(),
        input_type: if req.input_type.is_empty() {
            "text".into()
        } else {
            req.input_type
        },
        status: "pending".into(),
        analysis: String::new(),
        web_results: "[]".into(),
        patent_results: "[]".into(),
        novelty_score: None,
        created_at: now.clone(),
        updated_at: now,
        discussion_summary: String::new(),
    };

    if let Err(e) = s.db.insert_idea(&idea) {
        return Json(json!({"status": "error", "message": e.to_string()}));
    }

    Json(json!({"status": "ok", "id": id}))
}

pub async fn api_idea_analyze(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = req["id"].as_str().unwrap_or("");
    if id.is_empty() {
        return Json(json!({"status": "error", "message": "missing idea id"}));
    }

    let mut idea = match s.db.get_idea(id) {
        Ok(Some(i)) => i,
        _ => return Json(json!({"status": "error", "message": "idea not found"})),
    };

    idea.status = "analyzing".into();
    if let Err(e) = s.db.update_idea(&idea) {
        tracing::error!("Failed to update idea {} status: {}", idea.id, e);
    }

    let config = s.config.read().unwrap().clone();

    // Step 1: Web search via SerpAPI (general web)
    let web_findings =
        search_web_for_idea(&config.serpapi_key, &idea.title, &idea.description).await;
    let web_summary = summarize_web_results(&web_findings);

    // Step 2: Google Patents search via SerpAPI (patent-specific)
    let online_patent_findings =
        search_patents_online_for_idea(&config.serpapi_key, &idea.title, &idea.description).await;

    // Step 3: Local patent search
    let local_patent_findings = search_patents_local(&s.db, &idea.title);

    // Merge patent results (online first, then local, deduplicated)
    let mut all_patent_findings = online_patent_findings;
    for local in &local_patent_findings {
        let num = local["patent_number"].as_str().unwrap_or("");
        let already_exists = all_patent_findings
            .iter()
            .any(|p| p["patent_number"].as_str().unwrap_or("") == num);
        if !already_exists {
            all_patent_findings.push(local.clone());
        }
    }

    let patent_summary = summarize_patent_results(&all_patent_findings);

    idea.web_results =
        serde_json::to_string(&web_findings).unwrap_or_else(|_| "[]".into());
    idea.patent_results =
        serde_json::to_string(&all_patent_findings).unwrap_or_else(|_| "[]".into());

    // Step 4: AI analysis（失败时降级到代码计算评分）
    let ai = config.ai_client();
    match ai
        .analyze_idea(
            &idea.title,
            &idea.description,
            &web_summary,
            &patent_summary,
        )
        .await
    {
        Ok(analysis) => {
            idea.novelty_score = extract_novelty_score(&analysis);
            idea.analysis = analysis;
            idea.status = "done".into();
        }
        Err(e) => {
            // AI 不可用时，用简单启发式估算新颖性
            let patent_count = all_patent_findings.len();
            let web_count = web_findings.len();
            let fallback_score = if patent_count == 0 && web_count < 3 {
                95.0 // 几乎无先例，高度原创
            } else if patent_count == 0 {
                85.0 // 无专利，有网络资料
            } else if patent_count < 3 {
                72.0 // 少量相关专利
            } else {
                55.0 // 较多相关专利，需人工评估
            };
            tracing::warn!("AI 分析失败，使用启发式评分 {}: {}", fallback_score, e);
            idea.novelty_score = Some(fallback_score);
            idea.analysis = format!(
                "AI 分析暂不可用（{}）。\n\n基于检索结果的启发式评估：\n- 找到 {} 篇网络资料\n- 找到 {} 条相关专利\n- 估算新颖性评分：{}/100\n\n建议配置可用的 AI 服务以获得深度分析。",
                e, web_count, patent_count, fallback_score as u32
            );
            idea.status = "done".into();
        }
    }

    if let Err(e) = s.db.update_idea(&idea) {
        tracing::error!("Failed to save idea {} analysis: {}", idea.id, e);
    }

    Json(json!({
        "status": "ok",
        "idea": {
            "id": idea.id,
            "title": idea.title,
            "status": idea.status,
            "analysis": idea.analysis,
            "novelty_score": idea.novelty_score,
            "web_results": serde_json::from_str::<serde_json::Value>(&idea.web_results).unwrap_or_default(),
            "patent_results": serde_json::from_str::<serde_json::Value>(&idea.patent_results).unwrap_or_default(),
        }
    }))
}

pub async fn api_idea_get(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_idea(&id) {
        Ok(Some(idea)) => Json(json!({
            "status": "ok",
            "idea": {
                "id": idea.id,
                "title": idea.title,
                "description": idea.description,
                "status": idea.status,
                "analysis": idea.analysis,
                "novelty_score": idea.novelty_score,
                "web_results": serde_json::from_str::<serde_json::Value>(&idea.web_results).unwrap_or_default(),
                "patent_results": serde_json::from_str::<serde_json::Value>(&idea.patent_results).unwrap_or_default(),
                "created_at": idea.created_at,
                "discussion_summary": idea.discussion_summary,
            }
        })),
        _ => Json(json!({"status": "error", "message": "not found"})),
    }
}

pub async fn api_idea_list(State(s): State<AppState>) -> Json<serde_json::Value> {
    match s.db.list_ideas() {
        Ok(ideas) => Json(json!({"status": "ok", "ideas": ideas})),
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
}

// ── Idea helper functions ────────────────────────────────────────────────────

/// Search general web via SerpAPI Google search.
async fn search_web_for_idea(
    api_key: &str,
    title: &str,
    description: &str,
) -> Vec<serde_json::Value> {
    if api_key.is_empty() || api_key == "your-serpapi-key-here" {
        return Vec::new();
    }

    let desc_preview: String = description.chars().take(100).collect();
    let query = format!("{} {}", title, desc_preview);

    let client = reqwest::Client::new();
    let url = format!(
        "https://serpapi.com/search.json?engine=google&q={}&num=10&api_key={}",
        urlencoding::encode(&query),
        api_key
    );

    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(results) = body["organic_results"].as_array() {
                    return results
                        .iter()
                        .take(10)
                        .map(|r| {
                            json!({
                                "title": r["title"].as_str().unwrap_or(""),
                                "snippet": r["snippet"].as_str().unwrap_or(""),
                                "link": r["link"].as_str().unwrap_or(""),
                                "source": r["source"].as_str().unwrap_or(""),
                            })
                        })
                        .collect();
                }
            }
            Vec::new()
        }
        Err(e) => {
            println!("[IDEA] Web search error: {}", e);
            Vec::new()
        }
    }
}

/// Search Google Patents via SerpAPI for patent-specific results.
async fn search_patents_online_for_idea(
    api_key: &str,
    title: &str,
    description: &str,
) -> Vec<serde_json::Value> {
    if api_key.is_empty() || api_key == "your-serpapi-key-here" {
        return Vec::new();
    }

    let desc_preview: String = description.chars().take(80).collect();
    let query = format!("{} {}", title, desc_preview);

    let client = reqwest::Client::new();
    let url = format!(
        "https://serpapi.com/search.json?engine=google_patents&q={}&num=10&api_key={}",
        urlencoding::encode(&query),
        api_key
    );

    println!("[IDEA] Searching Google Patents: {}", title);
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(err) = body.get("error") {
                    println!("[IDEA] Google Patents search error: {}", err);
                    return Vec::new();
                }
                if let Some(results) = body["organic_results"].as_array() {
                    println!("[IDEA] Found {} patent results", results.len());
                    return results
                        .iter()
                        .take(10)
                        .map(|r| {
                            json!({
                                "patent_number": r["publication_number"].as_str().unwrap_or(""),
                                "title": r["title"].as_str().unwrap_or(""),
                                "abstract": r["snippet"].as_str().unwrap_or(""),
                                "applicant": r["assignee"].as_str().unwrap_or(""),
                                "filing_date": r["filing_date"].as_str().unwrap_or(""),
                                "source": "google_patents",
                            })
                        })
                        .collect();
                }
            }
            Vec::new()
        }
        Err(e) => {
            println!("[IDEA] Google Patents search error: {}", e);
            Vec::new()
        }
    }
}

/// Search local patent database.
fn search_patents_local(
    db: &crate::db::Database,
    title: &str,
) -> Vec<serde_json::Value> {
    let keywords: Vec<&str> = title.split_whitespace().take(5).collect();
    let query = keywords.join(" ");
    if query.is_empty() {
        return Vec::new();
    }

    match db.search_smart(
        &query,
        Some(&SearchType::Keyword),
        None,
        None,
        None,
        1,
        10,
    ) {
        Ok((patents, _, _)) => patents
            .iter()
            .map(|p| {
                json!({
                    "patent_number": p.patent_number,
                    "title": p.title,
                    "abstract": p.abstract_text,
                    "applicant": p.applicant,
                    "filing_date": p.filing_date,
                    "source": "local",
                })
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn summarize_web_results(results: &[serde_json::Value]) -> String {
    if results.is_empty() {
        return "未找到相关网络结果（SerpAPI 未配置或无结果）".into();
    }
    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. **{}**\n   {}\n   来源：{}",
                i + 1,
                r["title"].as_str().unwrap_or(""),
                r["snippet"].as_str().unwrap_or(""),
                r["link"].as_str().unwrap_or(""),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn summarize_patent_results(results: &[serde_json::Value]) -> String {
    if results.is_empty() {
        return "未找到相关专利".into();
    }
    results
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "{}. [{}] {}\n   申请人：{} | 来源：{} | 摘要：{}",
                i + 1,
                p["patent_number"].as_str().unwrap_or(""),
                p["title"].as_str().unwrap_or(""),
                p["applicant"].as_str().unwrap_or(""),
                p["source"].as_str().unwrap_or("unknown"),
                p["abstract"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn extract_novelty_score(analysis: &str) -> Option<f64> {
    // Try multiple patterns for robustness
    for line in analysis.lines() {
        if line.contains("新颖性评分") || line.contains("novelty") || line.contains("评分") {
            // Extract numbers from the line, looking for XX/100 pattern
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i].is_ascii_digit() {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    if let Ok(n) = num_str.parse::<f64>() {
                        if (0.0..=100.0).contains(&n) {
                            // Check if followed by /100 pattern
                            if i < chars.len() && chars[i] == '/' {
                                return Some(n);
                            }
                            // Or just a reasonable score in context
                            if n >= 1.0 {
                                return Some(n);
                            }
                        }
                    }
                }
                i += 1;
            }
        }
    }
    None
}

// ── Idea multi-round chat ────────────────────────────────────────────

/// Send a message in an idea discussion (multi-round with context)
pub async fn api_idea_chat(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let user_msg = req["message"].as_str().unwrap_or("").trim();
    if user_msg.is_empty() {
        return Json(json!({"error": "消息不能为空"}));
    }
    if user_msg.len() > 5000 {
        return Json(json!({"error": "消息过长（最多5000字符）"}));
    }

    // Get the idea for context
    let idea = match s.db.get_idea(&idea_id) {
        Ok(Some(i)) => i,
        _ => return Json(json!({"error": "创意不存在"})),
    };

    // Get previous messages for context
    let history = s.db.get_idea_messages(&idea_id).unwrap_or_default();

    // Build context-aware prompt with full conversation history
    let mut system_context = format!(
        "你是一位创新分析师。正在与用户讨论一个创意想法。\n\n\
         ## 创意信息\n\
         **标题：** {}\n\
         **描述：** {}\n\
         **状态：** {}\n",
        idea.title, idea.description, idea.status
    );

    // Add analysis results if available
    if !idea.analysis.is_empty() {
        let analysis_preview: String = idea.analysis.chars().take(1000).collect();
        system_context.push_str(&format!("\n## 之前的分析结果\n{}\n", analysis_preview));
    }

    if let Some(score) = idea.novelty_score {
        system_context.push_str(&format!("\n**新颖性评分：** {}/100\n", score));
    }

    // Add discussion summary if exists
    let summary = s.db.get_idea_summary(&idea_id).unwrap_or_default();
    if !summary.is_empty() {
        system_context.push_str(&format!("\n## 之前的讨论总结\n{}\n", summary));
    }

    system_context.push_str(
        "\n请基于以上背景信息与用户继续讨论。回答要专业、有深度、有建设性。\
         帮助用户完善创意、规避风险、找到差异化方向。"
    );

    // Build message history
    let mut messages = vec![];

    // Include recent history (last 10 messages to stay within token limits)
    let recent_history: Vec<_> = if history.len() > 10 {
        history[history.len() - 10..].to_vec()
    } else {
        history.clone()
    };

    for (_id, role, content, _ts) in &recent_history {
        messages.push(json!({"role": role, "content": content}));
    }

    // Add current user message
    messages.push(json!({"role": "user", "content": user_msg}));

    // Save user message to DB
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = s.db.add_idea_message(&user_msg_id, &idea_id, "user", user_msg) {
        return Json(json!({"error": format!("保存消息失败: {}", e)}));
    }

    // Call AI with full context
    let ai = s.config.read().unwrap().ai_client();
    let ai_response = match ai.chat(user_msg, Some(&system_context)).await {
        Ok(content) => content,
        Err(e) => {
            return Json(json!({"error": format!("AI 响应失败: {}", e)}));
        }
    };

    // Save AI response to DB
    let ai_msg_id = uuid::Uuid::new_v4().to_string();
    let _ = s.db.add_idea_message(&ai_msg_id, &idea_id, "assistant", &ai_response);

    Json(json!({
        "status": "ok",
        "message": {
            "id": ai_msg_id,
            "role": "assistant",
            "content": ai_response
        }
    }))
}

/// Get all messages for an idea discussion
pub async fn api_idea_messages(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_idea_messages(&idea_id) {
        Ok(msgs) => {
            let list: Vec<serde_json::Value> = msgs
                .into_iter()
                .map(|(id, role, content, created_at)| {
                    json!({
                        "id": id,
                        "role": role,
                        "content": content,
                        "created_at": created_at,
                    })
                })
                .collect();
            Json(json!({"messages": list}))
        }
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

// ── Pipeline API ─────────────────────────────────────────────────────

/// POST /api/idea/pipeline — 启动 12 步创新验证流水线
pub async fn api_idea_pipeline(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = req["id"].as_str().unwrap_or("");
    if id.is_empty() {
        return Json(json!({"status": "error", "message": "缺少创意 ID"}));
    }

    let idea = match s.db.get_idea(id) {
        Ok(Some(i)) => i,
        _ => return Json(json!({"status": "error", "message": "创意不存在"})),
    };

    // Create broadcast channel for progress
    let (tx, _) = tokio::sync::broadcast::channel::<PipelineProgress>(64);
    {
        let mut channels = s.pipeline_channels.lock().unwrap();
        channels.insert(id.to_string(), tx.clone());
    }

    // Build runner from config
    let config = s.config.read().unwrap().clone();
    let ai_client = config.ai_client();
    let db = s.db.clone();
    let serpapi_key = config.serpapi_key.clone();
    let bing_api_key = config.bing_api_key.clone();
    let lens_api_key = config.lens_api_key.clone();
    let runner = PipelineRunner::new(ai_client, db.clone(), serpapi_key, bing_api_key, lens_api_key);

    let idea_id = id.to_string();
    let title = idea.title.clone();
    let description = idea.description.clone();
    let channels = s.pipeline_channels.clone();

    // Run pipeline in background
    tokio::spawn(async move {
        tracing::info!("Pipeline 开始执行: {}", idea_id);
        let result = runner.run(&idea_id, &title, &description, Some(tx)).await;
        tracing::info!("Pipeline 执行完毕: {} => {:?}", idea_id, result.is_ok());

        // Save result to database
        match &result {
            Ok(ctx) => {
                if let Ok(Some(mut idea)) = db.get_idea(&idea_id) {
                    idea.novelty_score = Some(ctx.novelty_score);
                    idea.analysis = if !ctx.ai_analysis.is_empty() {
                        ctx.ai_analysis.clone()
                    } else {
                        // Use code-generated report from finalize step
                        format!(
                            "## 创新验证报告\n\n\
                             **新颖性评分：{:.0}/100**\n\n\
                             ### 评分细项\n\
                             - 最高相似度：{:.1}%\n\
                             - Top5 平均相似度：{:.1}%\n\
                             - 矛盾信号加分：+{:.0}\n\
                             - 覆盖缺口加分：+{:.0}\n\n\
                             ### 搜索结果\n\
                             - 网络结果：{} 条\n\
                             - 专利结果：{} 条\n\
                             - 多样性评分：{:.0}%\n\n\
                             ### Top 匹配\n{}\n\n\
                             {}{}",
                            ctx.novelty_score,
                            ctx.score_breakdown.max_similarity * 100.0,
                            ctx.score_breakdown.avg_top5_similarity * 100.0,
                            ctx.score_breakdown.contradiction_bonus,
                            ctx.score_breakdown.coverage_gap_bonus,
                            ctx.web_results.len(),
                            ctx.patent_results.len(),
                            ctx.diversity_score * 100.0,
                            ctx.top_matches
                                .iter()
                                .take(5)
                                .map(|m| format!(
                                    "- **{}** (相似度 {:.0}%) [{}]({})",
                                    m.source_title,
                                    m.combined_score * 100.0,
                                    m.source_type,
                                    m.source_url
                                ))
                                .collect::<Vec<_>>()
                                .join("\n"),
                            if !ctx.contradictions.is_empty() {
                                format!(
                                    "\n### 矛盾信号（创新机会）\n{}\n",
                                    ctx.contradictions
                                        .iter()
                                        .map(|c| format!("- {} (信号强度 {:.0}%)", c.opportunity, c.signal_strength * 100.0))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                )
                            } else {
                                String::new()
                            },
                            if !ctx.action_plan.is_empty() {
                                format!("\n### 行动建议\n{}\n", ctx.action_plan)
                            } else {
                                String::new()
                            },
                        )
                    };
                    idea.status = "done".into();
                    let _ = db.update_idea(&idea);
                }
            }
            Err(e) => {
                if let Ok(Some(mut idea)) = db.get_idea(&idea_id) {
                    idea.analysis = format!("流水线执行失败：{}", e);
                    idea.status = "error".into();
                    let _ = db.update_idea(&idea);
                }
            }
        }

        // Clean up channel
        let mut ch = channels.lock().unwrap();
        ch.remove(&idea_id);
    });

    Json(json!({"status": "ok", "message": "流水线已启动"}))
}

/// GET /api/idea/:id/progress — SSE 实时进度流
pub async fn api_idea_progress(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let channels = s.pipeline_channels.clone();
    let id = idea_id.clone();

    let stream = async_stream::stream! {
        // 等待 channel 创建（最多 5 秒，给 pipeline 启动时间）
        let mut rx = None;
        for _ in 0..10 {
            {
                let ch = channels.lock().unwrap();
                if let Some(tx) = ch.get(&id) {
                    rx = Some(tx.subscribe());
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if let Some(mut rx) = rx {
            loop {
                match rx.recv().await {
                    Ok(progress) => {
                        let data = serde_json::to_string(&progress).unwrap_or_default();
                        yield Ok(Event::default().data(data));
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        yield Ok(Event::default().event("done").data("complete"));
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                }
            }
        } else {
            // 检查是否已完成（pipeline 可能在等待期间就跑完了）
            yield Ok(Event::default().event("done").data("no active pipeline"));
        }
    };

    Sse::new(stream)
}

/// GET /api/idea/:id/report — 获取流水线完整报告
pub async fn api_idea_report(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_idea(&idea_id) {
        Ok(Some(idea)) => Json(json!({
            "status": "ok",
            "report": {
                "id": idea.id,
                "title": idea.title,
                "description": idea.description,
                "status": idea.status,
                "novelty_score": idea.novelty_score,
                "analysis": idea.analysis,
                "web_results": serde_json::from_str::<serde_json::Value>(&idea.web_results).unwrap_or_default(),
                "patent_results": serde_json::from_str::<serde_json::Value>(&idea.patent_results).unwrap_or_default(),
                "created_at": idea.created_at,
            }
        })),
        _ => Json(json!({"status": "error", "message": "创意不存在"})),
    }
}

/// Generate a summary of the idea discussion
pub async fn api_idea_summarize_discussion(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    let idea = match s.db.get_idea(&idea_id) {
        Ok(Some(i)) => i,
        _ => return Json(json!({"error": "创意不存在"})),
    };

    let history = s.db.get_idea_messages(&idea_id).unwrap_or_default();
    if history.is_empty() {
        return Json(json!({"error": "没有讨论记录可以总结"}));
    }

    // Build conversation text for summarization
    let mut conversation = format!(
        "创意标题：{}\n创意描述：{}\n\n讨论记录：\n",
        idea.title, idea.description
    );
    for (_id, role, content, _ts) in &history {
        let role_label = if role == "user" { "用户" } else { "AI" };
        let content_preview: String = content.chars().take(500).collect();
        conversation.push_str(&format!("\n【{}】{}\n", role_label, content_preview));
    }

    let prompt = format!(
        "{}\n\n请对以上讨论进行总结，包括：\n\
         1. **核心讨论点**：讨论了哪些关键问题\n\
         2. **达成的共识**：确定了哪些方向或方案\n\
         3. **待解决问题**：还有哪些未决问题\n\
         4. **行动建议**：下一步应该做什么\n\n\
         总结要简洁，重点突出。",
        conversation.chars().take(4000).collect::<String>()
    );

    let ai = s.config.read().unwrap().ai_client();
    match ai.chat(&prompt, None).await {
        Ok(summary) => {
            // Save summary to DB
            let _ = s.db.update_idea_summary(&idea_id, &summary);
            Json(json!({"status": "ok", "summary": summary}))
        }
        Err(e) => Json(json!({"error": format!("总结生成失败: {}", e)})),
    }
}
