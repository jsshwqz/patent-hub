# DeepScientist 借鉴分析 — InnoForge 架构演进参考

> 分析日期：2026-04-08
> 对比版本：InnoForge v0.5.1 (Rust, 12,800 行) vs DeepScientist v1.5.17 (Python+Node.js)
> 对比仓库：[ResearAI/DeepScientist](https://github.com/ResearAI/DeepScientist) (ICLR 2026 Top 10, 1.8k stars)
> 参考输入：`INTEGRATION_SPEC.md`（深度集成技术规范）
> 协作分析：Claude + Gemini 交叉审查，Aion-Forge 多引擎研究

---

## 1. 项目定位对比

| 维度 | InnoForge (创研台) | DeepScientist |
|------|-------------------|---------------|
| **定位** | 专利/创新验证平台 | 自主研究工作室 |
| **核心流程** | 13 步创新验证管道 | 持续实验循环 (baseline→hypothesis→experiment→paper) |
| **AI 角色** | 辅助分析（代码编排 LLM） | 自主执行（AI 驱动全流程） |
| **数据持久化** | SQLite + 管道快照 | Git 仓库 (one quest = one repo) |
| **语言** | Rust + Vanilla JS | Python + React |
| **部署** | 零依赖单体二进制 | npm 全局安装 + daemon |
| **目标用户** | 专利工程师、研发经理 | 研究生、实验室 |

**核心差异**：DeepScientist 让 AI 全自主跑实验写论文；InnoForge 让 AI 辅助验证创新点是否成立。两者互补而非竞争。

**DeepScientist 七大契约**（AGENTS.md）：
1. One Quest, One Repository — 每个研究课题一个 Git 仓库
2. Python runtime + npm launcher — 双语言混合架构
3. MCP 三命名空间 — `memory`, `artifact`, `bash_exec`
4. Prompt 定义工作流 — Skill 提供执行行为
5. Registry 注册模式 — 扩展点统一用 register/get/list API
6. 客户端平权 — Web UI 和 TUI 消费相同 daemon API
7. QQ 集成是核心产品形态 — 不是 hack

**DeepScientist 八阶段 Skill**：`scout` → `baseline` → `idea` → `experiment` → `analysis-campaign` → `write` → `finalize` → `decision`

---

## 2. 值得借鉴的理念

DeepScientist 的核心哲学：**「研究是持续积累的过程，不是一次性查询」**

InnoForge 当前是「跑一次管道 → 出报告 → 结束」，缺乏迭代和积累机制。

### 2.1 Findings Memory（发现记忆系统）⭐⭐⭐

**DeepScientist 做法**：失败路径不丢弃，总结保存，供后续实验复用。

**InnoForge 现状**：`ResearchState.excluded_paths` 只是内存中的临时字符串列表，Pipeline 结束即消失。

**改进方案**：
- 在 `evidence_chain` 表增加 `finding_type` 字段（`discovery` / `dead_end` / `insight`）
- 新 Idea 提交时，自动检索历史排除路径中的相似记录
- 跨 Idea 的 Findings 检索 API：`GET /api/findings?q=关键词`

**涉及文件**：`src/db/evidence.rs`, `src/pipeline/context.rs`, `src/routes/idea.rs`

### 2.2 持续迭代分析 ⭐⭐⭐

**DeepScientist 做法**：每轮实验结果自动喂入下一轮假设生成，形成闭环。

**InnoForge 现状**：Pipeline 跑一次就结束，用户需手动重新提交。

**改进方案**：
- 新增 `POST /api/idea/:id/iterate`：基于上一次分析 + 用户反馈，自动调整关键词重跑 Pipeline
- `ResearchState.open_questions` 作为下一轮输入种子
- 迭代计数器：`iteration_count` 字段，展示方案经过几轮验证

**涉及文件**：`src/pipeline/runner.rs`, `src/routes/idea.rs`

### 2.3 Research Map 可视化 ⭐⭐⭐

**DeepScientist 做法**：Canvas 可视化研究分支、已完成路径、死胡同。

**InnoForge 现状**：只有线性 13 步进度条。

**改进方案**：
- 在报告页增加「创新探索地图」：以 Idea 为中心，展示 prior_art_clusters → contradictions → novelty 关系
- 用 Mermaid.js（已有 Markdown 渲染能力）或 D3.js 渲染证据链网络图
- 红色节点 = 反对证据，绿色节点 = 支持证据，灰色 = 排除路径

**涉及文件**：`templates/` (新增), `src/routes/idea.rs` (report.html 增强)

### 2.4 假设演化链 ⭐⭐

**DeepScientist 做法**：贝叶斯优化驱动假设选择。

**InnoForge 现状**：`current_hypothesis` 在 ParseInput 设置一次，后续不更新。

**改进方案**：
- 每步结束后根据新证据更新假设
- `hypothesis_history: Vec<(step, old_hypothesis, new_hypothesis, reason)>`
- Step 11 (DeepReasoning) 的 `novel_directions` 自动转为新假设候选

**涉及文件**：`src/pipeline/context.rs`, `src/pipeline/steps/deep_reasoning.rs`

### 2.5 人机协作 — 中途重定向 ⭐⭐

**DeepScientist 做法**："Human takeover anytime"，研究者可随时暂停、编辑、重定向。

**InnoForge 现状**：已有 resume 能力，但只能从断点续跑，无法修改方向。

**改进方案**：
- `POST /api/idea/:id/redirect`：注入新约束（如追加排除关键词、修改技术领域）
- Pipeline resume 时检查 override 参数

**涉及文件**：`src/pipeline/runner.rs`

### 2.6 Webhook 通知 ⭐⭐

**DeepScientist 做法**：WeChat/Telegram/飞书多通道推送。

**InnoForge 现状**：仅 Web SSE。

**改进方案**：
- `POST /api/settings/webhook` 配置回调 URL
- Pipeline 完成时 POST JSON 到用户 URL（标题、评分、结论摘要）

**涉及文件**：`src/routes/settings.rs`, `src/pipeline/runner.rs`

---

## 3. Gemini 集成规范（INTEGRATION_SPEC.md）评审

Gemini 提出了 4 个模块（A/B/C/D），以下是务实评估：

### 模块 A：自主实验验证引擎 — 采纳 20%

| Gemini 方案 | 问题 | 务实替代 |
|------------|------|---------|
| 自动生成验证脚本 + Docker 沙箱运行 | 过重，InnoForge 用户不跑实验 | AI **模拟论证**：基于文献数据生成对比实验表格 |
| 指标捕获（吞吐量/延迟） | 依赖真实环境 | 用 AI 从已有 paper 中提取相关指标数据 |
| 报告注入到 docx | 可保留 | 保留：将模拟数据填入交底书模板 |

**结论**：不做真实实验执行，改为 AI 驱动的「虚拟对比论证」。

### 模块 B：Git 驱动版本管理 — 采纳 40%

| Gemini 方案 | 问题 | 务实替代 |
|------------|------|---------|
| 每个 Quest 创建 Git 子仓库 | 用户不懂 Git | SQLite `idea_versions` 表 |
| `failed-path` 分支 | 概念过重 | Findings Memory（见 2.1） |
| `design-around-01` 分支 | 好思路 | `idea_variants` 表：关联同一原始 idea 的规避方案 |

**结论**：用 DB 版本链替代 Git 子仓库，UI 展示为「方案演进时间线」。

### 模块 C：权利要求特征拆解 — 采纳 90% ⭐

| Gemini 方案 | 与现有架构的契合度 | 改进点 |
|------------|-------------------|--------|
| Feature Matrix 拆解 | 完美对接 FeatureCard 5 维体系 | 已有基础，需增加「必要技术特征」提取 |
| Gap 分析 | 完美对接 `classify_diff` | 已做 structure/method/parameter 分类 |
| 授权率预测 | 需新增 | 基于 Gap 类型 + 数量 → 预测授权概率 |

**这是最应该优先做的模块**，因为它与 T1-T3 刚完成的 FeatureCard 增强天然衔接。

**实施路径**：
1. AI 提取对比文件的「必要技术特征」列表 → 存为 FeatureCard
2. 用户 Idea 的 FeatureCard vs 对比文件的 FeatureCard → `classify_diff`
3. 汇总 Gap 数量和类型 → 输出授权率预测
4. 新 API：`GET /api/idea/:id/patentability` 返回授权率 + 理由

### 模块 D：动态技术空间地图 — 采纳 60%

| Gemini 方案 | 问题 | 务实替代 |
|------------|------|---------|
| 向量空间投影 | 当前无 embedding 模型 | 用 `prior_art_clusters` 数据做气泡图 |
| 热力图 | 需要大量专利向量 | 聚类密度图：红区/绿区标注 |
| ECharts/React Flow | 前端是 Vanilla JS | 用 ECharts CDN 或 Mermaid.js |

**结论**：先用现有聚类数据做简化版；embedding 向量化作为后期增强。

---

## 4. 综合优先级排序

| 优先级 | 方向 | 来源 | 工作量 | 价值 |
|--------|------|------|--------|------|
| **P0** | 权利要求特征拆解 + 授权率预测 | Gemini 模块 C | M | 直接对接已有 FeatureCard，核心竞争力 |
| **P0** | 持续迭代 iterate API | DeepScientist | M | 核心体验提升 |
| **P0** | Findings Memory 跨 Idea 复用 | DeepScientist | M | 研究效率飞跃 |
| P1 | Research Map / 技术空间地图 | 两者结合 | L | 差异化亮点 |
| P1 | 假设演化链 | DeepScientist | S | 研究严谨性 |
| P2 | 方案版本管理（DB 版本链） | Gemini 模块 B | M | 规避设计支持 |
| P2 | 人机协作 redirect | DeepScientist | M | 灵活性 |
| P2 | Webhook 通知 | DeepScientist | S | 集成能力 |
| P3 | AI 模拟论证（替代真实实验） | Gemini 模块 A | L | 交底书增强 |

---

## 5. 不采纳的部分

| 方案 | 不采纳理由 |
|------|-----------|
| AI 全自主执行实验代码 | InnoForge 定位是验证辅助，不是自主研究 |
| Docker/WASM 沙箱 | 破坏零依赖单体架构的核心优势 |
| Git 子仓库 | 目标用户不懂 Git，增加认知负担 |
| npm/Python 依赖 | Rust 单体更适合目标场景 |
| TUI 终端界面 | 已有 Web + MCP，够用 |

---

## 6. 下一步行动

建议 v0.6.0 版本聚焦 P0 三件事：

```
v0.6.0 — 从「一次性验证」到「持续创新研究」

1. 权利要求特征拆解 + 授权率预测（模块 C）
   → GET /api/idea/:id/patentability

2. 持续迭代 API
   → POST /api/idea/:id/iterate

3. Findings Memory
   → evidence_chain.finding_type + GET /api/findings?q=
```
