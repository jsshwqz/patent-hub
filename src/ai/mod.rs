//! AI 多模型容灾客户端 / Multi-Provider AI Client with Failover
//!
//! 支持 6 种 AI 服务商自动切换：智谱 GLM、OpenRouter、Gemini、OpenAI、NVIDIA、DeepSeek。
//!
//! 模块结构：
//! - client: 核心 HTTP 客户端与多 provider failover
//! - chat: 聊天接口（单轮/多轮/流式）
//! - patent: 专利分析（摘要/权利要求/侵权/对比/批量）
//! - idea: 创意分析与图片描述

mod chat;
mod client;
mod idea;
mod patent;
mod tests;

pub use client::AiClient;
