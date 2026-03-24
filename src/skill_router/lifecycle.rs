use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::types::SkillStats;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleRecommendation {
    Observe,
    Keep,
    Polish,
    PublishCandidate,
    Deprecate,
    PurgeCandidate,
}

impl LifecycleRecommendation {
    pub fn from_stats(stats: &SkillStats, now: SystemTime) -> Self {
        if let Some(last_used) = stats.last_used {
            if now.duration_since(last_used).unwrap_or_default()
                >= Duration::from_secs(180 * 24 * 60 * 60)
            {
                return Self::PurgeCandidate;
            }
            if now.duration_since(last_used).unwrap_or_default()
                >= Duration::from_secs(90 * 24 * 60 * 60)
            {
                return Self::Deprecate;
            }
        }

        if stats.uses_30d >= 15 && stats.success_rate >= 0.8 {
            Self::PublishCandidate
        } else if stats.uses_30d >= 8 {
            Self::Polish
        } else if stats.uses_30d >= 3 {
            Self::Keep
        } else {
            Self::Observe
        }
    }
}
