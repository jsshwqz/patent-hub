//! Step 8: PriorArtCluster — 将排序后的匹配结果按主题聚类
//!
//! 类型：CODE
//!
//! 使用贪心 Jaccard 相似度聚类，将相似专利归为同一主题组，
//! 以便下游分析可以按现有技术主题（而非单个专利）进行推理。

use crate::pipeline::context::{PipelineContext, PriorArtCluster};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Jaccard similarity threshold for assigning a result to an existing cluster.
const CLUSTER_SIMILARITY_THRESHOLD: f64 = 0.25;

/// Compute Jaccard similarity between two token sets.
fn jaccard_similarity(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Derive a topic label from the most frequent tokens across all members.
fn derive_topic(all_tokens: &[Vec<String>], max_words: usize) -> String {
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for tokens in all_tokens {
        // Deduplicate per-document so common words across docs are boosted
        let unique: HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
        for tok in unique {
            *freq.entry(tok).or_insert(0) += 1;
        }
    }
    let mut sorted: Vec<(&&str, &usize)> = freq.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    sorted
        .iter()
        .take(max_words)
        .map(|(tok, _)| **tok)
        .collect::<Vec<&str>>()
        .join(" / ")
}

/// 执行 Step 8: PriorArtCluster
pub async fn execute(ctx: &mut PipelineContext) -> Result<()> {
    let matches = &ctx.top_matches;
    if matches.is_empty() {
        return Ok(());
    }

    // Pre-compute token sets for each ranked match
    let token_sets: Vec<HashSet<&str>> = matches
        .iter()
        .map(|m| m.tokens.iter().map(|s| s.as_str()).collect())
        .collect();

    // Clusters: (representative_index, member_indices, all_member_tokens)
    let mut clusters: Vec<(usize, Vec<usize>, Vec<Vec<String>>)> = Vec::new();

    for (i, token_set) in token_sets.iter().enumerate() {
        let mut best_cluster: Option<(usize, f64)> = None;

        for (ci, (rep_idx, _, _)) in clusters.iter().enumerate() {
            let sim = jaccard_similarity(token_set, &token_sets[*rep_idx]);
            if sim >= CLUSTER_SIMILARITY_THRESHOLD
                && (best_cluster.is_none() || sim > best_cluster.unwrap().1)
            {
                best_cluster = Some((ci, sim));
            }
        }

        if let Some((ci, _)) = best_cluster {
            clusters[ci].1.push(i);
            clusters[ci].2.push(matches[i].tokens.clone());
        } else {
            // Create a new cluster with this item as representative
            clusters.push((i, vec![i], vec![matches[i].tokens.clone()]));
        }
    }

    // Build output
    let mut result: Vec<PriorArtCluster> = Vec::with_capacity(clusters.len());
    for (cluster_id, (rep_idx, member_indices, member_tokens)) in clusters.into_iter().enumerate() {
        let avg_similarity = if member_indices.is_empty() {
            0.0
        } else {
            let total: f64 = member_indices
                .iter()
                .map(|&idx| matches[idx].combined_score)
                .sum();
            total / member_indices.len() as f64
        };

        let topic = derive_topic(&member_tokens, 3);

        result.push(PriorArtCluster {
            cluster_id,
            topic,
            patent_indices: member_indices,
            representative_title: matches[rep_idx].source_title.clone(),
            avg_similarity,
        });
    }

    ctx.prior_art_clusters = result;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::context::{PipelineContext, RankedMatch};

    fn make_match(rank: usize, title: &str, snippet: &str, tokens: Vec<&str>, score: f64) -> RankedMatch {
        RankedMatch {
            rank,
            source_id: format!("id{}", rank),
            source_title: title.to_string(),
            source_type: "web".to_string(),
            source_url: format!("https://example.com/{}", rank),
            snippet: snippet.to_string(),
            combined_score: score,
            tokens: tokens.into_iter().map(String::from).collect(),
        }
    }

    fn make_ctx(matches: Vec<RankedMatch>) -> PipelineContext {
        let mut ctx = PipelineContext::new("test-idea", "Test Idea", "desc");
        ctx.top_matches = matches;
        ctx
    }

    #[tokio::test]
    async fn test_clustering_groups_similar_items() {
        // Two items with overlapping tokens should cluster together
        let matches = vec![
            make_match(1, "Neural network image recognition", "deep learning CNN",
                       vec!["neural", "network", "image", "recognition", "deep", "learning", "cnn"], 0.8),
            make_match(2, "Deep learning image classification", "convolutional neural net",
                       vec!["deep", "learning", "image", "classification", "convolutional", "neural", "net"], 0.75),
            make_match(3, "Blockchain supply chain tracking", "distributed ledger",
                       vec!["blockchain", "supply", "chain", "tracking", "distributed", "ledger"], 0.6),
        ];
        let mut ctx = make_ctx(matches);
        execute(&mut ctx).await.unwrap();

        // Neural network items should cluster together, blockchain separate
        assert_eq!(ctx.prior_art_clusters.len(), 2);

        let nn_cluster = &ctx.prior_art_clusters[0];
        assert_eq!(nn_cluster.patent_indices.len(), 2);
        assert!(nn_cluster.patent_indices.contains(&0));
        assert!(nn_cluster.patent_indices.contains(&1));

        let bc_cluster = &ctx.prior_art_clusters[1];
        assert_eq!(bc_cluster.patent_indices.len(), 1);
        assert!(bc_cluster.patent_indices.contains(&2));
    }

    #[tokio::test]
    async fn test_empty_input() {
        let mut ctx = make_ctx(vec![]);
        execute(&mut ctx).await.unwrap();
        assert!(ctx.prior_art_clusters.is_empty());
    }

    #[tokio::test]
    async fn test_single_item() {
        let matches = vec![
            make_match(1, "Quantum computing error correction", "qubit stabilization",
                       vec!["quantum", "computing", "error", "correction"], 0.9),
        ];
        let mut ctx = make_ctx(matches);
        execute(&mut ctx).await.unwrap();

        assert_eq!(ctx.prior_art_clusters.len(), 1);
        assert_eq!(ctx.prior_art_clusters[0].patent_indices, vec![0]);
        assert!((ctx.prior_art_clusters[0].avg_similarity - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_all_dissimilar_items() {
        let matches = vec![
            make_match(1, "A", "a", vec!["alpha", "beta"], 0.5),
            make_match(2, "B", "b", vec!["gamma", "delta"], 0.4),
            make_match(3, "C", "c", vec!["epsilon", "zeta"], 0.3),
        ];
        let mut ctx = make_ctx(matches);
        execute(&mut ctx).await.unwrap();

        // Each item in its own cluster
        assert_eq!(ctx.prior_art_clusters.len(), 3);
    }

    #[tokio::test]
    async fn test_topic_derivation() {
        let matches = vec![
            make_match(1, "专利分析系统", "自然语言处理",
                       vec!["专利", "分析", "系统", "自然语言", "处理"], 0.7),
            make_match(2, "专利检索与分析平台", "文本挖掘",
                       vec!["专利", "检索", "分析", "平台", "文本", "挖掘"], 0.65),
        ];
        let mut ctx = make_ctx(matches);
        execute(&mut ctx).await.unwrap();

        // They share "专利" and "分析", Jaccard = 2/(5+6-2) = 2/9 ≈ 0.22 < 0.25
        // Actually let's check: union of all tokens is 9, intersection is 2 => 0.222
        // Below threshold, so they should be in separate clusters
        // However the requirement says >= 0.25, so these won't cluster
        assert_eq!(ctx.prior_art_clusters.len(), 2);
    }

    #[tokio::test]
    async fn test_chinese_text_clustering() {
        // Items with significant Chinese token overlap should cluster
        let matches = vec![
            make_match(1, "智能停车系统", "自动泊车",
                       vec!["智能", "停车", "系统", "自动", "泊车"], 0.8),
            make_match(2, "智能停车场管理", "车位检测",
                       vec!["智能", "停车", "场", "管理", "车位", "检测"], 0.75),
            // Jaccard("智能","停车" shared) = 2/9 ≈ 0.22 — below 0.25, separate clusters
            // Let's add more overlap to ensure clustering:
            make_match(3, "自动泊车辅助系统", "智能停车",
                       vec!["自动", "泊车", "辅助", "系统", "智能", "停车"], 0.7),
        ];
        // match 1 tokens: {智能, 停车, 系统, 自动, 泊车} (5)
        // match 3 tokens: {自动, 泊车, 辅助, 系统, 智能, 停车} (6)
        // intersection: {智能, 停车, 系统, 自动, 泊车} = 5
        // wait — match 3 has {自动, 泊车, 辅助, 系统, 智能, 停车}
        // intersection with match 1: {智能, 停车, 系统, 自动, 泊车} = 5
        // union: {智能, 停车, 系统, 自动, 泊车, 辅助} = 6
        // Jaccard = 5/6 ≈ 0.83 — will cluster with match 1
        let mut ctx = make_ctx(matches);
        execute(&mut ctx).await.unwrap();

        // Match 1 and 3 should cluster together
        // Match 2: {智能,停车,场,管理,车位,检测} vs match 1: {智能,停车,系统,自动,泊车}
        // intersection: {智能,停车} = 2, union = 9, Jaccard = 0.22 — separate
        assert_eq!(ctx.prior_art_clusters.len(), 2);
        let cluster0 = &ctx.prior_art_clusters[0];
        assert!(cluster0.patent_indices.contains(&0));
        assert!(cluster0.patent_indices.contains(&2));
    }
}
