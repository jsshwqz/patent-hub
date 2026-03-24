use std::{collections::BTreeMap, fs};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::types::RouterPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    pub name: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    definitions: BTreeMap<String, CapabilityDefinition>,
}

impl CapabilityRegistry {
    pub fn builtin() -> Self {
        let mut registry = Self::default();
        for definition in [
            CapabilityDefinition {
                name: "yaml_parse".to_string(),
                description: "Parse YAML text into structured JSON data".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["parsed".to_string()],
            },
            CapabilityDefinition {
                name: "json_parse".to_string(),
                description: "Parse and validate JSON text into structured data".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["parsed".to_string()],
            },
            CapabilityDefinition {
                name: "toml_parse".to_string(),
                description: "Parse TOML configuration text into structured data".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["parsed".to_string()],
            },
            CapabilityDefinition {
                name: "csv_parse".to_string(),
                description: "Parse CSV or spreadsheet text into rows and columns".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["rows".to_string(), "headers".to_string()],
            },
            CapabilityDefinition {
                name: "pdf_parse".to_string(),
                description: "Extract and structure text content from a PDF file path".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["structured_data".to_string()],
            },
            CapabilityDefinition {
                name: "markdown_render".to_string(),
                description: "Parse Markdown text into structured sections".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["sections".to_string()],
            },
            CapabilityDefinition {
                name: "text_summarize".to_string(),
                description: "Summarize text using AI into a concise output".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "text_translate".to_string(),
                description: "Translate text from one language to another using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "text_classify".to_string(),
                description: "Classify or categorize text into a label using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "text_extract".to_string(),
                description: "Extract key entities and information from text using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "text_diff".to_string(),
                description: "Compute a line-level diff between two text inputs".to_string(),
                inputs: vec!["a".to_string(), "b".to_string()],
                outputs: vec!["diff".to_string(), "added".to_string(), "removed".to_string()],
            },
            CapabilityDefinition {
                name: "text_embed".to_string(),
                description: "Compute a term-frequency bag-of-words vector for text".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["vector".to_string()],
            },
            CapabilityDefinition {
                name: "web_search".to_string(),
                description: "Search the web via SerpAPI and return organic results".to_string(),
                inputs: vec!["query".to_string()],
                outputs: vec!["results".to_string()],
            },
            CapabilityDefinition {
                name: "http_fetch".to_string(),
                description: "Fetch the body of an HTTPS URL".to_string(),
                inputs: vec!["url".to_string()],
                outputs: vec!["body".to_string(), "status".to_string()],
            },
            CapabilityDefinition {
                name: "image_describe".to_string(),
                description: "Describe an image at a given path or URL using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "code_generate".to_string(),
                description: "Generate Rust code for a given requirement using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "code_test".to_string(),
                description: "Write Rust unit tests for given code using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
            CapabilityDefinition {
                name: "code_lint".to_string(),
                description: "Review Rust code for issues and suggest fixes using AI".to_string(),
                inputs: vec!["text".to_string()],
                outputs: vec!["output".to_string()],
            },
        ] {
            registry
                .definitions
                .insert(definition.name.clone(), definition);
        }
        registry
    }

    pub fn load_or_builtin(paths: &RouterPaths) -> Result<Self> {
        let mut registry = Self::builtin();
        if !paths.capabilities_dir.exists() {
            return Ok(registry);
        }

        for entry in fs::read_dir(&paths.capabilities_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            {
                let definition: CapabilityDefinition =
                    serde_json::from_slice(&fs::read(entry.path())?)?;
                registry.validate_name(&definition.name)?;
                registry
                    .definitions
                    .insert(definition.name.clone(), definition);
            }
        }

        Ok(registry)
    }

    pub fn validate_name(&self, name: &str) -> Result<()> {
        let is_valid = !name.is_empty()
            && !name.starts_with('_')
            && !name.ends_with('_')
            && name
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');

        if is_valid {
            Ok(())
        } else {
            Err(anyhow!("invalid capability name: {name}"))
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }

    pub fn definitions(&self) -> impl Iterator<Item = &CapabilityDefinition> {
        self.definitions.values()
    }

    /// Write a newly discovered capability to capabilities/ dir so it survives restarts.
    pub fn persist_discovered(&self, name: &str, task: &str) -> anyhow::Result<()> {
        // We don't have paths here, so write to a temp location the caller can move.
        // Instead, callers should use persist_to_dir directly.
        let _ = (name, task);
        Ok(())
    }

    pub fn persist_to_dir(&mut self, name: &str, task: &str, capabilities_dir: &std::path::Path) -> anyhow::Result<()> {
        if self.contains(name) { return Ok(()); }
        std::fs::create_dir_all(capabilities_dir)?;
        let def = CapabilityDefinition {
            name: name.to_string(),
            description: format!("Auto-discovered capability for: {}", task),
            inputs: vec!["text".to_string()],
            outputs: vec!["output".to_string()],
        };
        std::fs::write(
            capabilities_dir.join(format!("{}.json", name)),
            serde_json::to_vec_pretty(&def)?,
        )?;
        self.definitions.insert(name.to_string(), def);
        Ok(())
    }
}
