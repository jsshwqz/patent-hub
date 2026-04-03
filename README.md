# 研发助手 (Patent Hub)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/jsshwqz/patent-hub)](https://github.com/jsshwqz/patent-hub/releases)

**仓库地址：** [GitHub](https://github.com/jsshwqz/patent-hub) ｜ [Gitee（国内）](https://gitee.com/jsshwqz/patent-hub)

> AI 辅助的技术验证工具。支持专利/文献检索、方案可行性分析、多角度 AI 推演，帮助研发人员快速验证想法。

### 📥 下载 ｜ [📖 使用文档](#快速开始)

| 平台 | GitHub 下载 | Gitee 下载（国内快） |
|------|------------|-------------------|
| Windows | [📦 下载](https://github.com/jsshwqz/patent-hub/releases/latest) | [📦 下载](https://gitee.com/jsshwqz/patent-hub/releases) |
| Linux / macOS | [📦 下载](https://github.com/jsshwqz/patent-hub/releases/latest) | [📦 下载](https://gitee.com/jsshwqz/patent-hub/releases) |
| Android | [📱 下载](https://github.com/jsshwqz/patent-hub/releases/latest) | [📱 下载](https://gitee.com/jsshwqz/patent-hub/releases) |
| Docker | `docker run -p 3000:3000 jsshwqz/patent-hub` | 同左 |

> 启动后打开浏览器访问 **http://127.0.0.1:3000** 即可使用。无需安装数据库，无需联网（AI 功能需配置 API 密钥）。

[English](#english)

---

## 功能特性

- **创新推演** -- AI 多角度分析你的技术想法，自动检索相关专利和文献，生成可行性报告
- **专利/文献检索** -- 搜索全球专利和技术文献，发现相似方案与先行技术
- **方案对比** -- 并排对比多个技术方案，支持上传文件（PDF、DOCX、图片）
- **AI 助手** -- 技术问答、文献解读、多轮讨论
- **13 步分析流水线** -- 从想法提交到可行性报告，自动完成检索、分析、评分全流程
- **技术附图查看** -- 查看文献技术附图，本地图片代理
- **PDF 导出** -- 导出技术详情（含附图）为 PDF
- **收藏与标签** -- 通过收藏夹和标签管理技术文献
- **AI 自动容灾** -- 多 AI 服务商自动切换（智谱 GLM、OpenRouter、Gemini、OpenAI、NVIDIA、DeepSeek）
- **中英双语** -- 完整的中文/英文界面支持
- **零配置数据库** -- 内嵌 SQLite + FTS5 全文搜索，无需安装

---

## 快速开始

### 环境要求

- [Rust](https://rustup.rs/) (1.70+)

### 源码运行

```bash
# GitHub
git clone https://github.com/jsshwqz/patent-hub.git
# 或 Gitee（国内快）
git clone https://gitee.com/jsshwqz/patent-hub.git

cd patent-hub
cp .env.example .env
# 编辑 .env 填入 API 密钥（可选）
cargo run --release --bin patent-hub
```

### 使用发布包

1. 从 [GitHub Releases](https://github.com/jsshwqz/patent-hub/releases) 或 [Gitee Releases](https://gitee.com/jsshwqz/patent-hub/releases) 下载
2. 解压
3. 运行 `start.bat`（Windows）或 `./start.sh`（Linux/macOS）
4. 打开 http://127.0.0.1:3000

### Docker

```bash
docker build -t patent-hub .
docker run -p 3000:3000 -v patent-data:/data patent-hub
```

### Android

从 [GitHub Releases](https://github.com/jsshwqz/patent-hub/releases) 或 [Gitee Releases](https://gitee.com/jsshwqz/patent-hub/releases) 下载 `patent-hub-android.apk` 安装即可。内嵌 Axum 服务器，本地运行，无需联网。

---

## 配置说明

所有设置均可通过**设置页面**（http://localhost:3000/settings）或 `.env` 文件配置。

| 变量 | 是否必需 | 说明 |
|------|----------|------|
| `AI_BASE_URL` | AI 功能需要 | 任意 OpenAI 兼容 API 端点 |
| `AI_API_KEY` | AI 功能需要 | AI 服务 API 密钥 |
| `AI_MODEL` | AI 功能需要 | 模型名（如 `glm-4.7-flash`） |
| `SERPAPI_KEY` | 在线搜索需要 | [SerpAPI](https://serpapi.com/) 密钥，用于在线文献检索 |
| `FALLBACK_AI_*` | 可选 | 最多 5 个备用 AI 服务商 |
| `HOST` | 可选 | 服务器绑定地址（默认 `0.0.0.0`） |
| `PORT` | 可选 | 服务器端口（默认 `3000`） |

**没有 API 密钥？** 应用仍可使用 -- 检索使用本地数据库，AI 功能会显示配置引导。

### 支持的 AI 服务商

| 服务商 | 免费额度 | API 地址 |
|--------|----------|----------|
| 智谱 GLM（推荐） | 完全免费（glm-4.7-flash） | `https://open.bigmodel.cn/api/paas/v4` |
| DeepSeek | 低价 | `https://api.deepseek.com/v1` |
| OpenRouter | 部分模型免费 | `https://openrouter.ai/api/v1` |
| Google Gemini | 15 次/分钟 | `https://generativelanguage.googleapis.com/v1beta/openai/` |
| OpenAI | 付费 | `https://api.openai.com/v1` |
| NVIDIA | 有免费额度 | `https://integrate.api.nvidia.com/v1` |
| Ollama | 本地免费 | `http://localhost:11434/v1` |

---

## API 概览

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/search` | POST | 本地文献搜索 |
| `/api/search/online` | POST | 在线技术文献搜索 |
| `/api/search/export/csv` | POST | 导出搜索结果为 CSV |
| `/api/ai/summarize` | POST | AI 文献摘要 |
| `/api/ai/chat` | POST | AI 技术问答 |
| `/api/idea/submit` | POST | 提交想法 |
| `/api/idea/pipeline` | POST | 启动 13 步分析流水线 |
| `/api/idea/:id/chat` | POST | 多轮讨论 |
| `/api/patent/enrich/:id` | GET | 加载文献全文 + 附图 |
| `/api/patent/pdf/:id` | GET | 导出为 PDF |
| `/api/settings` | GET/POST | 配置管理 |
| `/api/collections` | GET/POST | 收藏管理 |

完整 API 文档：[docs/API.md](docs/API.md)

---

## 项目结构

```
patent-hub/                # Rust 主仓
  src/
    main.rs               # Web 服务器入口
    lib.rs                # 库导出 + Android JNI 入口
    ai.rs                 # AI 客户端（多服务商容灾）
    db.rs                 # SQLite 数据库 + FTS5 全文搜索
    patent.rs             # 数据模型
    routes/               # API 路由处理器
    pipeline/             # 13步创新推演流水线
      steps/
        deep_reasoning.rs # AI 深度分析
    bin/
      mcp-server.rs       # MCP 服务器
  mobile/                 # Dioxus 移动端（Rust UI）
  templates/              # HTML 页面模板
  static/                 # 静态资源
  tests/                  # 集成测试
  docs/                   # 文档
```

**关联仓库：**
- [patent-hub-desktop](https://gitee.com/jsshwqz/patent-hub-desktop) -- Tauri 桌面/移动端壳
- [patent-hub-ios](https://gitee.com/jsshwqz/patent-hub-ios) -- iOS 原生壳
- [patent-hub-harmony](https://gitee.com/jsshwqz/patent-hub-harmony) -- 鸿蒙原生壳

---

## 技术栈

- **后端**：Rust + Axum + SQLite（内嵌，零配置）
- **前端**：HTML 模板（Rust include_str! 内嵌）
- **AI**：任意 OpenAI 兼容 API + 自动容灾切换
- **搜索**：SQLite FTS5 + SerpAPI + Google Patents + 搜狗免费搜索（国内无VPN可用）
- **移动端**：Rust cdylib + JNI + Android WebView / Dioxus
- **国际化**：中英双语

---

## 致谢

### 架构灵感

- [Harness Research](https://github.com/Nimo1987/harness-research) -- 本项目的验证流水线受其启发，借鉴了以下设计理念：
  - **状态机驱动的流水线模式** -- 将复杂任务拆解为确定性步骤链，每步可独立重试/跳过
  - **LLM 与代码的职责分离** -- 评分、排序、去重等结构化任务由代码完成，AI 仅负责语义分析和推演
  - **多层搜索降级架构** -- 多数据源级联回退，确保任何网络环境都能返回结果
  - 注：本项目未复制 Harness Research 的任何源代码，所有实现均为原创

### 使用的算法

- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- 词频-逆文档频率，用于文本相似度匹配（实现于 pipeline/steps/similarity.rs）
- [Jaccard 相似系数](https://en.wikipedia.org/wiki/Jaccard_index) -- 集合相似度，用于重叠检测与矛盾分析（实现于 similarity.rs、contradiction.rs）
- [余弦相似度](https://en.wikipedia.org/wiki/Cosine_similarity) -- 向量空间相似度，作为 TF-IDF 流水线的一部分

### 第三方库与服务

- [Chart.js](https://www.chartjs.org/) v4 (MIT) -- 统计图表可视化
- [SerpAPI](https://serpapi.com/) -- 在线文献检索接口
- [Lens.org](https://www.lens.org/) -- 开放技术文献数据库 API
- [Sogou](https://www.sogou.com/) -- 国内免费搜索降级方案

---

## 许可证

[MIT](LICENSE)

---

## 贡献

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing`)
3. 提交更改
4. 推送到分支
5. 发起 Pull Request

---

## 获取更新通知 / Stay Updated

想在新版本发布时收到通知？

1. **GitHub Watch** -- 在仓库页面右上角点击 **Watch → Custom → Releases**，每次发布新版会收到邮件通知
2. **Gitee 关注** -- 在 [Gitee 仓库](https://gitee.com/jsshwqz/patent-hub) 点击 **Watch**
3. **Release 页面** -- 查看最新版本：[GitHub Releases](https://github.com/jsshwqz/patent-hub/releases) | [Gitee Tags](https://gitee.com/jsshwqz/patent-hub/tags)
4. **RSS 订阅** -- GitHub Releases 的 RSS 地址：`https://github.com/jsshwqz/patent-hub/releases.atom`

---

<a name="english"></a>
## English

> **R&D Assistant** -- AI-powered technical validation tool. Patent/literature search, feasibility analysis, and multi-angle AI reasoning to help engineers validate ideas quickly.

### Features

- **Innovation Reasoning** -- AI analyzes your technical ideas from multiple angles, searches related patents and literature, generates feasibility reports
- **Patent/Literature Search** -- Search global patents and technical literature, discover similar solutions and prior art
- **Solution Comparison** -- Side-by-side comparison of multiple technical solutions with file upload support (PDF, DOCX, images)
- **AI Assistant** -- Technical Q&A, literature interpretation, multi-round discussions
- **13-Step Analysis Pipeline** -- From idea submission to feasibility report, automated search, analysis, and scoring
- **Technical Drawings** -- View technical drawings with local image proxy
- **PDF Export** -- Export technical details (with drawings) to PDF
- **Collections & Tags** -- Organize technical literature with collections and tags
- **AI Failover** -- Automatic failover across multiple AI providers (Zhipu GLM, OpenRouter, Gemini, OpenAI, NVIDIA, DeepSeek)
- **i18n** -- Full Chinese/English bilingual support
- **Zero Config Database** -- Embedded SQLite with FTS5 full-text search, no installation needed

### Quick Start

#### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)

#### Run from Source

```bash
# GitHub
git clone https://github.com/jsshwqz/patent-hub.git
# or Gitee (faster in China)
git clone https://gitee.com/jsshwqz/patent-hub.git

cd patent-hub
cp .env.example .env
# Edit .env with your API keys (optional)
cargo run --release --bin patent-hub
```

#### Run from Release Package

1. Download from [GitHub Releases](https://github.com/jsshwqz/patent-hub/releases) or [Gitee Releases](https://gitee.com/jsshwqz/patent-hub/releases)
2. Extract the archive
3. Run `start.bat` (Windows) or `./start.sh` (Linux/macOS)
4. Open http://127.0.0.1:3000

#### Docker

```bash
docker build -t patent-hub .
docker run -p 3000:3000 -v patent-data:/data patent-hub
```

#### Android

Download `patent-hub-android.apk` from [GitHub Releases](https://github.com/jsshwqz/patent-hub/releases) or [Gitee Releases](https://gitee.com/jsshwqz/patent-hub/releases). Embedded Axum server runs locally, no internet required.

### Configuration

All settings can be configured via the **Settings page** (http://localhost:3000/settings) or `.env` file.

| Variable | Required | Description |
|----------|----------|-------------|
| `AI_BASE_URL` | For AI features | Any OpenAI-compatible API endpoint |
| `AI_API_KEY` | For AI features | API key for AI service |
| `AI_MODEL` | For AI features | Model name (e.g., `glm-4.7-flash`) |
| `SERPAPI_KEY` | For online search | [SerpAPI](https://serpapi.com/) key for online literature search |
| `FALLBACK_AI_*` | Optional | Up to 5 backup AI providers |
| `HOST` | Optional | Server bind address (default: `0.0.0.0`) |
| `PORT` | Optional | Server port (default: `3000`) |

**No API keys?** The app still works -- search uses local database, AI features show helpful setup messages.

#### Supported AI Providers

| Provider | Free Tier | Base URL |
|----------|-----------|----------|
| Zhipu GLM (Recommended) | Free (glm-4.7-flash) | `https://open.bigmodel.cn/api/paas/v4` |
| DeepSeek | Low cost | `https://api.deepseek.com/v1` |
| OpenRouter | Free models available | `https://openrouter.ai/api/v1` |
| Google Gemini | 15 RPM free | `https://generativelanguage.googleapis.com/v1beta/openai/` |
| OpenAI | Paid | `https://api.openai.com/v1` |
| NVIDIA | Free tier | `https://integrate.api.nvidia.com/v1` |
| Ollama | Local/Free | `http://localhost:11434/v1` |

### Tech Stack

- **Backend**: Rust + Axum + SQLite (embedded, zero-config)
- **Frontend**: Vanilla HTML/CSS/JS (no build tools needed)
- **AI**: Any OpenAI-compatible API + automatic failover
- **Search**: SQLite FTS5 + SerpAPI + Google Patents
- **Mobile**: Rust cdylib/staticlib + JNI (Android) / FFI (iOS) + WebView
- **i18n**: Shared JS translation system

### Credits

- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- Term frequency-inverse document frequency for text similarity matching
- [Jaccard Similarity](https://en.wikipedia.org/wiki/Jaccard_index) -- Set similarity metric for overlap detection and contradiction analysis
- [Cosine Similarity](https://en.wikipedia.org/wiki/Cosine_similarity) -- Vector space similarity, part of the TF-IDF pipeline
- [Harness Research](https://github.com/Nimo1987/harness-research) -- Architectural inspiration: state-machine pipeline pattern and separation of LLM vs. code responsibilities

### License

[MIT](LICENSE)
