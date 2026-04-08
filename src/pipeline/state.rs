use serde::{Deserialize, Serialize};

/// 流水线步骤枚举 — 13 步固定序列
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStep {
    ParseInput,
    ExpandQuery,
    SearchWeb,
    SearchPatents,
    DiversityGate,
    ComputeSimilarity,
    RankAndFilter,
    PriorArtCluster,
    DetectContradictions,
    ScoreNovelty,
    AiDeepAnalysis,
    AiActionPlan,
    ExperimentValidation,
    Finalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Code,
    Llm,
}

impl PipelineStep {
    pub const TOTAL_STEPS: usize = 14;

    pub fn step_type(&self) -> StepType {
        match self {
            Self::ExpandQuery | Self::AiDeepAnalysis | Self::AiActionPlan => StepType::Llm,
            _ => StepType::Code,
        }
    }

    pub fn next(&self) -> Option<PipelineStep> {
        match self {
            Self::ParseInput => Some(Self::ExpandQuery),
            Self::ExpandQuery => Some(Self::SearchWeb),
            Self::SearchWeb => Some(Self::SearchPatents),
            Self::SearchPatents => Some(Self::DiversityGate),
            Self::DiversityGate => Some(Self::ComputeSimilarity),
            Self::ComputeSimilarity => Some(Self::RankAndFilter),
            Self::RankAndFilter => Some(Self::PriorArtCluster),
            Self::PriorArtCluster => Some(Self::DetectContradictions),
            Self::DetectContradictions => Some(Self::ScoreNovelty),
            Self::ScoreNovelty => Some(Self::AiDeepAnalysis),
            Self::AiDeepAnalysis => Some(Self::AiActionPlan),
            Self::AiActionPlan => Some(Self::ExperimentValidation),
            Self::ExperimentValidation => Some(Self::Finalize),
            Self::Finalize => None,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::ParseInput => 0,
            Self::ExpandQuery => 1,
            Self::SearchWeb => 2,
            Self::SearchPatents => 3,
            Self::DiversityGate => 4,
            Self::ComputeSimilarity => 5,
            Self::RankAndFilter => 6,
            Self::PriorArtCluster => 7,
            Self::DetectContradictions => 8,
            Self::ScoreNovelty => 9,
            Self::AiDeepAnalysis => 10,
            Self::AiActionPlan => 11,
            Self::ExperimentValidation => 12,
            Self::Finalize => 13,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::ParseInput => "解析输入，提取关键词",
            Self::ExpandQuery => "AI 扩展搜索词",
            Self::SearchWeb => "网络搜索",
            Self::SearchPatents => "专利搜索",
            Self::DiversityGate => "多样性检查",
            Self::ComputeSimilarity => "相似度计算",
            Self::RankAndFilter => "排序过滤",
            Self::PriorArtCluster => "现有技术聚类 / Prior art clustering",
            Self::DetectContradictions => "矛盾信号检测",
            Self::ScoreNovelty => "新颖性评分",
            Self::AiDeepAnalysis => "AI 深度分析",
            Self::AiActionPlan => "AI 行动方案",
            Self::ExperimentValidation => "实验验证",
            Self::Finalize => "生成报告",
        }
    }

    /// 关键步骤失败时终止流水线，非关键步骤可跳过
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::ParseInput | Self::SearchWeb | Self::ComputeSimilarity | Self::ScoreNovelty
        )
    }

    /// 快速模式下跳过的步骤
    pub fn skipped_in_quick_mode(&self) -> bool {
        matches!(
            self,
            Self::ExpandQuery
                | Self::DiversityGate
                | Self::RankAndFilter
                | Self::PriorArtCluster
                | Self::DetectContradictions
                | Self::AiDeepAnalysis
                | Self::AiActionPlan
                | Self::ExperimentValidation
        )
    }
}
