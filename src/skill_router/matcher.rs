use anyhow::{anyhow, Result};

use super::{
    registry::RegistryStore,
    types::{SkillDefinition, SkillSource},
};

pub struct Matcher;

impl Matcher {
    /// Select the best skill from candidates.
    /// Scoring: source priority (local > generated > remote) + registry success rate + usage count.
    pub fn select_best(
        capability: &str,
        primary_candidates: &[SkillDefinition],
        fallback_candidates: &[SkillDefinition],
    ) -> Result<SkillDefinition> {
        Self::select_best_with_registry(capability, primary_candidates, fallback_candidates, None)
    }

    pub fn select_best_with_registry(
        capability: &str,
        primary_candidates: &[SkillDefinition],
        fallback_candidates: &[SkillDefinition],
        registry: Option<&RegistryStore>,
    ) -> Result<SkillDefinition> {
        let mut scored: Vec<(f64, SkillDefinition)> = primary_candidates
            .iter()
            .chain(fallback_candidates.iter())
            .filter(|skill| skill.supports_capability(capability))
            .map(|skill| {
                let score = Self::score(skill, registry);
                (score, skill.clone())
            })
            .collect();

        // Higher score = better; stable sort so ties keep original order
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .next()
            .map(|(_, skill)| skill)
            .ok_or_else(|| anyhow!("no candidate available for capability {capability}"))
    }

    /// Score a skill. Range roughly 0.0–10.0.
    /// - Source priority contributes 0–4 points (local=4, generated=2, remote=1)
    /// - Success rate contributes 0–3 points
    /// - Usage (30d) contributes 0–3 points (log-scaled, caps at 50 uses)
    fn score(skill: &SkillDefinition, registry: Option<&RegistryStore>) -> f64 {
        let source_score = match skill.source {
            SkillSource::Local           => 4.0,
            SkillSource::Generated       => 2.0,
            SkillSource::RemoteCandidate => 1.0,
        };

        let (success_score, usage_score) = if let Some(reg) = registry {
            if let Some(stats) = reg.skill_stats(&skill.metadata.name) {
                let s = stats.success_rate * 3.0;
                let u = (stats.uses_30d as f64 + 1.0).ln() / (50.0_f64 + 1.0).ln() * 3.0;
                (s, u.min(3.0))
            } else {
                // New skill: assume neutral success rate, zero usage
                (1.5, 0.0)
            }
        } else {
            (1.5, 0.0)
        };

        source_score + success_score + usage_score
    }
}
