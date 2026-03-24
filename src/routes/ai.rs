use super::AppState;
use crate::patent::*;
use axum::{extract::State, Json};
use serde_json::json;

pub async fn api_ai_chat(
    State(s): State<AppState>,
    Json(req): Json<AiChatRequest>,
) -> Json<AiResponse> {
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
    match ai.chat(&req.message, ctx.as_deref()).await {
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
                content: "专利1未找到".into(),
            })
        }
    };
    let p2 = match s.db.get_patent(id2) {
        Ok(Some(p)) => p,
        _ => {
            return Json(AiResponse {
                content: "专利2未找到".into(),
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
    for (i, id) in ids.iter().enumerate() {
        if let Ok(Some(p)) = s.db.get_patent(id) {
            let claims_preview: String = p.claims.chars().take(800).collect();
            patents_info.push_str(&format!(
                "### 专利 {} - {}\n专利号：{}\n申请人：{}\n摘要：{}\n权利要求：{}\n\n",
                i + 1, p.title, p.patent_number, p.applicant, p.abstract_text, claims_preview
            ));
        }
    }

    if patents_info.is_empty() {
        return Json(json!({"error": "未找到指定的专利"}));
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
                i + 1, p.patent_number, p.title, p.applicant, p.abstract_text, claims_preview
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
    for id in &ids {
        if let Ok(Some(p)) = s.db.get_patent(id) {
            patents_data.push((p.id.clone(), p.title.clone(), p.abstract_text.clone()));
        }
    }

    let ai = s.config.read().unwrap().ai_client();
    let results = ai.batch_summarize(&patents_data).await;

    let summaries: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(id, result)| match result {
            Ok(summary) => json!({"id": id, "summary": summary}),
            Err(e) => json!({"id": id, "error": format!("{}", e)}),
        })
        .collect();

    Json(json!({"status": "ok", "summaries": summaries}))
}
