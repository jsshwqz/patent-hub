use std::{
    collections::BTreeMap,
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::types::{RouterPaths, SkillStats};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryFile {
    #[serde(default)]
    skills: BTreeMap<String, StoredSkillStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredSkillStats {
    #[serde(default)]
    executions: Vec<ExecutionStamp>,
    last_used_epoch_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionStamp {
    epoch_ms: u128,
    success: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RegistryStore {
    state: RegistryFile,
}

impl RegistryStore {
    pub fn load(paths: &RouterPaths) -> Result<Self> {
        paths.ensure_base_dirs()?;
        if !paths.registry_path.exists() {
            return Ok(Self::default());
        }

        let state = serde_json::from_slice(&fs::read(&paths.registry_path)?)?;
        Ok(Self { state })
    }

    pub fn save(&self, paths: &RouterPaths) -> Result<()> {
        paths.ensure_base_dirs()?;
        fs::write(
            &paths.registry_path,
            serde_json::to_vec_pretty(&self.state)?,
        )?;
        Ok(())
    }

    pub fn record_execution(&mut self, skill_name: &str, success: bool, now: SystemTime) {
        let entry = self.state.skills.entry(skill_name.to_string()).or_default();
        let now_ms = system_time_to_epoch_ms(now);
        entry.executions.push(ExecutionStamp {
            epoch_ms: now_ms,
            success,
        });
        entry.last_used_epoch_ms = Some(now_ms);
        prune_old_executions(&mut entry.executions, now);
    }

    pub fn record_synthetic_stats(
        &mut self,
        skill_name: &str,
        total_uses: usize,
        last_used: Option<SystemTime>,
    ) {
        let entry = self.state.skills.entry(skill_name.to_string()).or_default();
        entry.executions.clear();

        let base = last_used.unwrap_or_else(SystemTime::now);
        let base_ms = system_time_to_epoch_ms(base);
        for offset in 0..total_uses {
            entry.executions.push(ExecutionStamp {
                epoch_ms: base_ms.saturating_add(offset as u128),
                success: true,
            });
        }
        entry.last_used_epoch_ms = last_used.map(system_time_to_epoch_ms);
    }

    /// Remove skills with no executions in the last `days` days.
    /// Returns the list of purged skill names.
    pub fn gc(&mut self, days: u64) -> Vec<String> {
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(days * 24 * 60 * 60))
            .unwrap_or(UNIX_EPOCH);
        let mut purged = Vec::new();
        self.state.skills.retain(|name, stored| {
            let last = stored.last_used_epoch_ms.map(epoch_ms_to_system_time).unwrap_or(UNIX_EPOCH);
            if last < cutoff {
                purged.push(name.clone());
                false
            } else {
                true
            }
        });
        purged
    }

    pub fn skill_names(&self) -> impl Iterator<Item = &str> {
        self.state.skills.keys().map(String::as_str)
    }

    pub fn skill_stats(&self, skill_name: &str) -> Option<SkillStats> {
        let stored = self.state.skills.get(skill_name)?;
        let now = SystemTime::now();
        let total_uses = stored.executions.len();
        let uses_30d = stored
            .executions
            .iter()
            .filter(|item| {
                now.duration_since(epoch_ms_to_system_time(item.epoch_ms))
                    .unwrap_or_default()
                    <= Duration::from_secs(30 * 24 * 60 * 60)
            })
            .count();
        let successes = stored.executions.iter().filter(|item| item.success).count();
        let success_rate = if total_uses == 0 {
            1.0
        } else {
            successes as f64 / total_uses as f64
        };

        Some(SkillStats {
            total_uses,
            uses_30d,
            success_rate,
            last_used: stored.last_used_epoch_ms.map(epoch_ms_to_system_time),
        })
    }
}

fn prune_old_executions(executions: &mut Vec<ExecutionStamp>, now: SystemTime) {
    executions.retain(|item| {
        now.duration_since(epoch_ms_to_system_time(item.epoch_ms))
            .unwrap_or_default()
            <= Duration::from_secs(365 * 24 * 60 * 60)
    });
}

fn system_time_to_epoch_ms(value: SystemTime) -> u128 {
    value
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn epoch_ms_to_system_time(value: u128) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(value.min(u64::MAX as u128) as u64)
}
