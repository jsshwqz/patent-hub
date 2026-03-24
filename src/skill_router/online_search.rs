use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::Deserialize;

use super::types::{PermissionSet, RouterPaths, SkillDefinition, SkillMetadata, SkillSource};

#[derive(Debug, Deserialize)]
struct TrustedSourcesFile {
    #[serde(default)]
    sources: Vec<TrustedSource>,
}

#[derive(Debug, Deserialize)]
struct TrustedSource {
    name: String,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteCatalog {
    #[serde(default)]
    skills: Vec<RemoteSkillDefinition>,
}

#[derive(Debug, Deserialize)]
struct RemoteSkillDefinition {
    name: String,
    version: String,
    capabilities: Vec<String>,
    entrypoint: String,
    #[serde(default)]
    permissions: PermissionSet,
}

pub struct TrustedSourceSearch;

impl TrustedSourceSearch {
    pub fn search(paths: &RouterPaths, capability: &str) -> Result<Vec<SkillDefinition>> {
        if !paths.trusted_sources_path.exists() {
            return Ok(Vec::new());
        }

        let config: TrustedSourcesFile =
            serde_json::from_slice(&fs::read(&paths.trusted_sources_path)?)?;
        let mut results = Vec::new();

        for source in config.sources {
            if let Some(path) = source.path {
                let absolute = if path.is_absolute() {
                    path
                } else {
                    paths.workspace_root.join(path)
                };
                if !absolute.exists() {
                    continue;
                }
                let catalog: RemoteCatalog = serde_json::from_slice(&fs::read(&absolute)?)?;
                for skill in catalog
                    .skills
                    .into_iter()
                    .filter(|skill| skill.capabilities.iter().any(|item| item == capability))
                {
                    results.push(SkillDefinition {
                        metadata: SkillMetadata {
                            name: skill.name,
                            version: skill.version,
                            capabilities: skill.capabilities,
                            entrypoint: skill.entrypoint,
                            permissions: skill.permissions,
                        },
                        root_dir: absolute
                            .parent()
                            .unwrap_or(&paths.workspace_root)
                            .to_path_buf(),
                        source: SkillSource::RemoteCandidate,
                    });
                }
            } else if let Some(url) = source.url {
                if !url.starts_with("https://") {
                    continue;
                }
                let response = reqwest::blocking::get(&url)?;
                if !response.status().is_success() {
                    continue;
                }
                let catalog: RemoteCatalog = response.json()?;
                for skill in catalog
                    .skills
                    .into_iter()
                    .filter(|skill| skill.capabilities.iter().any(|item| item == capability))
                {
                    results.push(SkillDefinition {
                        metadata: SkillMetadata {
                            name: format!("{}::{}", source.name, skill.name),
                            version: skill.version,
                            capabilities: skill.capabilities,
                            entrypoint: skill.entrypoint,
                            permissions: skill.permissions,
                        },
                        root_dir: paths.workspace_root.clone(),
                        source: SkillSource::RemoteCandidate,
                    });
                }
            }
        }

        Ok(results)
    }
}
