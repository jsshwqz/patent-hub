//! 编排引擎 / Orchestration Engine
//!
//! 替代 runner.rs 的线性循环，支持状态机驱动：
//! - Continue: 正常推进到下一步
//! - Jump: 跳转到指定步骤（迭代/回退）
//! - Branch: 创建分支，在新分支上继续
//! - Retry: 重试当前步骤
//! - Abort: 终止

use super::command::OrchestratorCommand;
use crate::ai::AiClient;
use crate::db::Database;
use crate::pipeline::context::{PipelineContext, PipelineProgress, StepResult, StepStatus};
use crate::pipeline::state::PipelineStep;
use crate::pipeline::steps;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

pub struct Orchestrator {
    ai_client: AiClient,
    db: Arc<Database>,
    serpapi_key: String,
    bing_api_key: String,
    lens_api_key: String,
    quick_mode: bool,
    /// 外部注入的命令（通过 iterate API 等触发）
    pending_command: Option<OrchestratorCommand>,
}

impl Orchestrator {
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
            pending_command: None,
        }
    }

    /// 注入一个命令（下次循环时处理）
    pub fn inject_command(&mut self, cmd: OrchestratorCommand) {
        self.pending_command = Some(cmd);
    }

    /// 状态机驱动的执行循环
    pub async fn run(
        &mut self,
        mut ctx: PipelineContext,
        progress_tx: Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
    ) -> Result<PipelineContext> {
        loop {
            // 1. 检查是否有待处理的命令
            if let Some(cmd) = self.pending_command.take() {
                match cmd {
                    OrchestratorCommand::Jump(target) => {
                        tracing::info!("Orchestrator: Jump to {:?}", target);
                        self.clear_step_side_effects(&mut ctx, target);
                        ctx.current_step = target;
                        continue;
                    }
                    OrchestratorCommand::Branch { name } => {
                        tracing::info!("Orchestrator: Branch '{}'", name);
                        let branch_id = super::create_branch(&self.db, &ctx, &name)?;
                        ctx.branch_id = branch_id;
                        continue;
                    }
                    OrchestratorCommand::Abort { reason } => {
                        tracing::info!("Orchestrator: Abort — {}", reason);
                        super::record_failure(&self.db, &ctx, ctx.current_step, &reason);
                        return Err(anyhow::anyhow!("Pipeline aborted: {}", reason));
                    }
                    OrchestratorCommand::Retry { max_attempts } => {
                        tracing::info!("Orchestrator: Retry (max {})", max_attempts);
                        // retry 逻辑在下面的执行流程中处理
                    }
                    OrchestratorCommand::Continue => {}
                }
            }

            let step = ctx.current_step;
            let step_start = Instant::now();

            // 2. 快速模式跳过
            if self.quick_mode && step.skipped_in_quick_mode() {
                self.send_progress(&progress_tx, &step, StepStatus::Skipped);
                ctx.step_results.push(StepResult {
                    step,
                    duration_ms: 0,
                    status: "skipped".into(),
                    error: None,
                });
                match self.decide_next(&ctx, step) {
                    Some(next) => {
                        ctx.current_step = next;
                        continue;
                    }
                    None => break,
                }
            }

            // 3. 发送 Running 进度
            self.send_progress(&progress_tx, &step, StepStatus::Running);

            // 4. 执行步骤
            let result = self.execute_step(&mut ctx, &progress_tx).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

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

            // 5. 处理结果
            match &result {
                Ok(()) => {
                    self.send_progress(&progress_tx, &step, StepStatus::Done);
                    // 保存快照（断点续跑）
                    if let Ok(json) = serde_json::to_string(&ctx) {
                        let _ = self.db.save_pipeline_snapshot(
                            &ctx.idea_id,
                            &json,
                            &format!("{:?}", step),
                        );
                    }
                    // 持久化研发状态机（跨续跑/跨版本可查询）
                    let _ = self
                        .db
                        .upsert_research_state(&ctx.idea_id, &ctx.research_state);
                    // 写入版本历史
                    super::save_version_snapshot(&self.db, &ctx, step);
                }
                Err(e) => {
                    if step.is_critical() {
                        self.send_progress(&progress_tx, &step, StepStatus::Error);
                        super::record_failure(&self.db, &ctx, step, &e.to_string());
                        return Err(anyhow::anyhow!(
                            "关键步骤「{}」失败: {}",
                            step.description(),
                            e
                        ));
                    }
                    tracing::warn!("非关键步骤 {:?} 失败: {}，跳过继续", step, e);
                    super::record_failure(&self.db, &ctx, step, &e.to_string());
                    self.send_progress(&progress_tx, &step, StepStatus::Skipped);
                }
            }

            // 6. 决定下一步（状态机核心）
            match self.decide_next(&ctx, step) {
                Some(next) => ctx.current_step = next,
                None => break,
            }
        }

        let _ = self.db.delete_pipeline_snapshot(&ctx.idea_id);
        Ok(ctx)
    }

    /// 状态机决策：根据当前状态决定下一步
    fn decide_next(&self, ctx: &PipelineContext, current: PipelineStep) -> Option<PipelineStep> {
        let diversity_gate_runs = ctx
            .step_results
            .iter()
            .filter(|r| r.step == PipelineStep::DiversityGate)
            .count();

        // DiversityGate 回退逻辑
        if should_fallback_diversity(
            self.quick_mode,
            current,
            ctx.diversity_score,
            ctx.retry_count,
            diversity_gate_runs,
        ) {
            // 正常最多回退 2 轮；若历史快照/状态异常导致 retry_count 未增长，也用 step 历史做硬上限兜底。
            tracing::info!(
                "多样性不足 ({:.0}%)，回退到 SearchWeb（第 {} 轮）",
                ctx.diversity_score * 100.0,
                diversity_gate_runs
            );
            return Some(PipelineStep::SearchWeb);
        }
        if !self.quick_mode && current == PipelineStep::DiversityGate && ctx.diversity_score < 0.3 {
            tracing::warn!(
                "多样性回退已达上限（retry_count={}, diversity_gate_runs={}），继续后续步骤",
                ctx.retry_count,
                diversity_gate_runs
            );
        }

        // 默认：线性推进
        current.next()
    }

    /// 清理跳转目标之后的副作用（重置被覆盖的 context 字段）
    fn clear_step_side_effects(&self, ctx: &mut PipelineContext, target: PipelineStep) {
        let target_idx = target.index();

        // 清理目标步骤之后的所有步骤产出
        if target_idx <= PipelineStep::SearchWeb.index() {
            ctx.web_results.clear();
            ctx.patent_results.clear();
        }
        if target_idx <= PipelineStep::ComputeSimilarity.index() {
            ctx.similarity_scores.clear();
            ctx.top_matches.clear();
        }
        if target_idx <= PipelineStep::ScoreNovelty.index() {
            ctx.novelty_score = 0.0;
            ctx.score_breakdown = Default::default();
        }
        if target_idx <= PipelineStep::AiDeepAnalysis.index() {
            ctx.ai_analysis.clear();
            ctx.action_plan.clear();
            ctx.deep_reasoning = Default::default();
        }

        // 保留 keywords/technical_domain/expanded_queries（用户可能手动调整过）
        tracing::info!("清理步骤 {:?} 之后的副作用", target);
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
            PipelineStep::DiversityGate => {
                let r = steps::diversity::execute(ctx).await;
                if r.is_ok() && ctx.diversity_score < 0.3 && ctx.retry_count < 2 {
                    ctx.retry_count += 1;
                }
                r
            }
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
                if result.is_ok() {
                    let _ = steps::analysis::extract_feature_cards(ctx, &self.db).await;
                    let _ =
                        steps::analysis::extract_feature_cards_ai(ctx, &self.ai_client, &self.db)
                            .await;
                }
                result
            }
            PipelineStep::ExperimentValidation => {
                steps::experiment::execute(ctx, &self.ai_client, &self.db).await
            }
            PipelineStep::BuildClaimTree => {
                steps::claim_tree::execute(ctx, &self.ai_client, &self.db).await
            }
            PipelineStep::Finalize => steps::finalize::execute(ctx, &self.db).await,
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

fn should_fallback_diversity(
    quick_mode: bool,
    current: PipelineStep,
    diversity_score: f64,
    retry_count: u32,
    diversity_gate_runs: usize,
) -> bool {
    if quick_mode || current != PipelineStep::DiversityGate || diversity_score >= 0.3 {
        return false;
    }
    // 正常路径：最多回退 2 轮（第 1/2 次 DiversityGate）
    // 异常路径：即使 retry_count 异常未增长，diversity_gate_runs 也会把回退硬限制在 2 次内。
    retry_count < 2 && diversity_gate_runs <= 2
}

#[cfg(test)]
mod tests {
    use super::should_fallback_diversity;
    use crate::pipeline::state::PipelineStep;

    #[test]
    fn diversity_fallback_first_two_rounds_allowed() {
        assert!(should_fallback_diversity(
            false,
            PipelineStep::DiversityGate,
            0.0,
            0,
            1
        ));
        assert!(should_fallback_diversity(
            false,
            PipelineStep::DiversityGate,
            0.1,
            1,
            2
        ));
    }

    #[test]
    fn diversity_fallback_stops_after_limit() {
        assert!(!should_fallback_diversity(
            false,
            PipelineStep::DiversityGate,
            0.0,
            2,
            3
        ));
    }

    #[test]
    fn diversity_fallback_has_hard_guard_when_retry_counter_stuck() {
        assert!(!should_fallback_diversity(
            false,
            PipelineStep::DiversityGate,
            0.0,
            0,
            3
        ));
    }
}
