# 研发输出深度门禁（发布前强制）

## 目标
避免“有返回但无价值”的伪通过。
发布前除功能可用外，必须验证 AI 输出达到研发可用深度。

## 门禁对象
- `/api/ai/chat`
- `/api/ai/compare`
- `/api/ai/inventiveness-analysis`
- `/api/ai/office-action-response`

## 通过标准（全部满足）
1. 结论命中数 >= 1
2. 依据命中数 >= 5
3. 维度命中数 >= 3
4. 风险命中数 >= 3
5. 建议命中数 >= 3
6. 输出长度 >= 350 字
7. 不包含硬失败信号（如 `AI error`、`无效或已过期`、`未配置`、`分析失败`）

说明：命中数由关键词规则统计，不做主观放行。

## 执行命令
```powershell
python tools/depth_gate_test.py `
  --worktree D:\test\innoforge-v053 `
  --bin innoforge `
  --out docs/depth-gate-runs/run-<timestamp>
```

## 输出物
- `depth_gate_result.json`：完整原始结果（含每条输出文本和打分明细）
- `depth_gate_summary.md`：汇总报告（通过率、失败原因、原文摘录）
- `server.log`：服务日志

## 发布规则
- 功能门禁（页面/API）通过 + 深度门禁通过，才允许发布。
- 任一门禁失败，必须修复并重测，直到全部通过。
