//! Step: 实验验证 / Experiment Validation
//!
//! 调用 AI 生成验证脚本，沙箱运行，捕获结果写入 context 和 findings。

use crate::ai::AiClient;
use crate::db::Database;
use crate::experiment::{generator, sandbox};
use crate::pipeline::context::PipelineContext;
use anyhow::Result;
use std::sync::Arc;

/// 执行实验验证步骤
pub async fn execute(
    ctx: &mut PipelineContext,
    ai: &AiClient,
    db: &Arc<Database>,
) -> Result<()> {
    // 1. AI 生成验证脚本
    tracing::info!("生成实验验证脚本...");
    let spec = match generator::generate_experiment(ctx, ai).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("实验脚本生成失败: {}", e);
            record_finding(ctx, db, "dead_end", "实验脚本生成失败", &e.to_string());
            return Ok(()); // 非关键步骤，不终止 Pipeline
        }
    };

    // 2. 沙箱运行
    tracing::info!("沙箱执行实验: {}", spec.title);
    let result = match sandbox::run_experiment(&spec).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("实验执行失败: {}", e);
            record_finding(ctx, db, "dead_end", "实验执行异常", &e.to_string());
            return Ok(());
        }
    };

    // 3. 记录结果
    let finding_type = if result.success { "experiment_result" } else { "dead_end" };
    let title = if result.success {
        format!("实验成功: {}", spec.title)
    } else {
        format!("实验失败 (exit={}): {}", result.exit_code, spec.title)
    };

    let content = format!(
        "假设: {}\n语言: {}\n耗时: {}ms\n退出码: {}\n指标: {}\n输出: {}",
        spec.hypothesis,
        result.language,
        result.duration_ms,
        result.exit_code,
        serde_json::to_string_pretty(&result.metrics).unwrap_or_default(),
        &result.stdout.chars().take(500).collect::<String>(),
    );

    record_finding(ctx, db, finding_type, &title, &content);
    ctx.experiment_results.push(result);

    tracing::info!("实验验证完成，共 {} 个结果", ctx.experiment_results.len());
    Ok(())
}

/// 写入 Finding 记录
fn record_finding(
    ctx: &PipelineContext,
    db: &Arc<Database>,
    finding_type: &str,
    title: &str,
    content: &str,
) {
    let finding = crate::db::version::Finding {
        id: uuid::Uuid::new_v4().to_string(),
        idea_id: ctx.idea_id.clone(),
        finding_type: finding_type.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        source_step: "ExperimentValidation".to_string(),
        branch_id: ctx.branch_id.clone(),
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    if let Err(e) = db.insert_finding(&finding) {
        tracing::warn!("Finding 写入失败: {}", e);
    }
}
