//! Step: 权利要求树构建 / Claim Tree Builder
//!
//! AI 从分析结果中提取权利要求结构，拆解为独立/从属权利要求 + 必要技术特征。

use crate::ai::AiClient;
use crate::db::Database;
use crate::pipeline::context::PipelineContext;
use anyhow::Result;
use rusqlite::params;
use std::sync::Arc;

pub async fn execute(ctx: &mut PipelineContext, ai: &AiClient, db: &Arc<Database>) -> Result<()> {
    let prompt = format!(
        "你是专利代理人。根据以下创意和分析结果，草拟权利要求书结构。\n\
         输出 JSON，格式如下（不要 markdown 标记）：\n\
         {{\"claims\":[{{\"claim_number\":1,\"claim_type\":\"independent\",\"content\":\"一种...\",\
         \"features\":[{{\"description\":\"特征描述\",\"novelty_flag\":true}}]}}]}}\n\n\
         要求：至少1条独立权利要求，每条包含2-5个必要技术特征，标注哪些是新颖性特征。\n\n\
         创意：{}\n描述：{}\n技术领域：{}\n新颖性评分：{:.1}/100\n分析：{}",
        ctx.title,
        ctx.description,
        ctx.technical_domain,
        ctx.novelty_score,
        &ctx.ai_analysis.chars().take(800).collect::<String>(),
    );

    let response = match ai.chat(&prompt, None).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("ClaimTree AI 生成失败: {}", e);
            return Ok(());
        }
    };

    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = match serde_json::from_str(cleaned) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("ClaimTree JSON 解析失败: {}", e);
            return Ok(());
        }
    };

    let claims = match parsed["claims"].as_array() {
        Some(c) => c,
        None => return Ok(()),
    };

    let conn = db.conn();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    for claim in claims.iter().take(10) {
        let claim_id = uuid::Uuid::new_v4().to_string();
        let claim_number = claim["claim_number"].as_u64().unwrap_or(0) as u32;
        let claim_type = claim["claim_type"].as_str().unwrap_or("independent");
        let content = claim["content"].as_str().unwrap_or("");

        if let Err(e) = conn.execute(
            "INSERT INTO claim_nodes (id, idea_id, claim_number, claim_type, parent_claim_id, content, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![claim_id, ctx.idea_id, claim_number, claim_type, Option::<String>::None, content, now],
        ) {
            tracing::warn!("claim_node 写入失败: {}", e);
            continue;
        }

        if let Some(features) = claim["features"].as_array() {
            for feat in features.iter().take(10) {
                let feat_id = uuid::Uuid::new_v4().to_string();
                let desc = feat["description"].as_str().unwrap_or("");
                let novelty = feat["novelty_flag"].as_bool().unwrap_or(false);

                let _ = conn.execute(
                    "INSERT INTO technical_features (id, claim_id, description, novelty_flag, evidence_ids, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![feat_id, claim_id, desc, novelty as i32, "[]", now],
                );
            }
        }
    }

    tracing::info!(
        "ClaimTree 构建完成: {} 条权利要求 (idea: {})",
        claims.len().min(10),
        ctx.idea_id
    );
    Ok(())
}
