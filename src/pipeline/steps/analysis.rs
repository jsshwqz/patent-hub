//! Step 11-12: AiDeepAnalysis + AiActionPlan
//!
//! 类型：LLM
//!
//! 关键设计：AI 接收的是结构化计算结果，而非让 AI 自己猜测

use crate::ai::AiClient;
use crate::db::Database;
use crate::patent::FeatureCard;
use crate::pipeline::context::{PipelineContext, PipelineProgress};
use anyhow::Result;

/// 执行 Step 11: AI 深度分析（多维推演引擎）
pub async fn deep_analysis(
    ctx: &mut PipelineContext,
    ai: &AiClient,
    progress_tx: &Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
) -> Result<()> {
    // 运行多维深度推演引擎
    let result = super::deep_reasoning::run_deep_reasoning(ctx, ai, progress_tx).await?;

    // 格式化为 Markdown 报告（兼容旧的 ai_analysis 字段）
    ctx.ai_analysis = super::deep_reasoning::format_report(&result);
    ctx.deep_reasoning = result;

    // 更新研发状态机：低新颖性时记录排除路径
    if ctx.novelty_score < 30.0 {
        ctx.research_state
            .excluded_paths
            .push("该方向与现有技术高度重叠，建议调整技术路线".to_string());
    }

    Ok(())
}

/// 执行 Step 11 的降级版本：单次 AI 调用（用于快速模式或推演引擎失败时）
pub async fn deep_analysis_simple(ctx: &mut PipelineContext, ai: &AiClient) -> Result<()> {
    // 构建结构化数据包，让 AI 基于代码计算的结果分析
    let top_matches_summary: String = ctx
        .top_matches
        .iter()
        .take(10)
        .map(|m| {
            format!(
                "- [{}] {} (相似度: {:.1}%)\n  {}\n  来源: {}",
                m.source_type,
                m.source_title,
                m.combined_score * 100.0,
                if m.snippet.len() > 150 {
                    format!("{}...", &m.snippet.chars().take(150).collect::<String>())
                } else {
                    m.snippet.clone()
                },
                m.source_url,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let contradictions_summary = if ctx.contradictions.is_empty() {
        "未检测到明显的技术路线矛盾。".to_string()
    } else {
        ctx.contradictions
            .iter()
            .map(|c| {
                format!(
                    "- 矛盾维度: {}\n  信号强度: {:.0}%\n  机会: {}",
                    c.dimension,
                    c.signal_strength * 100.0,
                    c.opportunity,
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let prompt = format!(
        "## 用户的创意\n\
         **标题：** {title}\n\
         **描述：** {description}\n\
         **技术领域：** {domain}\n\n\
         ## 代码计算结果（已验证，请基于此分析）\n\n\
         **新颖性评分：{score:.0}/100**\n\
         - 最高相似度：{max_sim:.1}%（与最相似的现有技术）\n\
         - Top5 平均相似度：{avg_sim:.1}%\n\
         - 矛盾信号加分：+{contra_bonus:.0}\n\
         - 覆盖缺口加分：+{gap_bonus:.0}\n\
         - 搜索多样性：{diversity:.0}%\n\n\
         ## 最相关的现有技术（按相似度排序）\n\n\
         {matches}\n\n\
         ## 矛盾信号（不同技术路线 = 创新空间）\n\n\
         {contradictions}\n\n\
         请基于以上**代码已计算的结构化数据**进行深度分析，用 Markdown 格式返回：\n\n\
         ### 1. 新颖性解读\n\
         - 解读评分含义，哪些方面是新颖的，哪些已有先例\n\n\
         ### 2. 已有方案分析\n\
         - 对最相关的 3-5 个现有方案进行优缺点分析\n\n\
         ### 3. 差异化机会\n\
         - 基于矛盾信号和覆盖缺口，指出最有前景的创新方向\n\n\
         ### 4. 风险提示\n\
         - 技术壁垒、知识产权风险、市场竞争风险",
        title = ctx.title,
        description = ctx.description,
        domain = ctx.technical_domain,
        score = ctx.novelty_score,
        max_sim = ctx.score_breakdown.max_similarity * 100.0,
        avg_sim = ctx.score_breakdown.avg_top5_similarity * 100.0,
        contra_bonus = ctx.score_breakdown.contradiction_bonus,
        gap_bonus = ctx.score_breakdown.coverage_gap_bonus,
        diversity = ctx.diversity_score * 100.0,
        matches = top_matches_summary,
        contradictions = contradictions_summary,
    );

    match ai.chat(&prompt, None).await {
        Ok(analysis) => ctx.ai_analysis = analysis,
        Err(e) => {
            ctx.ai_analysis = format!(
                "AI 分析暂不可用（{}）。\n\n\
                 基于代码计算的结果：新颖性评分 {:.0}/100，\
                 找到 {} 个相关现有技术，{} 个矛盾信号。\n\
                 请参考上方的量化数据进行判断。",
                e,
                ctx.novelty_score,
                ctx.top_matches.len(),
                ctx.contradictions.len(),
            );
        }
    }

    Ok(())
}

/// 执行 Step 12: AI 生成行动方案
pub async fn action_plan(ctx: &mut PipelineContext, ai: &AiClient) -> Result<()> {
    let prompt = format!(
        "基于以下创意分析结果，生成具体的行动方案：\n\n\
         **创意：** {}\n\
         **新颖性评分：** {:.0}/100\n\
         **技术领域：** {}\n\
         **矛盾信号数量：** {}\n\n\
         **AI 分析摘要（前 500 字）：**\n{}\n\n\
         请给出：\n\n\
         ### 优化建议\n\
         - 3-5 条具体可执行的技术改进方向\n\n\
         ### 推荐行动\n\
         - 短期（1-3个月）应该做什么\n\
         - 中期（3-6个月）应该做什么\n\n\
         ### 潜在合作/资源\n\
         - 可以参考或合作的机构/团队\n\
         - 推荐关注的技术社区或会议",
        ctx.title,
        ctx.novelty_score,
        ctx.technical_domain,
        ctx.contradictions.len(),
        ctx.ai_analysis.chars().take(500).collect::<String>(),
    );

    match ai.chat(&prompt, None).await {
        Ok(plan) => ctx.action_plan = plan,
        Err(e) => {
            ctx.action_plan = format!("行动方案生成暂不可用：{}", e);
        }
    }

    Ok(())
}

/// 从 AI 分析结果和 top_matches 自动提取特征卡片并存入数据库
/// Extract feature cards from AI analysis + ranked matches, persist to DB
pub async fn extract_feature_cards(ctx: &PipelineContext, db: &Database) -> Result<()> {
    let idea_id = &ctx.idea_id;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 从 top_matches 提取：每个高相似度匹配 → 一张特征卡（含 5 维字段）
    // 反转 combined_score 为 novelty_score：相似度越高 → 新颖性越低
    for (i, m) in ctx.top_matches.iter().take(5).enumerate() {
        let novelty = ((1.0 - m.combined_score) * 100.0).clamp(0.0, 100.0);
        let description = if m.snippet.chars().count() > 300 {
            format!("{}...", m.snippet.chars().take(300).collect::<String>())
        } else {
            m.snippet.clone()
        };

        let card = FeatureCard {
            id: format!("{}-fc-{}", idea_id, i + 1),
            idea_id: idea_id.clone(),
            title: format!("[{}] {}", m.source_type, m.source_title),
            description,
            novelty_score: Some(novelty),
            created_at: now.clone(),
            technical_problem: format!("与「{}」相关的技术问题", ctx.title),
            core_structure: m.snippet.chars().take(200).collect::<String>(),
            key_relations: m.tokens.join(", "),
            process_steps: String::new(),
            application_scenarios: ctx.technical_domain.clone(),
        };

        if let Err(e) = db.insert_feature_card(&card) {
            tracing::warn!("特征卡片存储失败: {} — {}", card.title, e);
        }
    }

    // 从矛盾信号提取：每个矛盾 → 一张「创新机会」特征卡（含 5 维字段）
    for (i, c) in ctx.contradictions.iter().enumerate() {
        let card = FeatureCard {
            id: format!("{}-fc-opp-{}", idea_id, i + 1),
            idea_id: idea_id.clone(),
            title: format!("创新机会: {}", c.dimension),
            description: c.opportunity.clone(),
            novelty_score: Some(c.signal_strength * 100.0),
            created_at: now.clone(),
            technical_problem: format!("{}与{}在{}维度的矛盾", c.source_a, c.source_b, c.dimension),
            core_structure: c.opportunity.clone(),
            key_relations: format!("{} ↔ {}", c.source_a, c.source_b),
            process_steps: String::new(),
            application_scenarios: ctx.technical_domain.clone(),
        };

        if let Err(e) = db.insert_feature_card(&card) {
            tracing::warn!("创新机会卡片存储失败: {} — {}", card.title, e);
        }
    }

    let total = ctx.top_matches.len().min(5) + ctx.contradictions.len();
    tracing::info!("已自动提取 {} 张特征卡片 (idea: {})", total, idea_id);
    Ok(())
}

/// AI 驱动的 5 维特征提取 — 在 deep_analysis 完成后调用
/// 向 AI 发送结构化 prompt，解析返回的 JSON 数组，创建带 5 维字段的 FeatureCard
pub async fn extract_feature_cards_ai(
    ctx: &PipelineContext,
    ai: &AiClient,
    db: &Database,
) -> Result<()> {
    let prompt = format!(
        "从以下创意描述和分析结果中提取技术特征卡片。\n\
         请输出 JSON 数组，每个元素包含以下字段：\n\
         - title: 特征标题\n\
         - technical_problem: 解决什么技术问题\n\
         - core_structure: 核心技术结构/方案\n\
         - key_relations: 关键技术关系/连接\n\
         - process_steps: 工艺/实施步骤\n\
         - application_scenarios: 应用场景/领域\n\n\
         创意标题：{}\n创意描述：{}\n\n分析结果：{}\n\n\
         请直接输出 JSON 数组，不要包含 markdown 标记。",
        ctx.title,
        ctx.description,
        ctx.ai_analysis.chars().take(2000).collect::<String>()
    );

    match ai.chat(&prompt, None).await {
        Ok(response) => {
            let cleaned = response
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            if let Ok(cards) = serde_json::from_str::<Vec<serde_json::Value>>(cleaned) {
                let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                for card_json in cards.iter().take(5) {
                    let card = FeatureCard {
                        id: uuid::Uuid::new_v4().to_string(),
                        idea_id: ctx.idea_id.clone(),
                        title: card_json["title"]
                            .as_str()
                            .unwrap_or("AI提取特征")
                            .to_string(),
                        description: String::new(),
                        novelty_score: None,
                        created_at: now.clone(),
                        technical_problem: card_json["technical_problem"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        core_structure: card_json["core_structure"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        key_relations: card_json["key_relations"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        process_steps: card_json["process_steps"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        application_scenarios: card_json["application_scenarios"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                    };
                    if let Err(e) = db.insert_feature_card(&card) {
                        tracing::warn!("AI 特征卡片存储失败: {}", e);
                    }
                }
                tracing::info!(
                    "AI 自动提取 {} 张 5 维特征卡片 (idea: {})",
                    cards.len().min(5),
                    ctx.idea_id
                );
            } else {
                tracing::warn!("AI 特征提取返回非法 JSON，跳过");
            }
        }
        Err(e) => {
            tracing::warn!("AI 特征卡片提取失败: {}", e);
        }
    }

    Ok(())
}
