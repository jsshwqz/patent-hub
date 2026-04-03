//! Pipeline Runner — 状态机驱动器
//!
//! 按固定 13 步序列执行，代码编排 LLM，不让 LLM 做编排决策

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

    /// 断点续跑：从快照恢复上次中断的管道 / Resume pipeline from last snapshot
    pub async fn resume(
        &self,
        idea_id: &str,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<Option<PipelineContext>> {
        match self.db.load_pipeline_snapshot(idea_id)? {
            Some((json, _step)) => {
                let mut ctx: PipelineContext = serde_json::from_str(&json)
                    .map_err(|e| anyhow::anyhow!("快照反序列化失败: {}", e))?;
                // 从下一步继续（当前步已完成）
                if let Some(next) = ctx.current_step.next() {
                    ctx.current_step = next;
                    tracing::info!("Pipeline 断点续跑: {} 从 {:?} 继续", idea_id, next);
                    let result = self.run_from_ctx(ctx, progress_tx).await?;
                    Ok(Some(result))
                } else {
                    // 已到最后一步，无需续跑
                    let _ = self.db.delete_pipeline_snapshot(idea_id);
                    Ok(None)
                }
            }
            None => Ok(None),
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
        let ctx = PipelineContext::new(idea_id, title, description);
        tracing::info!("Pipeline runner started for idea: {}", idea_id);
        self.run_from_ctx(ctx, progress_tx).await
    }

    /// 内部实现：从给定 context 开始执行 / Inner: run pipeline from given context
    async fn run_from_ctx(
        &self,
        mut ctx: PipelineContext,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<PipelineContext> {

        loop {
            let step = ctx.current_step;
            let step_start = Instant::now();
            tracing::info!("Pipeline step {:?} starting", step);

            // 快速模式：跳过非必要步骤
            if self.quick_mode && step.skipped_in_quick_mode() {
                tracing::info!("Quick mode: skipping step {:?}", step);
                self.send_progress(&progress_tx, &step, StepStatus::Skipped);
                ctx.step_results.push(StepResult {
                    step,
                    duration_ms: 0,
                    status: "skipped".into(),
                    error: None,
                });
                match step.next() {
                    Some(next) => {
                        ctx.current_step = next;
                        continue;
                    }
                    None => break,
                }
            }

            // 发送进度
            self.send_progress(&progress_tx, &step, StepStatus::Running);

            // 执行当前步骤
            let result = self.execute_step(&mut ctx, &progress_tx).await;
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
                    // 每步成功后保存快照，支持断点续跑 / Save snapshot for resume support
                    if let Ok(json) = serde_json::to_string(&ctx) {
                        let _ = self.db.save_pipeline_snapshot(
                            &ctx.idea_id,
                            &json,
                            &format!("{:?}", step),
                        );
                    }
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
                    tracing::warn!("非关键步骤 {:?} 失败: {}，跳过继续", step, e);
                    self.send_progress(&progress_tx, &step, StepStatus::Skipped);
                }
            }

            // DiversityGate 可能触发回退
            if step == PipelineStep::DiversityGate
                && ctx.diversity_score < 0.3
                && ctx.retry_count < 2
            {
                ctx.retry_count += 1;
                ctx.current_step = PipelineStep::SearchWeb;
                tracing::info!(
                    "多样性不足 ({:.0}%)，补充搜索第 {} 轮",
                    ctx.diversity_score * 100.0,
                    ctx.retry_count
                );
                continue;
            }

            // 转移到下一步
            match step.next() {
                Some(next) => ctx.current_step = next,
                None => break,
            }
        }

        // 管道完成，清理快照 / Pipeline done, remove snapshot
        let _ = self.db.delete_pipeline_snapshot(&ctx.idea_id);
        Ok(ctx)
    }

    /// 执行单个步骤
    async fn execute_step(
        &self,
        ctx: &mut PipelineContext,
        progress_tx: &Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<()> {
        match ctx.current_step {
            PipelineStep::ParseInput => steps::parse::execute(ctx).await,
            PipelineStep::ExpandQuery => steps::expand::execute(ctx, &self.ai_client).await,
            PipelineStep::SearchWeb => {
                steps::search::search_web(ctx, &self.serpapi_key, &self.bing_api_key, &self.db)
                    .await
            }
            PipelineStep::SearchPatents => {
                steps::search::search_patents(ctx, &self.serpapi_key, &self.lens_api_key, &self.db)
                    .await
            }
            PipelineStep::DiversityGate => steps::diversity::execute(ctx).await,
            PipelineStep::ComputeSimilarity => steps::similarity::execute(ctx).await,
            PipelineStep::RankAndFilter => steps::rank::execute(ctx).await,
            PipelineStep::PriorArtCluster => steps::prior_art_cluster::execute(ctx).await,
            PipelineStep::DetectContradictions => steps::contradiction::execute(ctx).await,
            PipelineStep::ScoreNovelty => steps::scoring::execute(ctx).await,
            PipelineStep::AiDeepAnalysis => {
                steps::analysis::deep_analysis(ctx, &self.ai_client, progress_tx).await
            }
            PipelineStep::AiActionPlan => {
                let result = steps::analysis::action_plan(ctx, &self.ai_client).await;
                // AI 分析完成后，自动提取特征卡片存入数据库
                // Auto-extract feature cards after AI analysis completes
                if result.is_ok() {
                    if let Err(e) = steps::analysis::extract_feature_cards(ctx, &self.db).await {
                        tracing::warn!("特征卡片提取失败（不影响管道）: {}", e);
                    }
                }
                result
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
                sub_step: None,
                sub_progress: None,
            });
        }
    }
}
