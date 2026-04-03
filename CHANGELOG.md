# 更新日志 / Changelog

所有重要变更都会记录在此文件中。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/)。
All notable changes are documented here. Format based on [Keep a Changelog](https://keepachangelog.com/).

---

## [v0.4.4] - 2026-04-03

### 新增 / Added
- **多维深度推演引擎** -- AI 深度分析升级为 7 轮多维推演（科学推导/辩证批判/知识审计/本质还原/减法思维/跨域映射 + 跨维度合成）
  Multi-dimensional deep reasoning engine -- 7-round analysis (scientific deduction / dialectical critique / knowledge audit / essence reduction / subtractive thinking / cross-domain mapping + synthesis)

### 修复 / Fixed
- 侵权评估结果字段名修正（assessment → analysis，前端可正确显示）
  Risk assessment field name fix (assessment → analysis)
- 创意删除级联清理特征卡片（FK constraint 修复）
  Idea deletion cascades to feature cards (FK constraint fix)
- 13 步文案全面修正（模板 + 注释中残留的 "12 步" 全部更新）
  All "12-step" text updated to "13-step" across templates and comments
- 设置页备用 AI 表单 XSS 加固（innerHTML → createElement）
  Settings page fallback AI form XSS hardening
- gitignore 清理（忽略临时测试文件）
  gitignore cleanup for temp test files

---

## [v0.4.3] - 2026-04-02

### 新增 / Added
- **Pipeline 13 步** -- 新增 PriorArtCluster 步骤（现有技术按主题聚类），优化矛盾检测上游数据
  Pipeline upgraded to 13 steps -- PriorArtCluster groups similar prior art by topic
- **特征卡片系统** -- Feature Card CRUD + 差异对比 API（`/api/ideas/:id/feature-cards`, `/api/feature-cards/diff`）
  Feature Cards system -- CRUD + diff comparison API for structured idea features
- **AI 流式聊天** -- SSE 方式逐字返回 AI 回答（`/api/ai/chat/stream`）
  AI streaming chat -- SSE-based token-by-token response
- **Pipeline 断点续跑** -- 中断后可从快照恢复（`/api/idea/:id/resume`）
  Pipeline resume -- restore from snapshot after interruption
- **HTML 验证报告** -- 可视化验证结果页面（`/api/idea/:id/report.html`）
  HTML validation report -- visual report page
- **批量创意对比** -- 2-10 个创意同时对比（`/api/ideas/batch-compare`）
  Batch idea comparison -- compare 2-10 ideas simultaneously
- **CORS 中间件** -- 支持跨域 API 调用（MCP 客户端等）
  CORS middleware -- enables cross-origin API calls for MCP clients

### 安全修复 / Security Fixes
- **XSS 防护** -- 全部 5 个 HTML 模板加入 DOMPurify，innerHTML 改为 createElement + textContent
  XSS protection -- DOMPurify added to all 5 HTML templates, innerHTML replaced with safe DOM APIs
- **SSRF 防护** -- 图片代理添加域名白名单（仅允许 googleapis.com / espacenet.com / sogou.com）
  SSRF protection -- image proxy now validates URL against domain allowlist
- **AI 全局超时** -- 从 24 分钟最坏情况降为 60 秒全局上限
  AI global timeout -- worst case reduced from 24 min to 60s global cap

### 改进 / Improved
- Pipeline 通道自动清理（超 5 分钟自动移除，防止内存泄漏）
  Pipeline channel auto-cleanup (removes stale entries after 5 min)
- 数据库 schema 升级到 v6（新增 feature_cards 表）
  Database schema upgraded to v6 (feature_cards table)
- Skill Router 安全规则：允许代码审查类任务讨论漏洞
  Skill Router security: allow code review tasks that discuss vulnerabilities

---

## [v0.4.2] - 2026-03-30

### 新增 / Added
- **历史记录增强** -- 显示精确时间（HH:MM）而非仅日期，同一天多条记录可清晰区分
  History records enhancement -- show precise time (HH:MM), clearly distinguish same-day entries
- **内容摘要预览** -- 历史列表显示创意描述前 40 字，快速识别每条记录
  Description preview -- show first 40 chars in history list for quick identification
- **对话计数标识** -- 显示每条创意的对话消息数，一目了然
  Chat count indicator -- show conversation message count per idea
- **自动滚动** -- 点击历史记录后自动滚动到对话区域
  Auto-scroll to discussion panel when clicking history items

### 改进 / Improved
- IdeaSummary 增加 description 和 message_count 字段
  IdeaSummary includes description and message_count fields
- list_ideas 查询优化，子查询统计消息数
  list_ideas query optimized with subquery for message count

---

## [v0.4.1] - 2026-03-30

### 新增 / Added
- **首页重塑** -- 从搜索框改为研发助手入口，三种模式（AI 对话 / 快速验证 / 深度验证）
  Homepage redesign -- from search box to R&D assistant entry with 3 modes
- **AI 对话增强** -- 专家级 prompt + 思维框架（第一性原理 / TRIZ / 逆向工程）
  AI conversation enhancement -- expert-level prompt with thinking frameworks
- **智能上下文管理** -- 超 8 轮自动压缩 + 摘要长记忆
  Smart context management -- auto-compress after 8 rounds + summary long memory
- **历史记录管理** -- 支持删除创意记录（级联清理对话）
  History management -- delete ideas with cascading message cleanup
- **文件上传** -- 首页拖拽上传 + 聊天区 📎 附件按钮（PDF/Word/图片）
  File upload -- drag & drop on homepage + chat attachment button

### 修复 / Fixed
- 评分显示精度（97.19... → 97.2）/ Score display precision fix
- AI 设置回显（不再重置为默认智谱）/ AI settings dropdown reflects saved value
- SerpAPI 免费额度（100 → 250 次/月）/ SerpAPI free quota corrected
- BAT 启动脚本中文编码 / BAT script Chinese encoding fix
- 讨论总结错误改为友好内联提示 / Discussion summary error as inline message

---

## [v0.4.0] - 2026-03-30

### 重大变更 / Breaking Changes
- **Pipeline 统一** -- api_idea_analyze 改为 pipeline 快捷模式，删除 220 行重复代码
  Pipeline unification -- api_idea_analyze uses quick_mode wrapper, removed 220 lines of duplicate code
- **仓库架构重组** -- 主仓瘦身为纯 Rust 核心，iOS/鸿蒙/Tauri 拆至独立仓库
  Repository restructuring -- core repo is pure Rust, iOS/HarmonyOS/Tauri split to independent repos

### 新增 / Added
- PipelineRunner 支持 quick_mode，跳过非必要步骤加速验证
  PipelineRunner supports quick_mode, skipping non-essential steps
- 前端「快速验证」（6 步）/ 「深度验证」（12 步）切换
  Frontend quick validation (6 steps) / deep validation (12 steps) toggle
- Android 固定签名证书，后续更新可直接覆盖安装
  Android fixed signing certificate for seamless updates
- CHANGELOG.md + CONTRIBUTING.md / Changelog and contributing guide

### 改进 / Improved
- cargo fmt 全量格式化 + cargo clippy 零警告 / Full formatting + zero clippy warnings
- .gitattributes 优化 GitHub 语言统计（Rust 占比 90%+）/ GitHub language stats optimized
- 清理冗余文件（Node.js 依赖、过时 CI、构建产物）/ Cleanup redundant files

---

## [v0.3.5] - 2026-03-29

### 新增 / Added
- 搜狗搜索内置免费方案，零配置开箱即用（国内无需 VPN 无需 Key）
  Built-in Sogou search, zero-config for China (no VPN, no API key needed)
- Bing Web Search API 支持（国内可用）/ Bing Web Search API support
- Lens.org 专利搜索 API 支持 / Lens.org patent search API support
- 搜索降级链：SerpAPI → Google Patents → Bing → Lens.org → 搜狗 → 本地 DB
  Search degradation chain: SerpAPI → Google Patents → Bing → Lens.org → Sogou → Local DB
- 设置页面新增「国内搜索配置」区域 / Settings page: domestic search configuration section
- 使用免费搜索时自动提示升级 / Auto-prompt to upgrade when using free search

---

## [v0.3.4] - 2026-03-29

### 修复 / Fixed
- APP 端创意分析 AI 失败时降级评分 / Fallback scoring when AI analysis fails on APP
- Tauri 前端浏览器测试路由 / Tauri frontend browser test routing
- 文档上传支持 .docx + GBK 编码处理 / Document upload: .docx support + GBK encoding
- AI 错误提示改善 / Improved AI error messages
- Pipeline SSE 时序修复 / Pipeline SSE timing fix
- 空 query 搜索校验 + 收藏专利 ID 验证 / Empty query validation + favorite patent ID check
- 收藏标签前端 UI 优化 / Favorites tag UI polish

---

## [v0.3.3] - 2026-03-27

### 新增 / Added
- 12 步创新验证流水线（ParseInput → ScoreNovelty → AI 分析 → 报告生成）
  12-step innovation validation pipeline
- 设置持久化（SQLite 存储，重启不丢失）/ Settings persistence in SQLite
- 鸿蒙 APP 构建配置 / HarmonyOS APP build configuration
- 多平台 APP 支持框架 / Multi-platform APP framework
- 全面中文化 / Full Chinese localization

### 修复 / Fixed
- 测试断言修复 / Test assertion fixes
- 引用准确性提升 / Citation accuracy improvements
- i18n 补全 / i18n completeness
- Clippy 错误修复 / Clippy error fixes

---

## [v0.3.2] - 2026-03-26

### 新增 / Added
- 设置页面改造：多 AI 预设 + 注册引导 + 自定义支持
  Settings page redesign: multi-AI presets + registration guide + custom support
- 纯 Rust Android APP 方案（无 Java 依赖）
  Pure Rust Android APP (no Java dependency)

### 修复 / Fixed
- 设置保存优化（先更新内存，.env 写入可选）/ Settings save optimization
- AI 未配置时友好中文提示 / Friendly Chinese prompt when AI not configured
- Android APK cdylib 共享库替代可执行文件 / Android APK: cdylib shared library
- wry 0.46 API 变更适配 / wry 0.46 API changes adaptation

---

## [v0.3.1] - 2026-03-26

### 新增 / Added
- Android APP 支持（ARM64 + x86_64 双架构）
  Android APP support (ARM64 + x86_64 dual architecture)
- Dioxus Mobile 移动端方案 / Dioxus Mobile solution
- 纯 Java WebView 方案（最终采用）/ Pure Java WebView approach (final choice)

### 修复 / Fixed
- APK 签名路径和上传条件 / APK signing path and upload conditions
- Android APP 闪退和图标问题 / Android APP crash and icon issues
- 静态文件内嵌二进制（Android 兼容）/ Static files embedded in binary
- Android 9+ 允许 localhost 明文 HTTP / Android 9+ cleartext HTTP for localhost
- CI 构建流程优化 / CI build workflow optimization

---

## [v0.3.0] - 2026-03-25

### 新增 / Added
- IPC 分类浏览 API / IPC classification browsing API
- 混合相关性评分算法（TF-IDF + 位置加权）/ Hybrid relevance scoring (TF-IDF + position weighting)
- Chart.js 可视化统计图表 / Chart.js visualization and statistics
- 对比矩阵 + 侵权风险评估 UI / Comparison matrix + infringement risk assessment UI
- 权利要求分析 + 批量摘要 / Claims analysis + batch summarize
- PWA 支持（可安装为桌面/移动应用）/ PWA support (installable as app)
- MCP Server（AI Agent 集成）/ MCP Server for AI Agent integration

---

## [v0.2.0] - 2026-03-24

### 新增 / Added
- AI 多服务商自动容灾切换（智谱 GLM、OpenRouter、Gemini、OpenAI、NVIDIA、DeepSeek）
  AI multi-provider automatic failover (6 providers)
- 专利技术附图查看 + 本地图片代理 / Patent drawings viewer + local image proxy
- PDF 导出（含附图）/ PDF export with drawings
- 中英双语国际化（i18n）/ Bilingual internationalization
- 搜索结果智能去重 / Smart search result deduplication

---

## [v0.1.0] - 2026-02-24

### 新增 / Added
- 在线专利搜索（SerpAPI + Google Patents）/ Online patent search
- 本地 SQLite 数据库 + FTS5 全文搜索 / Local SQLite + FTS5 full-text search
- AI 智能分析（OpenAI 兼容 API）/ AI analysis (OpenAI compatible API)
- 专利对比分析 / Patent comparison analysis
- 相似专利推荐 / Similar patent recommendations
- 文件上传对比 / File upload comparison
- 搜索历史管理 / Search history management
- 统计图表展示 / Statistics charts
- Excel 数据导出 / Excel data export
- 跨平台支持（Windows/Linux/macOS）/ Cross-platform support
- 设置页面（网页配置 API Key）/ Settings page (web-based API key configuration)
