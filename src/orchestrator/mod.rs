//! 编排器 / Orchestrator
//!
//! 替代原有 runner.rs 的线性编排，支持状态机驱动的迭代、跳转、分支。
//! 当前版本保持向后兼容：默认行为与原线性 Pipeline 一致，
//! 通过 OrchestratorCommand 可触发高级控制流。

#[allow(dead_code)]
pub mod command;

#[allow(unused_imports)]
pub use command::OrchestratorCommand;

use crate::db::Database;
use crate::db::version::{IdeaVersion, IdeaBranch, Finding};
use crate::pipeline::context::PipelineContext;
use crate::pipeline::state::PipelineStep;
use anyhow::Result;
use std::sync::Arc;

/// 保存版本快照到 idea_versions 表
pub fn save_version_snapshot(
    db: &Arc<Database>,
    ctx: &PipelineContext,
    step: PipelineStep,
) {
    let version_number = db
        .get_next_version_number(&ctx.idea_id)
        .unwrap_or(1);

    let version = IdeaVersion {
        id: uuid::Uuid::new_v4().to_string(),
        idea_id: ctx.idea_id.clone(),
        version_number,
        context_json: serde_json::to_string(ctx).unwrap_or_default(),
        current_step: format!("{:?}", step),
        branch_id: ctx.branch_id.clone(),
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };

    if let Err(e) = db.insert_idea_version(&version) {
        tracing::warn!("版本快照保存失败: {}", e);
    }
}

/// 记录失败路径到 findings 表
pub fn record_failure(
    db: &Arc<Database>,
    ctx: &PipelineContext,
    step: PipelineStep,
    error: &str,
) {
    let finding = Finding {
        id: uuid::Uuid::new_v4().to_string(),
        idea_id: ctx.idea_id.clone(),
        finding_type: "dead_end".to_string(),
        title: format!("步骤「{}」失败", step.description()),
        content: error.to_string(),
        source_step: format!("{:?}", step),
        branch_id: ctx.branch_id.clone(),
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };

    if let Err(e) = db.insert_finding(&finding) {
        tracing::warn!("失败记录写入失败: {}", e);
    }
}

/// 创建新分支
#[allow(dead_code)]
pub fn create_branch(
    db: &Arc<Database>,
    ctx: &PipelineContext,
    branch_name: &str,
) -> Result<String> {
    let branch_id = uuid::Uuid::new_v4().to_string();
    let branch = IdeaBranch {
        id: branch_id.clone(),
        idea_id: ctx.idea_id.clone(),
        name: branch_name.to_string(),
        parent_branch_id: ctx.branch_id.clone(),
        parent_version_id: Some(ctx.parent_version_id.clone()),
        status: "active".to_string(),
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    db.insert_idea_branch(&branch)?;
    tracing::info!("创建分支 '{}' (id: {})", branch_name, branch_id);
    Ok(branch_id)
}
