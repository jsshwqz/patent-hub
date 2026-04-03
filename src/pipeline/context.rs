use super::state::PipelineStep;
use serde::{Deserialize, Serialize};

/// 相似度条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityEntry {
    pub source_id: String,
    pub source_title: String,
    pub source_type: String, // "web" or "patent"
    pub tfidf_score: f64,
    pub jaccard_score: f64,
    pub combined_score: f64,
}

/// 排序后的匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMatch {
    pub rank: usize,
    pub source_id: String,
    pub source_title: String,
    pub source_type: String,
    pub source_url: String,
    pub snippet: String,
    pub combined_score: f64,
    pub tokens: Vec<String>,
}

/// 现有技术聚类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorArtCluster {
    pub cluster_id: usize,
    pub topic: String,
    pub patent_indices: Vec<usize>,
    pub representative_title: String,
    pub avg_similarity: f64,
}

/// 矛盾信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub source_a: String,
    pub source_b: String,
    pub dimension: String,
    pub signal_strength: f64,
    pub opportunity: String,
}

/// 新颖性评分细项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub max_similarity: f64,
    pub avg_top5_similarity: f64,
    pub contradiction_bonus: f64,
    pub coverage_gap_bonus: f64,
    pub final_score: f64,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step: PipelineStep,
    pub duration_ms: u64,
    pub status: String,
    pub error: Option<String>,
}

/// 搜索结果（通用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub link: String,
    pub source: String,
}

/// 单个维度的推演结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionInsight {
    pub dimension: String,
    pub label: String,
    pub reasoning: String,
    pub key_insight: String,
}

/// 多维深度推演结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeepReasoningResult {
    pub dimensions: Vec<DimensionInsight>,
    pub synthesis: String,
    pub novel_directions: Vec<String>,
    pub blind_spots: Vec<String>,
}

/// 流水线进度消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub step: PipelineStep,
    pub step_index: usize,
    pub total_steps: usize,
    pub status: StepStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_progress: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Running,
    Done,
    Skipped,
    Error,
}

/// 流水线上下文 — 在步骤间传递的数据载体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub idea_id: String,
    pub title: String,
    pub description: String,

    // Step 1: ParseInput
    pub keywords: Vec<String>,
    pub technical_domain: String,

    // Step 2: ExpandQuery
    pub expanded_queries: Vec<String>,

    // Step 3-4: Search
    pub web_results: Vec<SearchResult>,
    pub patent_results: Vec<SearchResult>,

    // Step 5: DiversityGate
    pub diversity_score: f64,
    pub coverage_dimensions: Vec<String>,
    pub retry_count: u32,

    // Step 6: ComputeSimilarity
    pub similarity_scores: Vec<SimilarityEntry>,

    // Step 7: RankAndFilter
    pub top_matches: Vec<RankedMatch>,

    // Step 8: PriorArtCluster
    pub prior_art_clusters: Vec<PriorArtCluster>,

    // Step 9: DetectContradictions
    pub contradictions: Vec<Contradiction>,

    // Step 10: ScoreNovelty
    pub novelty_score: f64,
    pub score_breakdown: ScoreBreakdown,

    // Step 11-12: AI Analysis
    pub ai_analysis: String,
    pub action_plan: String,

    // 多维深度推演结果
    #[serde(default)]
    pub deep_reasoning: DeepReasoningResult,

    // 元数据
    pub current_step: PipelineStep,
    pub step_results: Vec<StepResult>,
}

impl PipelineContext {
    pub fn new(idea_id: &str, title: &str, description: &str) -> Self {
        Self {
            idea_id: idea_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            keywords: Vec::new(),
            technical_domain: String::new(),
            expanded_queries: Vec::new(),
            web_results: Vec::new(),
            patent_results: Vec::new(),
            diversity_score: 0.0,
            coverage_dimensions: Vec::new(),
            retry_count: 0,
            similarity_scores: Vec::new(),
            top_matches: Vec::new(),
            prior_art_clusters: Vec::new(),
            contradictions: Vec::new(),
            novelty_score: 0.0,
            score_breakdown: ScoreBreakdown::default(),
            ai_analysis: String::new(),
            action_plan: String::new(),
            deep_reasoning: DeepReasoningResult::default(),
            current_step: PipelineStep::ParseInput,
            step_results: Vec::new(),
        }
    }
}
