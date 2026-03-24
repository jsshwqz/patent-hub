use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use patent_hub::skill_router::{
    capability_registry::CapabilityRegistry,
    executor::Executor,
    lifecycle::LifecycleRecommendation,
    loader::Loader,
    matcher::Matcher,
    planner::Planner,
    registry::RegistryStore,
    synth::Synthesizer,
    types::{ExecutionContext, PermissionSet, RouterPaths, SkillSource},
    SkillRouter,
};
use serde_json::json;

fn temp_workspace(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skill-router-{name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_skill(
    workspace: &Path,
    skill_name: &str,
    capability: &str,
    entrypoint: &str,
    permissions: serde_json::Value,
) {
    let skill_dir = workspace.join("skills").join(skill_name);
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("skill.json"),
        serde_json::to_vec_pretty(&json!({
            "name": skill_name,
            "version": "0.1.0",
            "capabilities": [capability],
            "entrypoint": entrypoint,
            "permissions": permissions,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn planner_infers_yaml_parse_and_validates_registry_names() {
    let registry = CapabilityRegistry::builtin();

    assert_eq!(
        Planner::infer_capability("parse this yaml", &registry).unwrap(),
        Some("yaml_parse".to_string())
    );
    assert!(registry.validate_name("yaml_parse").is_ok());
    assert!(registry.validate_name("yaml-parse").is_err());
}

#[test]
fn loader_reads_local_skills_and_matcher_prefers_local_candidates() {
    let workspace = temp_workspace("loader");
    write_skill(
        &workspace,
        "local_yaml",
        "yaml_parse",
        "builtin:yaml_parse",
        json!({
            "network": false,
            "filesystem_read": true,
            "filesystem_write": false,
            "process_exec": false
        }),
    );

    let paths = RouterPaths::for_workspace(&workspace);
    let registry = CapabilityRegistry::builtin();
    let local_skills = Loader::load_local_skills(&paths, &registry).unwrap();

    assert_eq!(local_skills.len(), 1);
    assert_eq!(local_skills[0].metadata.name, "local_yaml");

    let synthesized =
        Synthesizer::placeholder_definition(&paths, "yaml_parse", "parse this yaml").unwrap();

    let selected = Matcher::select_best("yaml_parse", &local_skills, &[synthesized]).unwrap();
    assert_eq!(selected.metadata.name, "local_yaml");
    assert_eq!(selected.source, SkillSource::Local);
}

#[test]
fn executor_denies_unsafe_permissions_and_logs_safe_execution() {
    let workspace = temp_workspace("executor");
    write_skill(
        &workspace,
        "safe_echo",
        "text_summarize",
        "builtin:echo",
        json!({
            "filesystem_read": true
        }),
    );
    write_skill(
        &workspace,
        "unsafe_writer",
        "text_summarize",
        "../outside.sh",
        json!({
            "filesystem_read": true,
            "filesystem_write": true
        }),
    );

    let paths = RouterPaths::for_workspace(&workspace);
    let registry = CapabilityRegistry::builtin();
    let skills = Loader::load_local_skills(&paths, &registry).unwrap();
    let safe_skill = skills
        .iter()
        .find(|skill| skill.metadata.name == "safe_echo")
        .unwrap();
    let unsafe_skill = skills
        .iter()
        .find(|skill| skill.metadata.name == "unsafe_writer")
        .unwrap();

    assert_eq!(
        safe_skill.metadata.permissions,
        PermissionSet::default_deny().with_filesystem_read(true)
    );
    assert!(Executor::validate_permissions(unsafe_skill, &paths).is_err());

    let response = Executor::execute(
        safe_skill,
        &ExecutionContext::new("summarize this text", "text_summarize")
            .with_context(json!({"input": "hello"})),
        &paths,
    )
    .unwrap();

    assert_eq!(response.status, "ok");
    assert_eq!(response.result["task"], "summarize this text");
    assert!(paths.executions_log.exists());
}

#[test]
fn synthesized_skill_is_created_inside_workspace_and_executable() {
    let workspace = temp_workspace("synth");
    let paths = RouterPaths::for_workspace(&workspace);

    let synthesized =
        Synthesizer::create_placeholder(&paths, "pdf_parse", "parse this pdf").unwrap();

    assert!(synthesized
        .root_dir
        .starts_with(&paths.generated_skills_dir));
    assert!(synthesized.root_dir.join("skill.json").exists());

    let response = Executor::execute(
        &synthesized,
        &ExecutionContext::new("parse this pdf", "pdf_parse"),
        &paths,
    )
    .unwrap();

    assert_eq!(response.status, "ok");
    assert_eq!(response.result["capability"], "pdf_parse");
}

#[test]
fn end_to_end_pipeline_updates_registry_and_lifecycle() {
    let workspace = temp_workspace("end-to-end");
    write_skill(
        &workspace,
        "yaml_local",
        "yaml_parse",
        "builtin:yaml_parse",
        json!({
            "filesystem_read": true
        }),
    );

    let router = SkillRouter::new(RouterPaths::for_workspace(&workspace)).unwrap();
    let first = router.route("parse this yaml").unwrap();

    assert_eq!(first.capability, "yaml_parse");
    assert_eq!(first.skill.metadata.name, "yaml_local");
    assert_eq!(first.execution.status, "ok");

    for _ in 0..14 {
        router.route("parse this yaml").unwrap();
    }

    let registry = RegistryStore::load(router.paths()).unwrap();
    let stats = registry.skill_stats("yaml_local").unwrap();
    assert_eq!(stats.uses_30d, 15);
    assert_eq!(
        LifecycleRecommendation::from_stats(&stats, SystemTime::now()),
        LifecycleRecommendation::PublishCandidate
    );
}

#[test]
fn registry_marks_old_unused_skills_for_deprecation_and_purge() {
    let workspace = temp_workspace("lifecycle");
    let paths = RouterPaths::for_workspace(&workspace);
    let mut registry = RegistryStore::load(&paths).unwrap();

    registry.record_synthetic_stats(
        "stale_90",
        0,
        Some(SystemTime::now() - Duration::from_secs(91 * 24 * 60 * 60)),
    );
    registry.record_synthetic_stats(
        "stale_180",
        0,
        Some(SystemTime::now() - Duration::from_secs(181 * 24 * 60 * 60)),
    );

    let stale_90 = registry.skill_stats("stale_90").unwrap();
    let stale_180 = registry.skill_stats("stale_180").unwrap();

    assert_eq!(
        LifecycleRecommendation::from_stats(&stale_90, SystemTime::now()),
        LifecycleRecommendation::Deprecate
    );
    assert_eq!(
        LifecycleRecommendation::from_stats(&stale_180, SystemTime::now()),
        LifecycleRecommendation::PurgeCandidate
    );
}
