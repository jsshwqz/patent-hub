use super::AppState;
use crate::patent::*;
use crate::pipeline::context::PipelineProgress;
use crate::pipeline::runner::PipelineRunner;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode},
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
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

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

    let idea = match s.db.get_idea(id) {
        Ok(Some(i)) => i,
        _ => return Json(json!({"status": "error", "message": "idea not found"})),
    };

    // Mark as analyzing
    {
        let mut idea_mut = idea.clone();
        idea_mut.status = "analyzing".into();
        if let Err(e) = s.db.update_idea(&idea_mut) {
            tracing::error!("Failed to update idea {} status: {}", idea.id, e);
        }
    }

    // Run pipeline in quick mode (synchronous await)
    let config = s.config.read().unwrap().clone();
    let ai_client = config.ai_client();
    let db = s.db.clone();
    let runner = PipelineRunner::new(
        ai_client,
        db.clone(),
        config.serpapi_key.clone(),
        config.bing_api_key.clone(),
        config.lens_api_key.clone(),
        true, // quick_mode
    );

    let result = runner.run(id, &idea.title, &idea.description, None).await;

    // Save result and build response
    match result {
        Ok(ctx) => {
            if let Ok(Some(mut idea)) = db.get_idea(id) {
                idea.novelty_score = Some(ctx.novelty_score);
                idea.web_results = serde_json::to_string(
                    &ctx.web_results
                        .iter()
                        .map(|r| {
                            json!({
                                "title": r.title,
                                "snippet": r.snippet,
                                "link": r.link,
                                "source": r.source,
                            })
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|_| "[]".into());
                idea.patent_results = serde_json::to_string(
                    &ctx.patent_results
                        .iter()
                        .map(|r| {
                            json!({
                                "patent_number": r.id,
                                "title": r.title,
                                "abstract": r.snippet,
                                "source": r.source,
                            })
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|_| "[]".into());
                idea.analysis = format!(
                    "## 快速验证报告\n\n\
                     **新颖性评分：{:.0}/100**\n\n\
                     ### 评分细项\n\
                     - 最高相似度：{:.1}%\n\
                     - Top5 平均相似度：{:.1}%\n\n\
                     ### 搜索结果\n\
                     - 网络结果：{} 条\n\
                     - 专利结果：{} 条\n",
                    ctx.novelty_score,
                    ctx.score_breakdown.max_similarity * 100.0,
                    ctx.score_breakdown.avg_top5_similarity * 100.0,
                    ctx.web_results.len(),
                    ctx.patent_results.len(),
                );
                idea.status = "done".into();
                let _ = db.update_idea(&idea);

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
            } else {
                Json(json!({"status": "error", "message": "idea not found after pipeline"}))
            }
        }
        Err(e) => {
            if let Ok(Some(mut idea)) = db.get_idea(id) {
                idea.analysis = format!("快速验证失败：{}", e);
                idea.status = "error".into();
                let _ = db.update_idea(&idea);
            }
            Json(json!({"status": "error", "message": format!("快速验证失败：{}", e)}))
        }
    }
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

// ── Idea delete ─────────────────────────────────────────────────────

/// Delete an idea and all its messages
pub async fn api_idea_delete(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    // 级联删除：证据链 → 特征卡片 → 消息 → 创意 / Cascade delete: evidence → feature cards → messages → idea
    let _ = s.db.delete_evidence_by_idea(&idea_id);
    let _ = s.db.delete_feature_cards_by_idea(&idea_id);
    let _ = s.db.delete_idea_messages(&idea_id);
    match s.db.delete_idea(&idea_id) {
        Ok(_) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
}

// ── Evidence chain API ───────────────────────────────────────────────

/// GET /api/idea/:id/evidence — 获取创意的证据链 / Get evidence chain for an idea
pub async fn api_idea_evidence(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_evidence_by_idea(&idea_id) {
        Ok(evidences) => Json(json!({"status": "ok", "evidence": evidences, "count": evidences.len()})),
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
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

    // === 智能上下文管理 ===
    // 策略：超过 8 轮对话时，自动将旧消息压缩为总结，保留最近 6 轮 + 总结
    // 这样既省 token 又不丢失关键信息
    let auto_summary_threshold = 8; // 超过这个数触发自动总结
    let keep_recent = 6; // 保留最近 N 条消息

    // 如果消息量超过阈值且没有现有总结，自动触发压缩
    let mut summary = s.db.get_idea_summary(&idea_id).unwrap_or_default();
    if history.len() > auto_summary_threshold && summary.is_empty() {
        // 取旧消息（除最近 keep_recent 条）进行压缩
        let old_count = history.len() - keep_recent;
        let old_messages: Vec<_> = history[..old_count].to_vec();
        if !old_messages.is_empty() {
            let mut conv_text = String::new();
            for (_id, role, content, _ts) in &old_messages {
                let label = if role == "user" { "用户" } else { "AI" };
                conv_text.push_str(&format!("{}：{}\n\n", label, content));
            }
            let compress_prompt = format!(
                "请将以下关于「{}」的研发讨论对话压缩为一段简洁的摘要，\
                 保留所有关键技术结论、决策和未解决的问题：\n\n{}",
                idea.title, conv_text
            );
            let ai_tmp = s.config.read().unwrap().ai_client();
            if let Ok(compressed) = ai_tmp.chat(&compress_prompt, None).await {
                let _ = s.db.update_idea_summary(&idea_id, &compressed);
                summary = compressed;
                tracing::info!("[CHAT] Auto-compressed {} old messages into summary", old_count);
            }
        }
    }

    // Build context-aware prompt with enhanced depth
    let mut system_context = format!(
        "你是一位资深研发创新顾问，拥有专利分析、技术路线规划和创新方法论（TRIZ、第一性原理）的深厚经验。\n\
         你正在与研发人员深入讨论一个创新想法。\n\n\
         ## 创意信息\n\
         **标题：** {}\n\
         **描述：** {}\n\
         **状态：** {}\n",
        idea.title, idea.description, idea.status
    );

    // Add analysis results if available
    if !idea.analysis.is_empty() {
        // 只提取分析中的关键结论段落，不灌水
        let analysis_lines: Vec<&str> = idea.analysis.lines()
            .filter(|l| {
                let t = l.trim();
                // 保留：标题行、评分行、结论行、风险行；跳过：空行、纯装饰、过长描述
                !t.is_empty() && (
                    t.starts_with('#') || t.starts_with('*') || t.starts_with('-') ||
                    t.contains("评分") || t.contains("结论") || t.contains("风险") ||
                    t.contains("建议") || t.contains("差异") || t.contains("创新") ||
                    t.contains("score") || t.contains("novel") || t.contains("risk")
                )
            })
            .take(30) // 最多 30 行关键内容
            .collect();
        if !analysis_lines.is_empty() {
            system_context.push_str(&format!(
                "\n## 验证分析结果（关键结论）\n{}\n",
                analysis_lines.join("\n")
            ));
        }
    }

    if let Some(score) = idea.novelty_score {
        system_context.push_str(&format!("\n**新颖性评分：** {:.1}/100\n", score));
    }

    // Add compressed discussion summary (long-term memory)
    if !summary.is_empty() {
        system_context.push_str(&format!(
            "\n## 之前的讨论记忆（已压缩）\n{}\n\
             （以上是之前多轮讨论的精华总结，请基于此继续深入）\n",
            summary
        ));
    }

    system_context.push_str(
        "\n## 你的思维工具箱（按场景灵活运用，不要每次都用全部）\n\
         - **第一性原理**：遇到复杂方案时，回到物理/化学/数学基本原理验证可行性\n\
         - **TRIZ 矛盾分析**：发现技术矛盾时（如强度 vs 重量），用 TRIZ 思路提出解决方向\n\
         - **逆向工程思维**：从期望结果倒推，找出实现路径上的关键瓶颈\n\
         - **类比迁移**：从其他行业/领域找到类似问题的已有解决方案\n\
         - **边界探测**：主动测试方案的极端情况和失效条件\n\n\
         ## 你的行为准则\n\
         1. **精准追问**：不问泛泛的问题，每个问题都指向具体的技术决策点\n\
         2. **假设-验证链**：先明确假设，再给出验证路径，最后给结论\n\
         3. **盲点发现**：主动指出用户可能忽略的技术风险或竞争威胁\n\
         4. **证据优先**：引用物理定律、已知材料参数、行业标准等硬证据\n\
         5. **风险分级**：区分「致命缺陷」「需要验证」「可接受风险」三个级别\n\
         6. **每轮推进**：回答末尾提出 1-2 个精准的引导性问题，推动研发进入下一层\n\
         7. **简洁有力**：不说废话，每句话都有信息量\n",
    );

    // Build message history with smart windowing
    let recent_history: Vec<_> = if history.len() > keep_recent {
        history[history.len() - keep_recent..].to_vec()
    } else {
        history.clone()
    };

    let mut chat_history: Vec<(String, String)> = Vec::new();
    for (_id, role, content, _ts) in &recent_history {
        chat_history.push((role.clone(), content.clone()));
    }
    // Add current user message
    chat_history.push(("user".into(), user_msg.to_string()));

    // Save user message to DB
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) =
        s.db.add_idea_message(&user_msg_id, &idea_id, "user", user_msg)
    {
        return Json(json!({"error": format!("保存消息失败: {}", e)}));
    }

    // Call AI with full message history (preserves multi-turn context)
    let ai = s.config.read().unwrap().ai_client();
    let ai_response = match ai.chat_with_history(&system_context, chat_history, 0.7).await {
        Ok(content) => content,
        Err(e) => {
            return Json(json!({"error": format!("AI 响应失败: {}", e)}));
        }
    };

    // Save AI response to DB
    let ai_msg_id = uuid::Uuid::new_v4().to_string();
    let _ =
        s.db.add_idea_message(&ai_msg_id, &idea_id, "assistant", &ai_response);

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

/// POST /api/idea/pipeline — 启动 13 步创新验证流水线
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

    // 创建进度广播通道 / Create broadcast channel for progress
    let (tx, _) = tokio::sync::broadcast::channel::<PipelineProgress>(64);
    {
        let mut channels = s.pipeline_channels.lock().unwrap();
        channels.insert(id.to_string(), super::PipelineChannelEntry {
            sender: tx.clone(),
            created_at: std::time::Instant::now(),
        });
    }

    // Build runner from config
    let config = s.config.read().unwrap().clone();
    let ai_client = config.ai_client();
    let db = s.db.clone();
    let serpapi_key = config.serpapi_key.clone();
    let bing_api_key = config.bing_api_key.clone();
    let lens_api_key = config.lens_api_key.clone();
    let runner = PipelineRunner::new(
        ai_client,
        db.clone(),
        serpapi_key,
        bing_api_key,
        lens_api_key,
        false,
    );

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
                                        .map(|c| format!(
                                            "- {} (信号强度 {:.0}%)",
                                            c.opportunity,
                                            c.signal_strength * 100.0
                                        ))
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

/// POST /api/idea/:id/resume — 断点续跑：从上次中断的步骤继续
/// Resume pipeline from last saved snapshot
pub async fn api_idea_resume(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // 检查是否有快照 / Check if snapshot exists
    match s.db.load_pipeline_snapshot(&id) {
        Ok(Some(_)) => {}
        Ok(None) => return Json(json!({"status": "error", "message": "没有可恢复的管道快照"})),
        Err(e) => return Json(json!({"status": "error", "message": e.to_string()})),
    }

    let config = s.config.read().unwrap().clone();
    let ai_client = config.ai_client();
    let db = s.db.clone();
    let runner = PipelineRunner::new(
        ai_client,
        db.clone(),
        config.serpapi_key.clone(),
        config.bing_api_key.clone(),
        config.lens_api_key.clone(),
        false,
    );

    let idea_id = id.clone();
    tokio::spawn(async move {
        tracing::info!("Pipeline 断点续跑: {}", idea_id);
        match runner.resume(&idea_id, None).await {
            Ok(Some(ctx)) => {
                if let Ok(Some(mut idea)) = db.get_idea(&idea_id) {
                    idea.novelty_score = Some(ctx.novelty_score);
                    if !ctx.ai_analysis.is_empty() {
                        idea.analysis = ctx.ai_analysis;
                    }
                    idea.status = "done".into();
                    let _ = db.update_idea(&idea);
                }
                tracing::info!("Pipeline 续跑完成: {}", idea_id);
            }
            Ok(None) => tracing::info!("Pipeline 无需续跑（已完成）: {}", idea_id),
            Err(e) => {
                tracing::error!("Pipeline 续跑失败: {} — {}", idea_id, e);
                if let Ok(Some(mut idea)) = db.get_idea(&idea_id) {
                    idea.status = "error".into();
                    idea.analysis = format!("续跑失败: {}", e);
                    let _ = db.update_idea(&idea);
                }
            }
        }
    });

    Json(json!({"status": "ok", "message": "管道正在恢复执行"}))
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
                if let Some(entry) = ch.get(&id) {
                    rx = Some(entry.sender.subscribe());
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

/// GET /api/idea/:id/report?type=executive|technical — 分层报告
///
/// - executive: 领导版（1 页摘要）
/// - technical: 研发版（详细技术报告）
/// - 无参数: 原始完整数据（向后兼容）
pub async fn api_idea_report(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let idea = match s.db.get_idea(&idea_id) {
        Ok(Some(idea)) => idea,
        _ => return Json(json!({"status": "error", "message": "创意不存在"})),
    };

    let report_type = params.get("type").map(|s| s.as_str()).unwrap_or("full");
    let score = idea.novelty_score.unwrap_or(0.0);
    let level = if score >= 70.0 { "高新颖性" } else if score >= 40.0 { "中等新颖性" } else { "低新颖性" };

    match report_type {
        "executive" => {
            // 领导版：1 页精简报告
            let risk = if score >= 70.0 { "低" } else if score >= 40.0 { "中" } else { "高" };
            let conclusion = if score >= 70.0 {
                "该创意具有较高新颖性，建议推进专利申请"
            } else if score >= 40.0 {
                "该创意有一定新颖性，建议进一步差异化后申请"
            } else {
                "该创意与现有技术重叠较多，建议调整技术路线"
            };
            let report_md = format!(
                "# 创新验证结论\n\
                 **项目**：{}\n\
                 **新颖性评分**：{:.1}/100（{}）\n\
                 **核心结论**：{}\n\
                 **建议**：\n\
                 - 1. 对核心技术差异点进行深度论证\n\
                 - 2. 补充实验数据支撑新颖性主张\n\
                 - 3. 关注竞品动态，及时更新技术方案\n\
                 **风险等级**：{}",
                idea.title, score, level, conclusion, risk
            );
            Json(json!({
                "status": "ok",
                "report_type": "executive",
                "report": report_md,
            }))
        }
        "technical" => {
            // 研发版：详细技术报告
            let web_results = serde_json::from_str::<serde_json::Value>(&idea.web_results).unwrap_or_default();
            let patent_results = serde_json::from_str::<serde_json::Value>(&idea.patent_results).unwrap_or_default();
            let report_md = format!(
                "# 技术验证报告\n\n\
                 ## 创意概述\n\
                 **标题**：{}\n\
                 **描述**：{}\n\n\
                 ## 新颖性评分\n\
                 **总分**：{:.1}/100（{}）\n\n\
                 ## 现有技术对比\n\
                 详见 web_results 和 patent_results 字段。\n\n\
                 ## 技术差异分析\n\
                 {}\n\n\
                 ## 行动方案\n\
                 基于以上分析，建议执行差异化策略。",
                idea.title, idea.description, score, level, idea.analysis
            );
            Json(json!({
                "status": "ok",
                "report_type": "technical",
                "report": report_md,
                "web_results": web_results,
                "patent_results": patent_results,
            }))
        }
        _ => {
            // 默认：完整原始数据（向后兼容）
            Json(json!({
                "status": "ok",
                "report_type": "full",
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
            }))
        }
    }
}

/// GET /api/idea/:id/report.html — 可打印的 HTML 验证报告（浏览器 Ctrl+P 另存 PDF）
/// Printable HTML validation report (browser Save as PDF)
pub async fn api_idea_report_html(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Response<Body> {
    let idea = match s.db.get_idea(&idea_id) {
        Ok(Some(i)) => i,
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("创意不存在"))
                .unwrap();
        }
    };

    let feature_cards = s
        .db
        .get_feature_cards_by_idea(&idea_id)
        .unwrap_or_default();

    let cards_html: String = if feature_cards.is_empty() {
        "<p>暂无特征卡片</p>".to_string()
    } else {
        feature_cards
            .iter()
            .map(|c| {
                format!(
                    "<div class='card'>\
                     <h4>{}</h4>\
                     <p>{}</p>\
                     <span class='score'>新颖性: {}</span>\
                     </div>",
                    html_escape(&c.title),
                    html_escape(&c.description),
                    c.novelty_score
                        .map(|s| format!("{:.0}", s))
                        .unwrap_or_else(|| "N/A".into()),
                )
            })
            .collect()
    };

    let analysis_html = if idea.analysis.is_empty() {
        "分析尚未完成".to_string()
    } else {
        html_escape(&idea.analysis)
    };
    let score_display = idea
        .novelty_score
        .map(|s| format!("{:.0}/100", s))
        .unwrap_or_else(|| "N/A".into());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<title>创新验证报告 — {title}</title>
<style>
  @media print {{ @page {{ margin: 1.5cm; }} }}
  body {{ font-family: -apple-system, "Microsoft YaHei", sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; color: #333; line-height: 1.6; }}
  h1 {{ border-bottom: 2px solid #2563eb; padding-bottom: 8px; }}
  .meta {{ color: #666; margin-bottom: 20px; }}
  .score-box {{ background: #eff6ff; border: 1px solid #bfdbfe; border-radius: 8px; padding: 16px; text-align: center; margin: 20px 0; }}
  .score-box .score {{ font-size: 2em; font-weight: bold; color: #2563eb; }}
  .card {{ border: 1px solid #e5e7eb; border-radius: 6px; padding: 12px; margin: 8px 0; }}
  .card h4 {{ margin: 0 0 4px 0; }}
  .card .score {{ font-size: 0.85em; color: #059669; }}
  .section {{ margin-top: 24px; }}
  .section h2 {{ color: #1e40af; }}
  .analysis {{ white-space: pre-wrap; background: #f9fafb; padding: 16px; border-radius: 8px; }}
  .print-btn {{ background: #2563eb; color: #fff; border: none; padding: 10px 24px; border-radius: 6px; cursor: pointer; font-size: 16px; }}
  @media print {{ .no-print {{ display: none; }} }}
</style>
</head>
<body>
<div class="no-print" style="text-align:right;margin-bottom:16px;">
  <button class="print-btn" onclick="window.print()">导出 PDF / 打印</button>
</div>
<h1>创新验证报告</h1>
<div class="meta">
  <strong>创意：</strong>{title}<br>
  <strong>描述：</strong>{description}<br>
  <strong>状态：</strong>{status}<br>
  <strong>创建时间：</strong>{created_at}
</div>

<div class="score-box">
  <div>新颖性评分</div>
  <div class="score">{score}</div>
</div>

<div class="section">
  <h2>AI 深度分析</h2>
  <div class="analysis">{analysis}</div>
</div>

<div class="section">
  <h2>特征卡片 ({card_count} 张)</h2>
  {cards}
</div>

<div class="section" style="color:#999;font-size:0.85em;margin-top:40px;border-top:1px solid #eee;padding-top:8px;">
  创研台 InnoForge · 创新推演报告 · {created_at}
</div>
</body>
</html>"#,
        title = html_escape(&idea.title),
        description = html_escape(&idea.description),
        status = html_escape(&idea.status),
        created_at = html_escape(&idea.created_at),
        score = score_display,
        analysis = analysis_html,
        card_count = feature_cards.len(),
        cards = cards_html,
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap()
}

/// 简单 HTML 转义 / Simple HTML escape
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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

/// POST /api/ideas/batch-compare — 批量创意对比矩阵 / Batch idea comparison matrix
///
/// 请求：{ "idea_ids": ["id1", "id2", ...] }（2-10 个）
/// 返回：每个创意的摘要 + 两两相似度矩阵
pub async fn api_ideas_batch_compare(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let ids: Vec<String> = match req["idea_ids"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        None => return Json(json!({"status": "error", "message": "缺少 idea_ids 数组"})),
    };
    if ids.len() < 2 {
        return Json(json!({"status": "error", "message": "至少需要 2 个创意进行对比"}));
    }
    if ids.len() > 10 {
        return Json(json!({"status": "error", "message": "最多支持 10 个创意同时对比"}));
    }

    // 加载所有创意
    let mut ideas = Vec::new();
    for id in &ids {
        match s.db.get_idea(id) {
            Ok(Some(idea)) => ideas.push(idea),
            _ => return Json(json!({"status": "error", "message": format!("创意 {} 不存在", id)})),
        }
    }

    // 构建摘要列表
    let summaries: Vec<serde_json::Value> = ideas
        .iter()
        .map(|i| {
            json!({
                "id": i.id,
                "title": i.title,
                "description": i.description.chars().take(200).collect::<String>(),
                "novelty_score": i.novelty_score,
                "status": i.status,
            })
        })
        .collect();

    // 计算两两文本相似度矩阵（Jaccard on character trigrams）
    let n = ideas.len();
    let mut matrix = vec![vec![0.0f64; n]; n];
    let trigrams: Vec<std::collections::HashSet<String>> = ideas
        .iter()
        .map(|i| {
            let text = format!("{} {}", i.title, i.description);
            char_trigrams(&text)
        })
        .collect();

    for i in 0..n {
        matrix[i][i] = 1.0;
        for j in (i + 1)..n {
            let sim = jaccard_trigram(&trigrams[i], &trigrams[j]);
            matrix[i][j] = sim;
            matrix[j][i] = sim;
        }
    }

    Json(json!({
        "status": "ok",
        "ideas": summaries,
        "similarity_matrix": matrix,
    }))
}

/// 字符三元组集合 / Character trigram set
fn char_trigrams(text: &str) -> std::collections::HashSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut set = std::collections::HashSet::new();
    if chars.len() >= 3 {
        for w in chars.windows(3) {
            set.insert(w.iter().collect::<String>());
        }
    }
    set
}

/// Jaccard 相似度 / Jaccard similarity between two sets
fn jaccard_trigram(
    a: &std::collections::HashSet<String>,
    b: &std::collections::HashSet<String>,
) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

// ── 版本管理 + 迭代 API / Version management + iterate API ────────────

/// POST /api/idea/:id/iterate — 持续迭代：基于上一轮结果重跑 Pipeline
pub async fn api_idea_iterate(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    // 获取最新版本快照
    let latest = match s.db.get_latest_version(&idea_id, "main") {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Json(serde_json::json!({"status": "error", "message": "无历史版本，请先运行 Pipeline"}));
        }
        Err(e) => {
            return Json(serde_json::json!({"status": "error", "message": e.to_string()}));
        }
    };

    // 反序列化上一轮 context
    let mut ctx: crate::pipeline::context::PipelineContext = match serde_json::from_str(&latest.context_json) {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({"status": "error", "message": format!("快照反序列化失败: {}", e)}));
        }
    };

    // 递增迭代计数
    ctx.iteration_count += 1;
    ctx.parent_version_id = latest.id.clone();

    // 从 SearchWeb 重新开始（保留 keywords 和 expanded_queries，补充新搜索）
    ctx.current_step = crate::pipeline::state::PipelineStep::SearchWeb;

    // 使用 open_questions 作为额外搜索种子
    if !ctx.research_state.open_questions.is_empty() {
        let extra_queries: Vec<String> = ctx.research_state.open_questions
            .iter()
            .take(3)
            .map(|q| q.chars().take(50).collect())
            .collect();
        ctx.expanded_queries.extend(extra_queries);
    }

    // 启动后台 Pipeline（从 SearchWeb 开始）
    let config = s.config.read().unwrap().clone();
    let runner = PipelineRunner::new(
        config.ai_client(),
        s.db.clone(),
        config.serpapi_key.clone(),
        config.bing_api_key.clone(),
        config.lens_api_key.clone(),
        false,
    );

    let db = s.db.clone();
    let idea_id_clone = idea_id.clone();
    tokio::spawn(async move {
        match runner.resume(&idea_id_clone, None).await {
            Ok(_) => tracing::info!("Iterate pipeline completed for {}", idea_id_clone),
            Err(e) => {
                tracing::error!("Iterate pipeline failed: {}", e);
                if let Ok(Some(mut idea)) = db.get_idea(&idea_id_clone) {
                    idea.analysis = format!("迭代失败: {}", e);
                    let _ = db.update_idea(&idea);
                }
            }
        }
    });

    Json(serde_json::json!({
        "status": "ok",
        "message": format!("第 {} 轮迭代已启动", ctx.iteration_count),
        "iteration": ctx.iteration_count,
    }))
}

/// GET /api/idea/:id/versions — 列出版本历史
pub async fn api_idea_versions(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_idea_versions(&idea_id) {
        Ok(versions) => {
            let summaries: Vec<serde_json::Value> = versions
                .iter()
                .map(|v| serde_json::json!({
                    "id": v.id,
                    "version_number": v.version_number,
                    "current_step": v.current_step,
                    "branch_id": v.branch_id,
                    "created_at": v.created_at,
                }))
                .collect();
            Json(serde_json::json!({"status": "ok", "versions": summaries}))
        }
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string()})),
    }
}

/// GET /api/idea/:id/branches — 列出分支
pub async fn api_idea_branches(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_idea_branches(&idea_id) {
        Ok(branches) => Json(serde_json::json!({"status": "ok", "branches": branches})),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string()})),
    }
}

/// GET /api/idea/:id/findings — 列出发现记录
pub async fn api_idea_findings(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_findings_by_idea(&idea_id) {
        Ok(findings) => Json(serde_json::json!({"status": "ok", "findings": findings})),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e.to_string()})),
    }
}
