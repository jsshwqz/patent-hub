//! 编排器命令 / Orchestrator commands
//!
//! 定义 Pipeline 状态机的控制指令，支持迭代、跳转、分支、回退。

use crate::pipeline::state::PipelineStep;

/// 编排器控制命令
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum OrchestratorCommand {
    /// 继续执行下一步
    Continue,
    /// 跳转到指定步骤（清理中间副作用后重跑）
    Jump(PipelineStep),
    /// 重试当前步骤
    Retry { max_attempts: u8 },
    /// 创建分支（快照当前 context，在新分支上继续）
    Branch { name: String },
    /// 终止执行
    Abort { reason: String },
}
