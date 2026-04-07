//! Step 13: Finalize — 汇总报告，持久化到数据库
//!
//! 类型：CODE

use crate::db::Database;
use crate::pipeline::context::PipelineContext;
use anyhow::Result;
use std::sync::Arc;

/// 执行 Step 13（持久化证据链到数据库）
pub async fn execute(ctx: &mut PipelineContext, db: &Arc<Database>) -> Result<()> {
    // 确保评分在合理范围内
    ctx.novelty_score = ctx.novelty_score.clamp(0.0, 100.0);

    // 如果 AI 分析为空（AI 不可用），生成基于代码计算的纯数据报告
    if ctx.ai_analysis.is_empty() {
        ctx.ai_analysis = generate_code_only_report(ctx);
    }

    // 持久化证据链到数据库 / Persist evidence chain to database
    if !ctx.evidence_chain.is_empty() {
        // 先清除旧证据（重新运行 pipeline 时）
        let _ = db.delete_evidence_by_idea(&ctx.idea_id);
        if let Err(e) = db.insert_evidence_batch(&ctx.evidence_chain) {
            tracing::warn!("证据链持久化失败: {}", e);
        } else {
            tracing::info!("证据链已保存: {} 条证据", ctx.evidence_chain.len());
        }
    }

    Ok(())
}

/// 在 AI 完全不可用时，生成纯代码计算的报告
fn generate_code_only_report(ctx: &PipelineContext) -> String {
    let mut report = String::new();

    report.push_str("## 创新验证报告（数据驱动）\n\n");
    report.push_str(&format!("**新颖性评分：{:.0}/100**\n\n", ctx.novelty_score));

    // 评分解读
    let level = match ctx.novelty_score as u32 {
        80..=100 => "高度新颖 — 未找到高度相似的现有方案",
        60..=79 => "较为新颖 — 存在部分相关方案但有明显差异",
        40..=59 => "中等新颖 — 存在较多相似方案，需要差异化",
        20..=39 => "新颖性较低 — 已有多个类似方案",
        _ => "新颖性很低 — 与现有方案高度重合",
    };
    report.push_str(&format!("**评估等级：** {}\n\n", level));

    // 评分细项
    report.push_str("### 评分构成\n\n");
    report.push_str(&format!(
        "| 指标 | 数值 | 说明 |\n\
         |------|------|------|\n\
         | 最高相似度 | {:.1}% | 与最相似现有技术的匹配度 |\n\
         | Top5 平均相似度 | {:.1}% | 前 5 个最相似方案的平均值 |\n\
         | 矛盾信号加分 | +{:.0} | 检测到 {} 个技术路线矛盾 |\n\
         | 覆盖缺口加分 | +{:.0} | 搜索覆盖度 {:.0}% |\n\n",
        ctx.score_breakdown.max_similarity * 100.0,
        ctx.score_breakdown.avg_top5_similarity * 100.0,
        ctx.score_breakdown.contradiction_bonus,
        ctx.contradictions.len(),
        ctx.score_breakdown.coverage_gap_bonus,
        ctx.diversity_score * 100.0,
    ));

    // 最相关结果
    if !ctx.top_matches.is_empty() {
        report.push_str("### 最相关的现有技术\n\n");
        for m in ctx.top_matches.iter().take(5) {
            report.push_str(&format!(
                "{}. **{}** (相似度 {:.1}%)\n   {}\n\n",
                m.rank,
                m.source_title,
                m.combined_score * 100.0,
                if m.snippet.len() > 100 {
                    format!("{}...", m.snippet.chars().take(100).collect::<String>())
                } else {
                    m.snippet.clone()
                },
            ));
        }
    }

    // 矛盾信号
    if !ctx.contradictions.is_empty() {
        report.push_str("### 矛盾信号（创新机会）\n\n");
        for c in &ctx.contradictions {
            report.push_str(&format!("- **{}**\n  {}\n\n", c.dimension, c.opportunity));
        }
    }

    report
}
