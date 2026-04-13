//! 多维深度推演引擎 / Multi-Dimensional Deep Reasoning Engine
//!
//! 6 个思维维度 + 1 轮跨维度合成，每轮用不同的认知框架约束 AI，
//! 迫使它产生真正超越「排列组合」的深度洞察。

use crate::ai::AiClient;
use crate::pipeline::context::{
    DeepReasoningResult, DimensionInsight, PipelineContext, PipelineProgress, StepStatus,
};
use crate::pipeline::state::PipelineStep;
use anyhow::Result;

/// 维度定义
struct Dimension {
    key: &'static str,
    label: &'static str,
    system_prompt: &'static str,
    temperature: f32,
}

const DIMENSIONS: &[Dimension] = &[
    // ── 维度 1: 科学推导 ──
    Dimension {
        key: "scientific",
        label: "科学推导",
        system_prompt:
            "你是一位理论物理学家兼材料科学家。你只从基本定律出发推理，不引用任何现有产品或方案。\n\n\
             你的思维方式：\n\
             1. 识别支配这个问题的基本物理/化学/数学方程\n\
             2. 从这些方程推导出理论极限（不是工程极限，是物理定律允许的极限）\n\
             3. 计算当前实践距离理论极限还有多远（这个差距就是创新空间）\n\
             4. 指出在这个差距中，哪些「自由能」可以被开发\n\n\
             约束：你必须给出至少一个定量推导或公式。不要泛泛而谈。",
        temperature: 0.3,
    },
    // ── 维度 2: 辩证批判 ──
    Dimension {
        key: "dialectical",
        label: "辩证批判",
        system_prompt:
            "你是一位辩证法大师。你的任务是攻击问题本身，而不是回答问题。\n\n\
             你的思维方式：\n\
             1. 列出这个问题陈述中包含的所有隐含假设（至少 3 个）\n\
             2. 对每个假设：如果它的反面成立呢？会打开什么新空间？\n\
             3. 找出现有方案之间的矛盾（不是缺点，是相互矛盾的技术路线）\n\
             4. 对每个矛盾：提出一个不做妥协的「合题」——同时满足矛盾双方的路径\n\n\
             约束：你必须产出至少 3 个明确的矛盾及其「合题」解决方案。不要只列缺点。",
        temperature: 0.8,
    },
    // ── 维度 3: 知识审计 ──
    Dimension {
        key: "epistemological",
        label: "知识审计",
        system_prompt:
            "你是一位认识论学者。你不分析技术，你分析「我们以为自己知道的东西」。\n\n\
             你的思维方式：\n\
             1. 列出这个领域里所有人认为「当然如此」的 5 个核心「事实」\n\
             2. 对每个「事实」追问：证据是什么？最后一次被验证是什么时候？在什么条件下验证的？\n\
             3. 哪些「事实」其实是历史遗留的假设，已经不适用于当前技术条件？\n\
             4. 如果某个「事实」被推翻，会打开什么全新的解决方案空间？\n\n\
             约束：你必须找出至少 1 个被广泛接受但实际上是未经验证的假设。",
        temperature: 0.5,
    },
    // ── 维度 4: 本质还原 ──
    Dimension {
        key: "phenomenological",
        label: "本质还原",
        system_prompt:
            "你是一位现象学思考者。你要剥掉所有技术术语和已知方案，回到最原始的需求。\n\n\
             你的思维方式：\n\
             1. 忘掉目前是怎么做的。用一句话描述最原始的需求（像对 5 岁小孩解释）\n\
             2. 从这个原始需求出发，假装你从未听说过任何现有方案，你会发明什么？\n\
             3. 列出 3-5 个「天真的」方案（不管目前是不是这么做的）\n\
             4. 对每个天真方案检验：它真的不可能吗？还是只是「不是这么做的」？\n\
             5. 找出那些「不是不可能，只是没人做过」的方案\n\n\
             约束：你必须产出至少 1 个看似天真但物理上可行的新方案。",
        temperature: 0.7,
    },
    // ── 维度 5: 减法思维 ──
    Dimension {
        key: "eastern",
        label: "减法思维",
        system_prompt:
            "你用「无为」和「减法」思维。最好的方案不是解决问题，而是消除产生问题的条件。\n\n\
             你的思维方式：\n\
             1. 这个问题为什么存在？什么上游条件产生了它？\n\
             2. 如果消除上游条件，问题本身就不需要解决了——怎么消除？\n\
             3. 当前方案中有多少复杂性只是为了补偿早期的错误决策？\n\
             4. 如果把当前方案的组件/步骤/参数减少到 1/10，会怎样？\n\
             5. 什么东西可以直接删掉，系统反而变得更好？\n\n\
             约束：你必须识别出至少 1 个可以完全移除的组件/步骤，并说明移除后为什么更好。",
        temperature: 0.6,
    },
    // ── 维度 6: 跨域映射 ──
    Dimension {
        key: "topological",
        label: "跨域映射",
        system_prompt:
            "你是一位数学家，你看到的不是具体事物，而是抽象结构和模式。\n\n\
             你的思维方式：\n\
             1. 抽象出这个问题的数学结构（它是什么类型的问题？优化？流动？扩散？拓扑？博弈？）\n\
             2. 在完全不同的领域中找到具有相同数学结构的问题（至少 2 个领域）\n\
             3. 那些领域是怎么解决它们的版本的？\n\
             4. 哪些解法可以直接迁移过来？需要做什么适配？\n\
             5. 这种跨域迁移能带来什么当前领域没有的突破？\n\n\
             约束：你必须给出至少 2 个跨域类比，每个都要指明具体的可迁移技术。",
        temperature: 0.7,
    },
];

/// 合成轮的 system prompt
const SYNTHESIS_SYSTEM: &str = "你是一位跨学科整合大师。你收到了 6 位不同思维模式专家的分析。\n\
     你的任务不是总结他们说了什么，而是找到他们之间的化学反应——\n\
     单个维度看不到，但两个或多个维度交叉时才会浮现的洞察。\n\n\
     你需要输出：\n\n\
     ### 收敛信号\n\
     多个维度独立指向的相同方向（这些是最可信的方向）\n\n\
     ### 突破性方向（Top 3）\n\
     跨维度组合产生的全新方案——这些方案在任何单一维度中都不会出现。\n\
     对每个方向：说明它来自哪些维度的交叉，为什么可行，具体怎么做。\n\n\
     ### 盲区\n\
     6 个维度都没有覆盖到的领域或可能性（可能是最大的机会）\n\n\
     约束：Top 3 方向必须是**从未在现有文献中出现过**的新方案，不是已有方案的改良。";

/// 构建用户上下文（所有维度共用的输入数据）
fn build_user_context(ctx: &PipelineContext) -> String {
    let top_matches: String = ctx
        .top_matches
        .iter()
        .take(8)
        .map(|m| {
            format!(
                "- [{}] {} (相似度: {:.0}%): {}",
                m.source_type,
                m.source_title,
                m.combined_score * 100.0,
                if m.snippet.len() > 120 {
                    format!("{}...", &m.snippet.chars().take(120).collect::<String>())
                } else {
                    m.snippet.clone()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let contradictions: String = if ctx.contradictions.is_empty() {
        "无".to_string()
    } else {
        ctx.contradictions
            .iter()
            .map(|c| {
                format!(
                    "- {}: {} (强度 {:.0}%)",
                    c.dimension,
                    c.opportunity,
                    c.signal_strength * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "## 待分析的创意\n\
         **标题：** {title}\n\
         **描述：** {desc}\n\
         **技术领域：** {domain}\n\
         **新颖性评分：** {score:.0}/100\n\n\
         ## 已发现的相关现有技术\n{matches}\n\n\
         ## 已检测到的技术路线矛盾\n{contras}\n\n\
         基于以上信息，按你的思维框架进行分析。",
        title = ctx.title,
        desc = ctx.description,
        domain = ctx.technical_domain,
        score = ctx.novelty_score,
        matches = top_matches,
        contras = contradictions,
    )
}

/// 从 AI 输出中提取核心洞察（第一段非空内容）
fn extract_key_insight(text: &str) -> String {
    text.lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("---")
                && trimmed.len() > 10
        })
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// 从合成结果中提取 novel_directions 和 blind_spots
fn parse_synthesis(text: &str) -> (Vec<String>, Vec<String>) {
    let mut novel = Vec::new();
    let mut blinds = Vec::new();
    let mut section = "";

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains("突破性方向") || trimmed.contains("Top 3") {
            section = "novel";
            continue;
        }
        if trimmed.contains("盲区") || trimmed.contains("盲点") {
            section = "blind";
            continue;
        }
        if trimmed.contains("收敛信号") {
            section = "converge";
            continue;
        }
        if trimmed.starts_with("###") {
            section = "";
            continue;
        }

        let content = trimmed.trim_start_matches(|c: char| {
            c == '-' || c == '*' || c.is_ascii_digit() || c == '.' || c == ' '
        });
        if content.len() > 10 {
            match section {
                "novel" => novel.push(content.to_string()),
                "blind" => blinds.push(content.to_string()),
                _ => {}
            }
        }
    }

    (novel, blinds)
}

/// 执行多维深度推演
pub async fn run_deep_reasoning(
    ctx: &mut PipelineContext,
    ai: &AiClient,
    progress_tx: &Option<tokio::sync::broadcast::Sender<PipelineProgress>>,
) -> Result<DeepReasoningResult> {
    let user_context = build_user_context(ctx);
    let total_rounds = DIMENSIONS.len() + 1; // 6 维度 + 1 合成
    let mut result = DeepReasoningResult::default();

    // ── 6 轮维度推演 ──
    for (i, dim) in DIMENSIONS.iter().enumerate() {
        tracing::info!(
            "Deep reasoning: dimension {}/{} [{}]",
            i + 1,
            total_rounds,
            dim.key
        );

        // 发送子进度
        if let Some(tx) = progress_tx {
            let _ = tx.send(PipelineProgress {
                step: PipelineStep::AiDeepAnalysis,
                step_index: PipelineStep::AiDeepAnalysis.index(),
                total_steps: PipelineStep::TOTAL_STEPS,
                status: StepStatus::Running,
                message: format!("多维推演：{} ({}/{})", dim.label, i + 1, total_rounds),
                sub_step: Some(dim.key.to_string()),
                sub_progress: Some(i as f64 / total_rounds as f64),
            });
        }

        let reasoning = match ai
            .chat_with_system(dim.system_prompt, &user_context, dim.temperature)
            .await
        {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!("维度 [{}] 推演失败: {}", dim.key, e);
                format!("（此维度推演未完成：{}）", e)
            }
        };

        let key_insight = extract_key_insight(&reasoning);

        result.dimensions.push(DimensionInsight {
            dimension: dim.key.to_string(),
            label: dim.label.to_string(),
            reasoning,
            key_insight,
        });
    }

    // ── 第 7 轮：跨维度合成 ──
    tracing::info!(
        "Deep reasoning: synthesis round ({}/{})",
        total_rounds,
        total_rounds
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(PipelineProgress {
            step: PipelineStep::AiDeepAnalysis,
            step_index: PipelineStep::AiDeepAnalysis.index(),
            total_steps: PipelineStep::TOTAL_STEPS,
            status: StepStatus::Running,
            message: format!("跨维度合成 ({}/{})", total_rounds, total_rounds),
            sub_step: Some("synthesis".to_string()),
            sub_progress: Some(6.0 / total_rounds as f64),
        });
    }

    // 构建合成输入：每个维度的核心洞察
    let synthesis_input = result
        .dimensions
        .iter()
        .map(|d| {
            format!(
                "## [{label}] 维度分析\n\n{reasoning}\n",
                label = d.label,
                reasoning = d.reasoning,
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n\n");

    let synthesis_prompt = format!(
        "以下是 6 位不同思维模式专家对同一个创意的分析：\n\n\
         **创意：** {title}\n\
         **描述：** {desc}\n\n\
         ---\n\n\
         {inputs}\n\n\
         ---\n\n\
         请进行跨维度合成分析。",
        title = ctx.title,
        desc = ctx.description,
        inputs = synthesis_input,
    );

    match ai
        .chat_with_system(SYNTHESIS_SYSTEM, &synthesis_prompt, 0.5)
        .await
    {
        Ok(text) => {
            let (novel, blinds) = parse_synthesis(&text);
            result.synthesis = text;
            result.novel_directions = novel;
            result.blind_spots = blinds;
        }
        Err(e) => {
            tracing::warn!("合成推演失败: {}", e);
            result.synthesis = format!("（跨维度合成未完成：{}）", e);
        }
    }

    Ok(result)
}

/// 将推演结果格式化为 Markdown 报告（兼容现有 ai_analysis 字段）
pub fn format_report(result: &DeepReasoningResult) -> String {
    let mut report = String::from("# 多维深度推演报告\n\n");

    // 合成结论放最前面
    if !result.synthesis.is_empty() && !result.synthesis.starts_with('（') {
        report.push_str("## 🔮 跨维度合成\n\n");
        report.push_str(&result.synthesis);
        report.push_str("\n\n---\n\n");
    }

    // 各维度详细分析
    let icons = ["🔬", "⚡", "🔍", "🌊", "☯️", "🔗"];
    for (i, dim) in result.dimensions.iter().enumerate() {
        let icon = icons.get(i).unwrap_or(&"📌");
        report.push_str(&format!(
            "## {} {} ({})\n\n{}\n\n---\n\n",
            icon, dim.label, dim.dimension, dim.reasoning
        ));
    }

    report
}
