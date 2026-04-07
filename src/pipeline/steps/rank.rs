//! Step 7: RankAndFilter — 按相似度排序，去重，取 Top-N
//!
//! 类型：CODE

use crate::pipeline::context::{Evidence, PipelineContext, RankedMatch};
use crate::pipeline::steps::parse::tokenize;
use anyhow::Result;
use std::collections::HashSet;

const MAX_TOP_MATCHES: usize = 15;

/// 执行 Step 7
pub async fn execute(ctx: &mut PipelineContext) -> Result<()> {
    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut ranked = Vec::new();

    // 合并 web + patent 结果的 URL 映射
    let url_map: std::collections::HashMap<String, (String, String)> = ctx
        .web_results
        .iter()
        .map(|r| (r.id.clone(), (r.link.clone(), r.snippet.clone())))
        .chain(
            ctx.patent_results
                .iter()
                .map(|r| (r.id.clone(), (r.link.clone(), r.snippet.clone()))),
        )
        .collect();

    for entry in &ctx.similarity_scores {
        // 去重：标题相似的只保留第一个
        let title_key = entry
            .source_title
            .chars()
            .take(30)
            .collect::<String>()
            .to_lowercase();
        if seen_titles.contains(&title_key) {
            continue;
        }
        seen_titles.insert(title_key);

        let (url, snippet) = url_map.get(&entry.source_id).cloned().unwrap_or_default();

        let tokens = tokenize(&format!("{} {}", entry.source_title, snippet));

        ranked.push(RankedMatch {
            rank: ranked.len() + 1,
            source_id: entry.source_id.clone(),
            source_title: entry.source_title.clone(),
            source_type: entry.source_type.clone(),
            source_url: url,
            snippet,
            combined_score: entry.combined_score,
            tokens,
        });

        if ranked.len() >= MAX_TOP_MATCHES {
            break;
        }
    }

    // 为高相似度匹配生成证据 / Generate evidence for high-similarity matches
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    for m in &ranked {
        if m.combined_score > 0.1 {
            let relation = if m.combined_score > 0.7 {
                "supports" // 高度相似，支撑"已有类似技术"结论
            } else if m.combined_score > 0.3 {
                "partial"
            } else {
                "supports"
            };
            ctx.evidence_chain.push(Evidence {
                id: uuid::Uuid::new_v4().to_string(),
                idea_id: ctx.idea_id.clone(),
                claim: format!("与现有技术「{}」相似度 {:.0}%", m.source_title, m.combined_score * 100.0),
                source_type: m.source_type.clone(),
                source_id: m.source_id.clone(),
                source_title: m.source_title.clone(),
                source_url: m.source_url.clone(),
                claim_number: None,
                excerpt: m.snippet.chars().take(500).collect(),
                relation: relation.to_string(),
                confidence: m.combined_score,
                produced_by: "RankAndFilter".to_string(),
                created_at: now.clone(),
            });
        }
    }

    ctx.top_matches = ranked;
    Ok(())
}
