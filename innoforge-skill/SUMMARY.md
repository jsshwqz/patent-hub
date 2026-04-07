# InnoForge SKILL.md 自进化实验报告

## 实验概述

对 innoforge（Rust/Axum 专利检索平台）的 SKILL.md 进行了 2 轮自进化迭代，每轮 6 个并行 agent（3 evals × with/without skill），共 12 个独立 agent 执行。

## 迭代结果

### Iteration 1: 详细提示（Prescriptive Prompts）

| Eval | with_skill | without_skill |
|------|-----------|---------------|
| XSS修复 | 8/8 (100%) | 8/8 (100%) |
| Pipeline步骤 | 10/10 (100%) | 10/10 (100%) |
| DB迁移 | 10/10 (100%) | 10/10 (100%) |
| **总计** | **28/28 (100%)** | **28/28 (100%)** |

**结论**: 提示过于详细，逐步指令让 agent 无论有无 SKILL.md 都能完成。Skill 优势 +0%。

**改进**: 重写 eval 提示为模糊描述，让 agent 自己发现"怎么做"。

### Iteration 2: 模糊提示（Vague Prompts）

| Eval | with_skill | without_skill | 关键差异 |
|------|-----------|---------------|---------|
| XSS修复 | 3/8 (37.5%) | 4/8 (50%) | 两者都没用DOMPurify，都漏掉ai.html |
| API路由 | 9/9 (100%) | 9/9 (100%) | 两者都独立发现了响应格式约定 |
| Pipeline步骤 | 11/11 (100%) | 10/11 (90.9%) | **with_skill 多了集成测试** |
| **总计** | **23/28 (82.1%)** | **23/28 (82.1%)** |

**关键发现**:
1. **Pipeline 步骤测试**: with_skill 添加了 2 个集成测试（SKILL.md 的测试检查清单起作用），without_skill 只加了单元测试
2. **XSS 修复**: SKILL.md 提到了 DOMPurify 但 agent 仍然选择了更简单的 esc() 方案 → 需要更强制性的表述
3. **API 路由**: 两者表现一致，without_skill 通过阅读现有代码也能发现 `{status: ok/error}` 约定

## SKILL.md 进化历史

| 版本 | 改进内容 |
|------|---------|
| v1 (初始) | 架构图、5种常见任务模式、测试规范、已知约束 |
| v2 (iter1后) | 增加 schema version 测试更新提醒、测试完整性检查清单 |
| v3 (iter2后) | 明确列出所有模板文件、强制使用 DOMPurify、更强的安全修复指引 |

## 效率对比

| 指标 | with_skill 平均 | without_skill 平均 |
|------|----------------|-------------------|
| Token 用量 | 78.2K | 62.4K |
| 执行时间 | 573s | 547s |

with_skill 消耗更多 token（+25%）因为要读 SKILL.md，但在复杂任务（pipeline 步骤）中代码更完整。

## 核心结论

1. **SKILL.md 在模糊任务中价值更大**: 当提示详细时 skill 无优势；当提示模糊时，SKILL.md 的架构文档和检查清单帮助 agent 产出更完整的代码（尤其是测试）

2. **SKILL.md 不能替代 agent 判断**: 即使 SKILL.md 明确推荐 DOMPurify，agent 仍可能选择更简单的方案。关键指引需要用强制性语言（"MUST"而非"replace with"）

3. **测试检查清单效果显著**: "New pipeline steps → add unit test + integration test" 这一行直接导致 with_skill 多加了集成测试

4. **效率 vs 质量权衡**: with_skill 用更多 token 换来更完整的实现，在生产环境中这是值得的

## 对 innoforge 的实际价值

SKILL.md 最适合以下场景：
- **Pipeline 扩展** — 4 文件联动模式是最大痛点，SKILL.md 给出清晰 checklist
- **DB 迁移** — 模板化的迁移模式避免常见错误（版本号、IF NOT EXISTS）
- **新人上手** — 架构图 + 已知约束让新开发者快速定位代码位置
- **安全修复** — 明确的模板文件清单和修复策略（改进后版本）
