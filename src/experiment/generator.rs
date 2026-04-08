//! AI 驱动的验证脚本生成器 / AI-driven verification script generator

use crate::ai::AiClient;
use crate::experiment::types::ExperimentSpec;
use crate::pipeline::context::PipelineContext;
use anyhow::Result;

/// 根据创意描述和分析结果，AI 生成验证脚本
pub async fn generate_experiment(
    ctx: &PipelineContext,
    ai: &AiClient,
) -> Result<ExperimentSpec> {
    let prompt = format!(
        "你是一名研发工程师。根据以下创意和分析结果，生成一个 Python 验证脚本。\n\
         \n\
         脚本要求：\n\
         1. 用 Python 3 编写，只使用标准库\n\
         2. 验证创意中描述的核心技术可行性\n\
         3. 输出结构化指标，格式为 JSON 行：{{\"metric_name\": value}}\n\
         4. 脚本末尾打印 EXPERIMENT_DONE 标记\n\
         5. 不要使用任何网络请求\n\
         \n\
         创意标题：{}\n\
         创意描述：{}\n\
         技术领域：{}\n\
         新颖性评分：{:.1}/100\n\
         AI 分析：{}\n\
         \n\
         请只输出 Python 代码，不要包含 markdown 标记或说明文字。",
        ctx.title,
        ctx.description,
        ctx.technical_domain,
        ctx.novelty_score,
        &ctx.ai_analysis.chars().take(500).collect::<String>(),
    );

    let response = ai.chat(&prompt, None).await?;

    // 清理可能的 markdown 代码块标记
    let script = response
        .trim()
        .trim_start_matches("```python")
        .trim_start_matches("```py")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();

    Ok(ExperimentSpec {
        title: format!("验证: {}", ctx.title),
        language: "python".to_string(),
        script_content: script,
        hypothesis: ctx.research_state.current_hypothesis.clone(),
        timeout_secs: 30,
    })
}
