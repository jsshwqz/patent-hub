# 技能路由器实现计划 / Skill Router Implementation Plan

> **给 Claude:** 必需子技能：使用 superpowers:executing-plans 逐任务实现此计划。

**目标：** 构建一个独立的 Rust 技能路由器 CLI，执行规范流水线 `任务 -> 能力 -> 技能 -> 执行 -> 生命周期更新`，采用本地优先、安全优先的设计。

**架构：** 在 `src/bin/` 下添加独立的 `skill-router` 二进制文件，将路由器核心放在 `src/skill_router/` 中，保持现有 Axum 应用不受影响。路由器状态持久化到工作空间本地的 `.skill-router/` 目录，优先使用 `skills/` 中的本地技能，不自动安装远程技能，仅在工作目录内合成安全的占位技能。

**技术栈：** Rust 2021, serde/serde_json, anyhow, chrono, uuid, std::process, std::fs

---

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an isolated Rust Skill Router CLI that executes the spec pipeline `task -> capability -> skill -> execution -> lifecycle update` with local-first, security-first behavior.

**Architecture:** Add a standalone `skill-router` binary under `src/bin/` and keep the router core in `src/skill_router/` so the existing Axum app stays untouched. Persist router state under the workspace-local `.skill-router/` directory, prefer local skills from `skills/`, never auto-install remote skills, and synthesize only safe placeholder skills inside the working directory.

**Tech Stack:** Rust 2021, serde/serde_json, anyhow, chrono, uuid, std::process, std::fs

---

### 任务 1：定义隔离的模块布局 / Task 1: Define the isolated module layout

**文件 / Files:**
- 创建: `src/skill_router/mod.rs`
- 创建: `src/skill_router/types.rs`
- 创建: `src/bin/skill-router.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加一个集成测试，期望 `skill-router` 二进制文件接受任务字符串并失败，因为路由器模块尚不存在。

Add an integration test that expects the `skill-router` binary to accept a task string and fail because the router modules do not exist yet.

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline -- --nocapture`
预期: 因缺少模块或二进制引用而失败。

Run: `cargo test skill_router_pipeline -- --nocapture`
Expected: FAIL with missing module or binary references.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

创建路由器模块脚手架和一个解析任务并打印占位 JSON 错误的 CLI 入口。

Create the router module scaffold and a CLI entrypoint that parses the task and prints a placeholder JSON error.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline -- --nocapture`
预期: CLI 烟雾测试通过，后续测试仍然失败。

Run: `cargo test skill_router_pipeline -- --nocapture`
Expected: PASS for the CLI smoke-path, with later tests still failing.

### 任务 2：构建规划器、能力注册表和本地加载器 / Task 2: Build planner, capability registry, and local loader

**文件 / Files:**
- 创建: `src/skill_router/planner.rs`
- 创建: `src/skill_router/loader.rs`
- 创建: `src/skill_router/capability_registry.rs`
- 修改: `src/skill_router/types.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加测试：
- 从 `parse this yaml` 推断出 `yaml_parse`
- 从 `skills/<name>/skill.json` 加载本地技能
- 拒绝非 snake_case 的格式错误能力

Add tests that:
- infer `yaml_parse` from `parse this yaml`
- load a local skill from `skills/<name>/skill.json`
- reject malformed capabilities that are not snake_case

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline planner loader -- --nocapture`
预期: 因缺少规划器/加载器行为而失败。

Run: `cargo test skill_router_pipeline planner loader -- --nocapture`
Expected: FAIL on missing planner/loader behavior.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

实现确定性能力推断、能力验证和仅从工作空间目录加载本地技能。

Implement deterministic capability inference, capability validation, and local skill loading from workspace directories only.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline planner loader -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline planner loader -- --nocapture`
Expected: PASS.

### 任务 3：添加注册表、匹配器和生命周期建议 / Task 3: Add registry, matcher, and lifecycle recommendation

**文件 / Files:**
- 创建: `src/skill_router/registry.rs`
- 创建: `src/skill_router/matcher.rs`
- 创建: `src/skill_router/lifecycle.rs`
- 修改: `src/skill_router/types.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加测试：
- 优先选择本地技能而非合成/远程候选者
- 将使用统计持久化到 `.skill-router/registry.json`
- 基于阈值返回 `keep`、`polish`、`publish_candidate`、`deprecate` 和 `purge_candidate`

Add tests that:
- prefer a local skill over synthesized/remote candidates
- persist usage stats into `.skill-router/registry.json`
- return `keep`, `polish`, `publish_candidate`, `deprecate`, and `purge_candidate` based on thresholds

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline matcher lifecycle -- --nocapture`
预期: 因缺少评分和持久化行为而失败。

Run: `cargo test skill_router_pipeline matcher lifecycle -- --nocapture`
Expected: FAIL on missing scoring and persistence behavior.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

实现注册表持久化、确定性技能评分和基于使用窗口的生命周期建议。

Implement registry persistence, deterministic skill scoring, and lifecycle recommendation from usage windows.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline matcher lifecycle -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline matcher lifecycle -- --nocapture`
Expected: PASS.

### 任务 4：实现带权限验证和日志的执行器 / Task 4: Implement executor with permission validation and logging

**文件 / Files:**
- 创建: `src/skill_router/executor.rs`
- 创建: `src/skill_router/security.rs`
- 修改: `src/skill_router/mod.rs`
- 修改: `src/skill_router/types.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加测试：
- 当权限缺失或显式不安全时拒绝执行
- 执行安全的内置测试技能并生成 SDK 格式的 JSON
- 将结构化执行日志追加到 `.skill-router/executions.log`

Add tests that:
- deny execution when permissions are missing or explicitly unsafe
- execute a safe builtin test skill and produce SDK-shaped JSON
- append structured execution logs to `.skill-router/executions.log`

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline executor -- --nocapture`
预期: 因权限强制执行或缺少日志输出而失败。

Run: `cargo test skill_router_pipeline executor -- --nocapture`
Expected: FAIL on permission enforcement or missing log output.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

实现默认拒绝的权限规范化、安全路径验证、合成/测试技能的内置执行支持和追加写入的执行日志。

Implement permission normalization with default-deny, safe path validation, builtin execution support for synthesized/test skills, and append-only execution logging.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline executor -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline executor -- --nocapture`
Expected: PASS.

### 任务 5：实现可信源搜索和安全合成回退 / Task 5: Implement trusted-source search and safe synthesis fallback

**文件 / Files:**
- 创建: `src/skill_router/online_search.rs`
- 创建: `src/skill_router/synth.rs`
- 修改: `src/skill_router/mod.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加测试：
- 仅搜索已配置的可信源
- 不自动安装远程技能
- 在 `.skill-router/generated-skills/` 内合成占位技能
- 通过执行器执行合成的占位技能

Add tests that:
- search only configured trusted sources
- do not auto-install remote skills
- synthesize a placeholder skill inside `.skill-router/generated-skills/`
- execute the synthesized placeholder through the executor

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline synth -- --nocapture`
预期: 因缺少可信源/合成行为而失败。

Run: `cargo test skill_router_pipeline synth -- --nocapture`
Expected: FAIL on missing trusted-source/synthesis behavior.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

实现可信源候选发现、仅本地元数据结果和在工作空间内写入的内置占位合成器。

Implement trusted-source candidate discovery, local metadata-only results, and a builtin-backed placeholder synthesizer that writes only inside the workspace.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline synth -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline synth -- --nocapture`
Expected: PASS.

### 任务 6：通过 CLI 连接完整路由器流水线 / Task 6: Wire the full router pipeline through the CLI

**文件 / Files:**
- 修改: `src/bin/skill-router.rs`
- 修改: `src/skill_router/mod.rs`
- 测试: `tests/skill_router_pipeline.rs`

**步骤 1：编写失败测试 / Step 1: Write the failing test**

添加端到端测试：调用二进制文件传入任务，路由到技能，执行它，更新注册表指标，并返回生命周期元数据。

Add an end-to-end test that invokes the binary with a task, routes to a skill, executes it, updates registry metrics, and returns lifecycle metadata.

**步骤 2：运行测试验证失败 / Step 2: Run test to verify it fails**

运行: `cargo test skill_router_pipeline::end_to_end -- --nocapture`
预期: 因流水线未完全连接而失败。

Run: `cargo test skill_router_pipeline::end_to_end -- --nocapture`
Expected: FAIL because the pipeline is not fully connected yet.

**步骤 3：编写最小实现 / Step 3: Write minimal implementation**

将规划器、加载器、匹配器、搜索、合成、执行器、注册表和生命周期连接到 CLI 使用的单一编排方法中。

Connect planner, loader, matcher, search, synthesis, executor, registry, and lifecycle in a single orchestrator method used by the CLI.

**步骤 4：运行测试验证通过 / Step 4: Run test to verify it passes**

运行: `cargo test skill_router_pipeline::end_to_end -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline::end_to_end -- --nocapture`
Expected: PASS.

### 任务 7：验证实现 / Task 7: Verify the implementation

**文件 / Files:**
- 测试: `tests/skill_router_pipeline.rs`
- 测试: `Cargo.toml`

**步骤 1：运行集中验证 / Step 1: Run focused verification**

运行: `cargo test skill_router_pipeline -- --nocapture`
预期: 通过。

Run: `cargo test skill_router_pipeline -- --nocapture`
Expected: PASS.

**步骤 2：运行完整验证 / Step 2: Run full verification**

运行: `cargo test`
预期: 通过，或单独记录不相关的预存失败。

Run: `cargo test`
Expected: PASS, or document unrelated pre-existing failures separately.

**步骤 3：总结行为 / Step 3: Summarize behavior**

记录已实现的内容、已验证的内容，以及远程技能执行或沙箱方面的剩余限制。

Capture what was implemented, what was verified, and any residual limitations around remote skill execution or sandboxing.
