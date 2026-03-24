pub mod capability_registry;
pub mod executor;
pub mod lifecycle;
pub mod loader;
pub mod matcher;
pub mod online_search;
pub mod planner;
pub mod registry;
pub mod security;
pub mod synth;
pub mod types;

use anyhow::Result;
use capability_registry::CapabilityRegistry;
use executor::Executor;
use lifecycle::LifecycleRecommendation;
use loader::Loader;
use matcher::Matcher;
use online_search::TrustedSourceSearch;
use planner::Planner;
use registry::RegistryStore;
use synth::Synthesizer;
use types::{ExecutionContext, RouteResult, RouterPaths, SkillDefinition};

pub struct SkillRouter {
    paths: RouterPaths,
    capability_registry: std::sync::Mutex<CapabilityRegistry>,
}

impl SkillRouter {
    pub fn new(paths: RouterPaths) -> Result<Self> {
        paths.ensure_base_dirs()?;
        let capability_registry = CapabilityRegistry::load_or_builtin(&paths)?;
        Ok(Self {
            paths,
            capability_registry: std::sync::Mutex::new(capability_registry),
        })
    }

    pub fn paths(&self) -> &RouterPaths {
        &self.paths
    }

    pub fn route(&self, task: &str) -> Result<RouteResult> {
        self.route_with_context(task, None)
    }

    pub fn route_with_context(&self, task: &str, context: Option<serde_json::Value>) -> Result<RouteResult> {
        let capability = {
            let mut reg = self.capability_registry.lock().unwrap();
            Planner::infer_capability_with_paths(task, &mut reg, &self.paths)?
                .ok_or_else(|| anyhow::anyhow!("could not infer capability for task: '{task}'"))?
        };
        self.route_inner(task, &capability, context)
    }

    pub fn route_with_capability(&self, task: &str, capability: &str, context: Option<serde_json::Value>) -> Result<RouteResult> {
        {
            let reg = self.capability_registry.lock().unwrap();
            reg.validate_name(capability)
                .map_err(|_| anyhow::anyhow!("unknown capability: {capability}"))?;
        }
        self.route_inner(task, capability, context)
    }

    fn route_inner(&self, task: &str, capability: &str, extra_context: Option<serde_json::Value>) -> Result<RouteResult> {
        let selected = {
            let reg = self.capability_registry.lock().unwrap();
            let local_skills = Loader::load_local_skills(&self.paths, &reg)?;
            let matching_local: Vec<SkillDefinition> = local_skills
                .into_iter()
                .filter(|skill| skill.supports_capability(capability))
                .collect();
            let trusted = TrustedSourceSearch::search(&self.paths, capability).unwrap_or_default();
            let registry_store = RegistryStore::load(&self.paths)?;
            let synthesized = if matching_local.is_empty() {
                vec![Synthesizer::create_placeholder(&self.paths, capability, task)?]
            } else {
                Vec::new()
            };
            if !matching_local.is_empty() {
                Matcher::select_best_with_registry(capability, &matching_local, &trusted, Some(&registry_store))?
            } else if !synthesized.is_empty() {
                Matcher::select_best_with_registry(capability, &synthesized, &trusted, Some(&registry_store))?
            } else {
                return Err(anyhow::anyhow!("no skill available for capability {capability}"));
            }
        };

        let exec_ctx = {
            let mut ctx = ExecutionContext::new(task, capability);
            if let Some(extra) = extra_context { ctx = ctx.with_context(extra); }
            ctx
        };

        let execution = Executor::execute(&selected, &exec_ctx, &self.paths)?;

        let mut registry = RegistryStore::load(&self.paths)?;
        registry.record_execution(&selected.metadata.name, execution.status == "ok", std::time::SystemTime::now());
        registry.save(&self.paths)?;

        let stats = registry.skill_stats(&selected.metadata.name)
            .ok_or_else(|| anyhow::anyhow!("missing registry stats for {}", selected.metadata.name))?;
        let lifecycle = LifecycleRecommendation::from_stats(&stats, std::time::SystemTime::now());

        Ok(RouteResult { capability: capability.to_string(), skill: selected, execution, lifecycle })
    }
}
