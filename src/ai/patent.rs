//! 专利分析方法 / Patent analysis methods

use super::client::{safe_truncate, AiClient, Message};
use anyhow::Result;

impl AiClient {
    pub async fn summarize_patent(
        &self,
        patent_title: &str,
        abstract_text: &str,
        claims: &str,
    ) -> Result<String> {
        let prompt = format!(
            "请对以下专利进行全面分析摘要：\n\n\
             标题：{patent_title}\n\n\
             摘要：{abstract_text}\n\n\
             权利要求（前部分）：{claims_preview}\n\n\
             请从以下几个方面分析：\n\
             1. 技术领域\n\
             2. 核心技术方案\n\
             3. 创新点\n\
             4. 应用场景\n\
             5. 关键权利要求解读",
            claims_preview = safe_truncate(claims, 2000)
        );
        self.chat(&prompt, None).await
    }

    /// Analyze patent claims: identify independent vs dependent, extract scope elements.
    pub async fn analyze_claims(&self, patent_title: &str, claims: &str) -> Result<String> {
        let prompt = format!(
            "请对以下专利的权利要求进行深度分析：\n\n\
             专利标题：{patent_title}\n\n\
             权利要求全文：\n{claims_text}\n\n\
             请按以下格式分析（使用 Markdown 表格）：\n\n\
             ### 1. 权利要求结构总览\n\
             列出每条权利要求的编号、类型（独立/从属）、所从属的权利要求号\n\n\
             ### 2. 独立权利要求分析\n\
             对每条独立权利要求：\n\
             - 保护范围要素（技术特征列表）\n\
             - 保护范围宽度评估（宽/中/窄）\n\
             - 可能的规避设计方向\n\n\
             ### 3. 从属权利要求层级\n\
             用缩进或树形结构展示权利要求之间的从属关系\n\n\
             ### 4. 关键技术特征\n\
             提取最核心的限定性技术特征（决定保护范围的关键要素）\n\n\
             ### 5. 保护强度评估\n\
             综合评估该专利权利要求的保护强度（强/中/弱），并说明原因",
            claims_text = safe_truncate(claims, 4000)
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深专利代理人和知识产权律师。你擅长解读专利权利要求书，\
                         分析保护范围，识别关键技术特征。请用专业、严谨的语言分析。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Assess infringement risk: compare a product/tech description against multiple patents.
    pub async fn assess_infringement(
        &self,
        product_description: &str,
        patents_info: &str,
    ) -> Result<String> {
        let prompt = format!(
            "## 待评估的产品/技术方案\n{product}\n\n\
             ## 对比专利列表\n{patents}\n\n\
             请对每个专利逐一进行侵权风险评估，按以下格式输出（使用 Markdown 表格）：\n\n\
             ### 侵权风险评估矩阵\n\
             | 专利号 | 风险等级 | 关键风险点 | 规避建议 |\n\
             |--------|----------|------------|----------|\n\n\
             风险等级说明：\n\
             - **高风险**: 产品技术方案与专利权利要求高度重合\n\
             - **中风险**: 部分技术特征重合，需进一步分析\n\
             - **低风险**: 技术方案存在明显差异\n\
             - **无风险**: 不在专利保护范围内\n\n\
             ### 详细分析\n\
             对每个高/中风险专利，详细说明：\n\
             1. 哪些技术特征与专利权利要求对应\n\
             2. 字面侵权还是等同侵权的可能性\n\
             3. 具体的规避设计建议\n\n\
             ### 综合建议\n\
             整体风险评估和应对策略建议",
            product = safe_truncate(product_description, 2000),
            patents = safe_truncate(patents_info, 4000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深知识产权律师和专利侵权分析专家。你擅长评估产品的专利侵权风险，\
                         对比技术方案与专利权利要求的对应关系。请客观、专业地分析，并提供可操作的建议。".into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Compare multiple patents across multiple dimensions.
    pub async fn compare_multiple(&self, patents_info: &str) -> Result<String> {
        let prompt = format!(
            "请对以下多个专利进行多维度对比分析：\n\n{patents}\n\n\
             请按以下格式输出（使用 Markdown 表格）：\n\n\
             ### 1. 基本信息对比\n\
             | 维度 | 专利1 | 专利2 | ... |\n\
             |------|-------|-------|-----|\n\
             | 技术领域 | | | |\n\
             | 核心问题 | | | |\n\
             | 申请人 | | | |\n\n\
             ### 2. 技术方案对比\n\
             | 维度 | 专利1 | 专利2 | ... |\n\
             |------|-------|-------|-----|\n\
             | 核心方案 | | | |\n\
             | 创新点 | | | |\n\
             | 技术路线 | | | |\n\n\
             ### 3. 优缺点分析\n\
             | 专利 | 优点 | 缺点 | 应用场景 |\n\n\
             ### 4. 综合评价\n\
             - 技术演进趋势\n\
             - 最具创新性的方案\n\
             - 互补性分析",
            patents = safe_truncate(patents_info, 6000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位专利技术分析专家，擅长对比分析多个专利的技术方案，\
                         识别技术演进趋势和创新差异。请用结构化的表格形式呈现分析结果。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.5).await
    }

    /// Inventiveness (创造性) analysis: compare my patent against reference documents
    /// using the three-step method (三步法) per Chinese patent examination guidelines.
    pub async fn inventiveness_analysis(
        &self,
        my_patent_info: &str,
        references_info: &str,
    ) -> Result<String> {
        let prompt = format!(
            "## 我的专利\n{my_patent}\n\n## 对比文件\n{references}\n\n\
             请按照中国专利审查指南的「三步法」，对我的专利的每条独立权利要求逐一进行创造性分析：\n\n\
             ### 分析要求\n\
             对每条独立权利要求，依次完成：\n\
             1. **确定最接近的现有技术**：从对比文件中选出最接近的一篇，说明理由\n\
             2. **确定区别技术特征**：列出我的权利要求与最接近现有技术之间的全部区别技术特征\n\
             3. **判断是否显而易见**：\n\
                - 该区别技术特征解决了什么技术问题（重新确定的技术问题）\n\
                - 对比文件中是否给出了将该区别特征应用于最接近现有技术的技术启示\n\
                - 综合判断：显而易见 / 非显而易见\n\
             4. **技术效果分析**：区别技术特征带来的技术效果（预料不到的效果加分）\n\
             5. **答辩建议**：\n\
                - 如果创造性成立：给出答辩要点\n\
                - 如果创造性不足：给出修改建议（如合并从属权利要求）\n\n\
             ### 输出格式\n\
             请使用 Markdown，对每条独立权利要求单独成节，使用表格汇总区别特征。\n\
             最后给出「综合答辩策略建议」。",
            my_patent = safe_truncate(my_patent_info, 5000),
            references = safe_truncate(references_info, 5000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深中国专利代理师（执业15年+），精通中国专利法及审查指南中的创造性判断标准（三步法）。\
                         你擅长答复审查意见通知书，尤其是创造性驳回（A22.3）的答辩。\
                         请用严谨、专业的语言分析，结论要有理有据。".into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Office action response: deep analysis for responding to first examination opinion
    pub async fn office_action_response(
        &self,
        my_patent_info: &str,
        office_action: &str,
        references_info: &str,
    ) -> Result<String> {
        let prompt = format!(
            "## 我的专利（权利要求书+说明书）\n{my_patent}\n\n\
             ## 审查意见通知书\n{oa}\n\n\
             ## 对比文献\n{refs}\n\n\
             请基于以上材料，生成完整的一审答复方案：\n\n\
             ## 第一部分：审查意见解析\n\
             - 逐条列出审查员对每项权利要求的驳回理由\n\
             - 识别审查员引用的对比文献和具体段落\n\n\
             ## 第二部分：逐项权利要求对比分析\n\
             对每项权利要求：\n\
             - 技术特征分解（逐个技术特征列出）\n\
             - vs 各对比文献公开的内容\n\
             - 真正的区别特征（审查员可能遗漏的）\n\n\
             ## 第三部分：反驳要点\n\
             1. 最接近现有技术选择是否合理？\n\
             2. 审查员的区别特征认定是否完整？有无遗漏？\n\
             3. 技术启示论证是否成立？（特别关注不同技术领域的组合动机）\n\
             4. 各特征组合后的协同技术效果是否被忽视？\n\
             5. 对比文献之间是否存在技术矛盾（不宜组合）？\n\n\
             ## 第四部分：答复策略建议\n\
             1. 修改权利要求的具体方案（增加哪些限定特征）\n\
             2. 意见陈述的核心论点\n\
             3. 建议的修改后权利要求书\n\n\
             ## 第五部分：意见陈述书草稿\n\
             生成可直接提交的意见陈述书，包括：\n\
             - 对审查意见的回应\n\
             - 修改说明\n\
             - 创造性论述",
            my_patent = safe_truncate(my_patent_info, 15000),
            oa = safe_truncate(office_action, 10000),
            refs = safe_truncate(references_info, 15000),
        );

        let messages = vec![
            Message {
                role: "system".into(),
                content: "你是一位资深中国专利代理师（执业20年+），精通中国专利法及审查指南。\
                         你擅长应对审查意见通知书，尤其是创造性驳回（A22.3）的答辩。\
                         你的答复策略注重：\n\
                         1. 精确分解技术特征，找出审查员遗漏的区别点\n\
                         2. 质疑对比文献组合的合理性（技术启示、技术领域差异、反向教导）\n\
                         3. 强调组合后的协同技术效果\n\
                         4. 提供可操作的权利要求修改方案\n\
                         请用严谨、专业的语言，结论要有理有据，可直接用于提交。"
                    .into(),
            },
            Message {
                role: "user".into(),
                content: prompt,
            },
        ];

        self.send_chat(messages, 0.3).await
    }

    /// Batch summarize multiple patents concurrently.
    pub async fn batch_summarize(
        &self,
        patents: &[(String, String, String)],
    ) -> Vec<(String, Result<String>)> {
        let mut results = Vec::new();
        for (id, title, abstract_text) in patents {
            let result = self
                .chat(
                    &format!(
                        "请用2-3句话简要总结这个专利的核心技术方案：\n标题：{}\n摘要：{}",
                        title,
                        safe_truncate(abstract_text, 500)
                    ),
                    None,
                )
                .await;
            results.push((id.clone(), result));
        }
        results
    }
}
