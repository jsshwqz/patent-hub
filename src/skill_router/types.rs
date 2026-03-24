use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::lifecycle::LifecycleRecommendation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    Local,
    Generated,
    RemoteCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PermissionSet {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub filesystem_read: bool,
    #[serde(default)]
    pub filesystem_write: bool,
    #[serde(default)]
    pub process_exec: bool,
}

impl PermissionSet {
    pub fn default_deny() -> Self {
        Self::default()
    }

    pub fn with_network(mut self, enabled: bool) -> Self {
        self.network = enabled;
        self
    }

    pub fn with_filesystem_read(mut self, enabled: bool) -> Self {
        self.filesystem_read = enabled;
        self
    }

    pub fn with_filesystem_write(mut self, enabled: bool) -> Self {
        self.filesystem_write = enabled;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub entrypoint: String,
    #[serde(default)]
    pub permissions: PermissionSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub metadata: SkillMetadata,
    #[serde(skip)]
    pub root_dir: PathBuf,
    pub source: SkillSource,
}

impl SkillDefinition {
    pub fn supports_capability(&self, capability: &str) -> bool {
        self.metadata
            .capabilities
            .iter()
            .any(|item| item == capability)
    }

    pub fn resolved_entrypoint(&self) -> PathBuf {
        self.root_dir.join(&self.metadata.entrypoint)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub task: String,
    pub capability: String,
    #[serde(default)]
    pub context: Value,
    #[serde(default)]
    pub artifacts: Value,
}

impl ExecutionContext {
    pub fn new(task: &str, capability: &str) -> Self {
        Self {
            task: task.to_string(),
            capability: capability.to_string(),
            context: Value::Object(Default::default()),
            artifacts: Value::Object(Default::default()),
        }
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context = context;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub status: String,
    pub result: Value,
    pub artifacts: Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub capability: String,
    pub skill: SkillDefinition,
    pub execution: ExecutionResponse,
    pub lifecycle: LifecycleRecommendation,
}

#[derive(Debug, Clone)]
pub struct RouterPaths {
    pub workspace_root: PathBuf,
    pub skills_dir: PathBuf,
    pub state_dir: PathBuf,
    pub generated_skills_dir: PathBuf,
    pub registry_path: PathBuf,
    pub executions_log: PathBuf,
    pub trusted_sources_path: PathBuf,
    pub capabilities_dir: PathBuf,
}

impl RouterPaths {
    pub fn for_workspace(workspace_root: &Path) -> Self {
        let workspace_root = workspace_root.to_path_buf();
        let state_dir = workspace_root.join(".skill-router");
        Self {
            skills_dir: workspace_root.join("skills"),
            generated_skills_dir: state_dir.join("generated-skills"),
            registry_path: state_dir.join("registry.json"),
            executions_log: state_dir.join("executions.log"),
            trusted_sources_path: state_dir.join("trusted-sources.json"),
            capabilities_dir: workspace_root.join("capabilities"),
            workspace_root,
            state_dir,
        }
    }

    pub fn ensure_base_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.workspace_root)?;
        fs::create_dir_all(&self.state_dir)?;
        fs::create_dir_all(&self.generated_skills_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SkillStats {
    pub total_uses: usize,
    pub uses_30d: usize,
    pub success_rate: f64,
    pub last_used: Option<SystemTime>,
}
