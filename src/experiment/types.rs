//! 实验验证引擎类型定义 / Experiment verification engine types

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 实验规格 — 描述要运行什么实验
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSpec {
    /// 实验标题
    pub title: String,
    /// 脚本语言 "python" | "rust"
    pub language: String,
    /// 生成的脚本源码
    pub script_content: String,
    /// 期望验证的假设
    pub hypothesis: String,
    /// 超时时间（秒）
    pub timeout_secs: u64,
}

/// 实验状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentStatus {
    /// 等待执行
    Pending,
    /// 正在运行
    Running,
    /// 执行成功
    Success,
    /// 执行失败（脚本错误或超时）
    Failed,
    /// 脚本生成失败
    GenerationFailed,
}

/// 实验指标 — 从脚本输出中提取的结构化数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentMetrics {
    /// 指标键值对
    pub values: std::collections::HashMap<String, f64>,
    /// 原始输出行
    pub raw_lines: Vec<String>,
}
