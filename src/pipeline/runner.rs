//! Pipeline Runner — 状态机驱动器
//!
//! 按固定 12 步序列执行，代码编排 LLM，不让 LLM 做编排决策

use super::context::{PipelineContext, PipelineProgress, StepResult, StepStatus};
use super::state::PipelineStep;
use super::steps;
use crate::ai::AiClient;
use crate::db::Database;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

pub struct PipelineRunner {
    ai_client: AiClient,
    db: Arc<Database>,
    serpapi_key: String,
}

impl PipelineRunner {
    pub fn new(ai_client: AiClient, db: Arc<Database>, serpapi_key: String) -> Self {
        Self {
            ai_client,
            db,
            serpapi_key,
        }
    }

    /// 运行完整流水线，通过 channel 发送进度更新
    pub async fn run(
        &self,
        idea_id: &str,
        title: &str,
        description: &str,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<PipelineContext> {
        let mut ctx = PipelineContext::new(idea_id, title, description);
        tracing::info!("Pipeline runner started for idea: {}", idea_id);

        loop {
            let step = ctx.current_step;
            let step_start = Instant::now();
            tracing::info!("Pipeline step {:?} starting", step);

            // 发送进度
            self.send_progress(&progress_tx, &step, StepStatus::Running);

            // 执行当前步骤
            let result = self.execute_step(&mut ctx).await;
            tracing::info!("Pipeline step {:?} result: {:?}", step, result.is_ok());

            let duration_ms = step_start.elapsed().as_millis() as u64;

            // 记录步骤结果
            ctx.step_results.push(StepResult {
                step,
                duration_ms,
                status: if result.is_ok() {
                    "ok".into()
                } else {
                    "error".into()
                },
                error: result.as_ref().err().map(|e| e.to_string()),
            });

            match &result {
                Ok(()) => {
                    self.send_progress(&progress_tx, &step, StepStatus::Done);
                }
                Err(e) => {
                    if step.is_critical() {
                        self.send_progress(&progress_tx, &step, StepStatus::Error);
                        return Err(anyhow::anyhow!(
                            "关键步骤「{}」失败: {}",
                            step.description(),
                            e
                        ));
                    }
                    // 非关键步骤失败，跳过继续
                    tracing::warn!(
                        "非关键步骤 {:?} 失败: {}，跳过继续",
                        step,
                        e
                    );
                    self.send_progress(&progress_tx, &step, StepStatus::Skipped);
                }
            }

            // DiversityGate 可能触发回退
            if step == PipelineStep::DiversityGate && ctx.diversity_score < 0.3 && ctx.retry_count < 2 {
                ctx.retry_count += 1;
                ctx.current_step = PipelineStep::SearchWeb;
                tracing::info!("多样性不足 ({:.0}%)，补充搜索第 {} 轮", ctx.diversity_score * 100.0, ctx.retry_count);
                continue;
            }

            // 转移到下一步
            match step.next() {
                Some(next) => ctx.current_step = next,
                None => break,
            }
        }

        Ok(ctx)
    }

    /// 执行单个步骤
    async fn execute_step(&self, ctx: &mut PipelineContext) -> Result<()> {
        match ctx.current_step {
            PipelineStep::ParseInput => steps::parse::execute(ctx).await,
            PipelineStep::ExpandQuery => steps::expand::execute(ctx, &self.ai_client).await,
            PipelineStep::SearchWeb => steps::search::search_web(ctx, &self.serpapi_key).await,
            PipelineStep::SearchPatents => {
                steps::search::search_patents(ctx, &self.serpapi_key, &self.db).await
            }
            PipelineStep::DiversityGate => steps::diversity::execute(ctx).await,
            PipelineStep::ComputeSimilarity => steps::similarity::execute(ctx).await,
            PipelineStep::RankAndFilter => steps::rank::execute(ctx).await,
            PipelineStep::DetectContradictions => steps::contradiction::execute(ctx).await,
            PipelineStep::ScoreNovelty => steps::scoring::execute(ctx).await,
            PipelineStep::AiDeepAnalysis => {
                steps::analysis::deep_analysis(ctx, &self.ai_client).await
            }
            PipelineStep::AiActionPlan => {
                steps::analysis::action_plan(ctx, &self.ai_client).await
            }
            PipelineStep::Finalize => steps::finalize::execute(ctx).await,
        }
    }

    fn send_progress(
        &self,
        tx: &Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
        step: &PipelineStep,
        status: StepStatus,
    ) {
        if let Some(tx) = tx {
            let _ = tx.send(PipelineProgress {
                step: *step,
                step_index: step.index(),
                total_steps: PipelineStep::TOTAL_STEPS,
                status,
                message: step.description().to_string(),
            });
        }
    }
}
