//! Pipeline Runner — Orchestrator 的薄包装
//!
//! 保持对外接口不变（run/resume），内部委托给 Orchestrator 状态机。

use super::context::{PipelineContext, PipelineProgress};
use crate::ai::AiClient;
use crate::db::Database;
use crate::orchestrator::engine::Orchestrator;
use anyhow::Result;
use std::sync::Arc;

pub struct PipelineRunner {
    ai_client: AiClient,
    db: Arc<Database>,
    serpapi_key: String,
    bing_api_key: String,
    lens_api_key: String,
    quick_mode: bool,
}

impl PipelineRunner {
    pub fn new(
        ai_client: AiClient,
        db: Arc<Database>,
        serpapi_key: String,
        bing_api_key: String,
        lens_api_key: String,
        quick_mode: bool,
    ) -> Self {
        Self {
            ai_client,
            db,
            serpapi_key,
            bing_api_key,
            lens_api_key,
            quick_mode,
        }
    }

    fn make_orchestrator(&self) -> Orchestrator {
        Orchestrator::new(
            self.ai_client.clone(),
            self.db.clone(),
            self.serpapi_key.clone(),
            self.bing_api_key.clone(),
            self.lens_api_key.clone(),
            self.quick_mode,
        )
    }

    /// 断点续跑
    pub async fn resume(
        &self,
        idea_id: &str,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<Option<PipelineContext>> {
        match self.db.load_pipeline_snapshot(idea_id)? {
            Some((json, _step)) => {
                let mut ctx: PipelineContext = serde_json::from_str(&json)
                    .map_err(|e| anyhow::anyhow!("快照反序列化失败: {}", e))?;
                if let Some(next) = ctx.current_step.next() {
                    ctx.current_step = next;
                    tracing::info!("Pipeline 断点续跑: {} 从 {:?} 继续", idea_id, next);
                    let mut orch = self.make_orchestrator();
                    let result = orch.run(ctx, progress_tx).await?;
                    Ok(Some(result))
                } else {
                    let _ = self.db.delete_pipeline_snapshot(idea_id);
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// 运行完整流水线
    pub async fn run(
        &self,
        idea_id: &str,
        title: &str,
        description: &str,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<PipelineContext> {
        let ctx = PipelineContext::new(idea_id, title, description);
        tracing::info!("Pipeline runner started for idea: {}", idea_id);
        let mut orch = self.make_orchestrator();
        orch.run(ctx, progress_tx).await
    }
}
