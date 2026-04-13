//! Step 9: DetectContradictions — 检测现有技术间的矛盾信号
//!
//! 类型：CODE
//!
//! 矛盾 = 两个结果都与用户创意相关，但彼此之间差异很大
//! 这意味着该领域技术路线尚未收敛，存在创新空间

use crate::pipeline::context::{Contradiction, Evidence, PipelineContext};
use anyhow::Result;
use std::collections::HashSet;

/// 计算两个 token 集合的 Jaccard 系数
fn token_jaccard(a: &[String], b: &[String]) -> f64 {
    let set_a: HashSet<&String> = a.iter().collect();
    let set_b: HashSet<&String> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// 提取两个结果之间的差异关键词
fn difference_keywords(a: &[String], b: &[String]) -> Vec<String> {
    let set_b: HashSet<&String> = b.iter().collect();
    a.iter()
        .filter(|t| !set_b.contains(t) && t.len() >= 2)
        .take(5)
        .cloned()
        .collect()
}

/// 执行 Step 9
pub async fn execute(ctx: &mut PipelineContext) -> Result<()> {
    let matches = &ctx.top_matches;
    if matches.len() < 2 {
        return Ok(());
    }

    let mut contradictions = Vec::new();

    // 两两比较 Top 匹配结果
    let check_count = matches.len().min(10); // 最多检查前 10 个
    for i in 0..check_count {
        for j in (i + 1)..check_count {
            let a = &matches[i];
            let b = &matches[j];

            // 两者都与用户创意有一定相关性（combined_score > 0.05）
            if a.combined_score < 0.05 || b.combined_score < 0.05 {
                continue;
            }

            // 计算两者之间的相似度
            let mutual_sim = token_jaccard(&a.tokens, &b.tokens);

            // 如果两者之间相似度很低（< 0.15），说明技术路线不同
            if mutual_sim < 0.15 {
                let a_unique = difference_keywords(&a.tokens, &b.tokens);
                let b_unique = difference_keywords(&b.tokens, &a.tokens);

                let signal_strength = 1.0 - mutual_sim; // 差异越大，信号越强

                let dimension = if !a_unique.is_empty() && !b_unique.is_empty() {
                    format!("{} vs {}", a_unique.join("/"), b_unique.join("/"))
                } else {
                    "技术路线差异".into()
                };

                let opportunity = format!(
                    "「{}」与「{}」解决类似问题但采用不同技术路线，\
                     两者的差异区间可能存在未被探索的创新空间",
                    truncate_title(&a.source_title),
                    truncate_title(&b.source_title),
                );

                contradictions.push(Contradiction {
                    source_a: a.source_title.clone(),
                    source_b: b.source_title.clone(),
                    dimension,
                    signal_strength,
                    opportunity,
                });
            }
        }
    }

    // 按信号强度排序，取 Top 5
    contradictions.sort_by(|a, b| {
        b.signal_strength
            .partial_cmp(&a.signal_strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    contradictions.truncate(5);

    // 为每个矛盾生成证据 / Generate evidence for each contradiction
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    for c in &contradictions {
        ctx.evidence_chain.push(Evidence {
            id: uuid::Uuid::new_v4().to_string(),
            idea_id: ctx.idea_id.clone(),
            claim: format!(
                "技术路线矛盾：{} vs {} ({})",
                c.source_a, c.source_b, c.dimension
            ),
            source_type: "contradiction".to_string(),
            source_id: format!("{}|{}", c.source_a, c.source_b),
            source_title: format!("{} ↔ {}", c.source_a, c.source_b),
            source_url: String::new(),
            claim_number: None,
            excerpt: c.opportunity.clone(),
            relation: "contradicts".to_string(),
            confidence: c.signal_strength,
            produced_by: "DetectContradictions".to_string(),
            created_at: now.clone(),
        });
    }

    ctx.contradictions = contradictions;

    // 更新研发状态机：为每个矛盾添加开放问题
    for c in &ctx.contradictions {
        ctx.research_state.open_questions.push(format!(
            "{}与{}在{}维度存在矛盾，需进一步验证",
            c.source_a, c.source_b, c.dimension
        ));
    }

    Ok(())
}

fn truncate_title(title: &str) -> String {
    if title.chars().count() > 20 {
        format!("{}...", title.chars().take(20).collect::<String>())
    } else {
        title.to_string()
    }
}
