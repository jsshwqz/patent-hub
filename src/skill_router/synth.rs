use std::fs;

use anyhow::Result;
use serde_json::json;

use super::types::{PermissionSet, RouterPaths, SkillDefinition, SkillMetadata, SkillSource};

pub struct Synthesizer;

impl Synthesizer {
    /// Build an in-memory SkillDefinition for a generated skill (no disk write).
    pub fn placeholder_definition(
        paths: &RouterPaths,
        capability: &str,
        _task: &str,
    ) -> Result<SkillDefinition> {
        let (entrypoint, permissions) = Self::builtin_for_capability(capability);
        let root_dir = paths.generated_skills_dir.join(format!("{capability}_gen"));
        Ok(SkillDefinition {
            metadata: SkillMetadata {
                name: format!("{capability}_gen"),
                version: "0.1.0".to_string(),
                capabilities: vec![capability.to_string()],
                entrypoint,
                permissions,
            },
            root_dir,
            source: SkillSource::Generated,
        })
    }

    /// Write the skill manifest + README to disk and return the definition.
    pub fn create_placeholder(
        paths: &RouterPaths,
        capability: &str,
        task: &str,
    ) -> Result<SkillDefinition> {
        let def = Self::placeholder_definition(paths, capability, task)?;
        fs::create_dir_all(&def.root_dir)?;

        fs::write(
            def.root_dir.join("skill.json"),
            serde_json::to_vec_pretty(&json!({
                "name":         def.metadata.name,
                "version":      def.metadata.version,
                "capabilities": def.metadata.capabilities,
                "entrypoint":   def.metadata.entrypoint,
                "permissions":  def.metadata.permissions,
                "generated_for": task,
            }))?,
        )?;

        let (description, usage_example) = Self::describe_capability(capability, task);
        fs::write(
            def.root_dir.join("README.md"),
            format!(
                "# {name}\n\n\
                 **Capability:** `{cap}`  \n\
                 **Entrypoint:** `{ep}`  \n\
                 **Generated for task:** {task}\n\n\
                 ## What this skill does\n\n\
                 {description}\n\n\
                 ## Usage\n\n\
                 ```json\n{usage}\n```\n\n\
                 ## Permissions\n\n\
                 {perms}\n",
                name        = def.metadata.name,
                cap         = capability,
                ep          = def.metadata.entrypoint,
                task        = task,
                description = description,
                usage       = usage_example,
                perms       = Self::describe_permissions(&def.metadata.permissions),
            ),
        )?;

        Ok(def)
    }

    fn builtin_for_capability(capability: &str) -> (String, PermissionSet) {
        match capability {
            "yaml_parse"      => ("builtin:yaml_parse".into(),      PermissionSet::default_deny().with_filesystem_read(true)),
            "json_parse"      => ("builtin:json_parse".into(),      PermissionSet::default_deny().with_filesystem_read(true)),
            "toml_parse"      => ("builtin:toml_parse".into(),      PermissionSet::default_deny().with_filesystem_read(true)),
            "csv_parse"       => ("builtin:csv_parse".into(),       PermissionSet::default_deny().with_filesystem_read(true)),
            "markdown_render" => ("builtin:markdown_render".into(), PermissionSet::default_deny().with_filesystem_read(true)),
            "text_diff"       => ("builtin:text_diff".into(),       PermissionSet::default_deny()),
            "text_embed"      => ("builtin:text_embed".into(),      PermissionSet::default_deny()),
            "text_summarize"  => ("builtin:text_summarize".into(),  PermissionSet::default_deny().with_network(true)),
            "text_translate"  => ("builtin:text_translate".into(),  PermissionSet::default_deny().with_network(true)),
            "text_classify"   => ("builtin:text_classify".into(),   PermissionSet::default_deny().with_network(true)),
            "text_extract"    => ("builtin:text_extract".into(),    PermissionSet::default_deny().with_network(true)),
            "web_search"      => ("builtin:web_search".into(),      PermissionSet::default_deny().with_network(true)),
            "http_fetch"      => ("builtin:http_fetch".into(),      PermissionSet::default_deny().with_network(true)),
            "code_generate"   => ("builtin:code_generate".into(),   PermissionSet::default_deny().with_network(true)),
            "code_test"       => ("builtin:code_test".into(),       PermissionSet::default_deny().with_network(true)),
            "code_lint"       => ("builtin:code_lint".into(),       PermissionSet::default_deny().with_network(true)),
            "image_describe"  => ("builtin:image_describe".into(),  PermissionSet::default_deny().with_network(true).with_filesystem_read(true)),
            "pdf_parse"       => ("builtin:pdf_parse".into(),       PermissionSet::default_deny().with_network(true).with_filesystem_read(true)),
            _                 => ("builtin:placeholder".into(),     PermissionSet::default_deny()),
        }
    }

    fn describe_capability(capability: &str, task: &str) -> (String, String) {
        match capability {
            "yaml_parse" => (
                "Parses YAML text into structured JSON using a built-in Rust parser.".into(),
                r#"{"capability":"yaml_parse","context":{"text":"key: value\nlist:\n- a\n- b"}}"#.into(),
            ),
            "json_parse" => (
                "Parses and validates JSON text using serde_json.".into(),
                r#"{"capability":"json_parse","context":{"text":"{\"key\":\"value\"}"}}"#.into(),
            ),
            "text_summarize" => (
                "Sends the input text to the configured AI model and returns a concise summary.".into(),
                r#"{"capability":"text_summarize","context":{"text":"<your text here>"}}"#.into(),
            ),
            "web_search" => (
                "Queries SerpAPI with the given query and returns organic results. Requires SERPAPI_KEY.".into(),
                format!(r#"{{"capability":"web_search","context":{{"query":"{}"}}}}"#, task),
            ),
            "text_diff" => (
                "Computes a line-level diff between two text inputs (a and b).".into(),
                r#"{"capability":"text_diff","context":{"a":"line1\nline2","b":"line1\nline3"}}"#.into(),
            ),
            "http_fetch" => (
                "Fetches the body of an HTTPS URL. Only HTTPS is allowed.".into(),
                r#"{"capability":"http_fetch","context":{"url":"https://example.com"}}"#.into(),
            ),
            _ => (
                format!("Generated skill for capability `{capability}`. Original task: {task}"),
                format!(r#"{{"capability":"{capability}","context":{{"text":"<input>"}}}}"#),
            ),
        }
    }

    fn describe_permissions(p: &PermissionSet) -> String {
        [
            format!("- network: {}", p.network),
            format!("- filesystem_read: {}", p.filesystem_read),
            format!("- filesystem_write: {}", p.filesystem_write),
            format!("- process_exec: {}", p.process_exec),
        ]
        .join("\n")
    }
}
