# Patent Hub

[![CI](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/jsshwqz/patent-hub)](https://github.com/jsshwqz/patent-hub/releases)

> **Patent Hub** -- 专利检索、AI 智能分析与管理平台，支持中英双语。

[English](#english) | [Gitee 镜像](https://gitee.com/jsshwqz/patent-hub)

---

## 第一次使用？

- 想先跑起来：看 [docs/快速上手.md](docs/快速上手.md)
- 想看更详细的中文说明：看 [docs/国内用户指南.md](docs/国内用户指南.md)
- 想了解接口：看 [docs/API.md](docs/API.md)

---

## 功能特性

- **专利检索** -- 全球专利搜索，支持相关性评分、去重、分类统计
- **AI 智能分析** -- AI 驱动的专利摘要、问答、创意验证与多轮对话
- **专利对比** -- 并排对比多个专利，支持上传文件（PDF、DOCX、图片）
- **专利附图** -- 查看专利技术附图，本地图片代理
- **PDF 导出** -- 导出专利详情（含附图）为 PDF
- **收藏与标签** -- 通过收藏夹和标签管理专利
- **AI 自动容灾** -- 多 AI 服务商自动切换（智谱 GLM、OpenRouter、Gemini、OpenAI、NVIDIA、DeepSeek）
- **中英双语** -- 完整的中文/英文界面支持
- **零配置数据库** -- 内嵌 SQLite + FTS5 全文搜索，无需安装

---

## 快速开始

### 环境要求

- [Rust](https://rustup.rs/) (1.70+)

### 源码运行

```bash
git clone https://github.com/jsshwqz/patent-hub.git
cd patent-hub
cp .env.example .env
# 编辑 .env 填入 API 密钥（可选）
cargo run --release --bin patent-hub
```

### 使用发布包

1. 从 [Releases](https://github.com/jsshwqz/patent-hub/releases) 下载
2. 解压
3. 运行 `start.bat`（Windows）或 `./start.sh`（Linux/macOS）
4. 打开 http://127.0.0.1:3000/search

### Docker

```bash
docker build -t patent-hub .
docker run -p 3000:3000 -v patent-data:/data patent-hub
```

### Android

从 [Releases](https://github.com/jsshwqz/patent-hub/releases) 下载 `patent-hub-android.apk` 安装即可。内嵌 Axum 服务器，本地运行，无需联网。

---

## 配置说明

所有设置均可通过**设置页面**（http://localhost:3000/settings）或 `.env` 文件配置。

| 变量 | 是否必需 | 说明 |
|------|----------|------|
| `AI_BASE_URL` | AI 功能需要 | 任意 OpenAI 兼容 API 端点 |
| `AI_API_KEY` | AI 功能需要 | AI 服务 API 密钥 |
| `AI_MODEL` | AI 功能需要 | 模型名（如 `glm-4.7-flash`） |
| `SERPAPI_KEY` | 在线搜索需要 | [SerpAPI](https://serpapi.com/) 密钥，用于搜索 Google Patents |
| `FALLBACK_AI_*` | 可选 | 最多 5 个备用 AI 服务商 |
| `HOST` | 可选 | 服务器绑定地址（默认 `0.0.0.0`） |
| `PORT` | 可选 | 服务器端口（默认 `3000`） |

**没有 API 密钥？** 应用仍可使用 -- 搜索使用本地数据库，AI 功能会显示配置引导。

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
| `/api/search` | POST | 本地专利搜索 |
| `/api/search/online` | POST | 在线专利搜索（SerpAPI / Google Patents） |
| `/api/search/export/csv` | POST | 导出搜索结果为 CSV |
| `/api/ai/summarize` | POST | AI 专利摘要 |
| `/api/ai/chat` | POST | AI 专利问答 |
| `/api/idea/submit` | POST | 提交创意验证 |
| `/api/idea/:id/chat` | POST | 创意多轮讨论 |
| `/api/patent/enrich/:id` | GET | 加载专利全文 + 附图 |
| `/api/patent/pdf/:id` | GET | 导出专利为 PDF |
| `/api/patent/image-proxy` | GET | 专利图片代理 |
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
    pipeline/             # 12步创新验证流水线
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
- **AI**：任意 OpenAI 兼容 API，自动容灾切换
- **搜索**：SQLite FTS5 + SerpAPI + Google Patents + 搜狗免费搜索（国内无VPN可用）
- **移动端**：Rust cdylib + JNI + Android WebView / Dioxus
- **国际化**：中英双语

---

## 致谢

本项目使用了以下算法，并受以下项目的架构理念启发：

- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- 词频-逆文档频率算法，用于权利要求与现有技术的文本相似度匹配（实现于 similarity.rs）
- [Jaccard 相似系数](https://en.wikipedia.org/wiki/Jaccard_index) -- 集合相似度度量，用于文本重叠检测与矛盾分析（实现于 similarity.rs、contradiction.rs）
- [余弦相似度](https://en.wikipedia.org/wiki/Cosine_similarity) -- 向量空间相似度计算，作为 TF-IDF 流水线的一部分使用
- [Harness Research](https://github.com/Nimo1987/harness-research) -- 架构理念启发：状态机驱动的流水线模式、LLM 与代码的职责分离

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

<a name="english"></a>
## English

> **Patent Hub** -- Patent search, AI analysis, and management platform with multi-language support.

### Features

- **Patent Search** -- Global patent search with relevance scoring, deduplication, and category statistics
- **AI Analysis** -- AI-powered patent summarization, Q&A, and idea validation with multi-round dialogue
- **Patent Comparison** -- Side-by-side patent comparison with file upload support (PDF, DOCX, images)
- **Patent Drawings** -- View patent technical drawings with local image proxy
- **PDF Export** -- Export patent details (with drawings) to PDF
- **Collections & Tags** -- Organize patents with collections and tags
- **AI Failover** -- Automatic failover across multiple AI providers (Zhipu GLM, OpenRouter, Gemini, OpenAI, NVIDIA, DeepSeek)
- **i18n** -- Full Chinese/English bilingual support
- **Zero Config Database** -- Embedded SQLite with FTS5 full-text search, no installation needed

### Quick Start

#### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)

#### Run from Source

```bash
git clone https://github.com/jsshwqz/patent-hub.git
cd patent-hub
cp .env.example .env
# Edit .env with your API keys (optional)
cargo run --release --bin patent-hub
```

#### Run from Release Package

1. Download from [Releases](https://github.com/jsshwqz/patent-hub/releases)
2. Extract the archive
3. Run `start.bat` (Windows) or `./start.sh` (Linux/macOS)
4. Open http://127.0.0.1:3000/search

#### Docker

```bash
docker build -t patent-hub .
docker run -p 3000:3000 -v patent-data:/data patent-hub
```

#### Android

Download `patent-hub-android.apk` from [Releases](https://github.com/jsshwqz/patent-hub/releases). Embedded Axum server runs locally, no internet required.

### Configuration

All settings can be configured via the **Settings page** (http://localhost:3000/settings) or `.env` file.

| Variable | Required | Description |
|----------|----------|-------------|
| `AI_BASE_URL` | For AI features | Any OpenAI-compatible API endpoint |
| `AI_API_KEY` | For AI features | API key for AI service |
| `AI_MODEL` | For AI features | Model name (e.g., `glm-4.7-flash`) |
| `SERPAPI_KEY` | For online search | [SerpAPI](https://serpapi.com/) key for Google Patents |
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

### API Overview

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/search` | POST | Local patent search |
| `/api/search/online` | POST | Online patent search (SerpAPI / Google Patents) |
| `/api/search/export/csv` | POST | Export results to CSV |
| `/api/ai/summarize` | POST | AI patent summarization |
| `/api/ai/chat` | POST | AI Q&A about patents |
| `/api/idea/submit` | POST | Submit idea for validation |
| `/api/idea/:id/chat` | POST | Multi-round idea discussion |
| `/api/patent/enrich/:id` | GET | Load patent full text + drawings |
| `/api/patent/pdf/:id` | GET | Export patent as printable PDF |
| `/api/patent/image-proxy` | GET | Proxy patent images |
| `/api/settings` | GET/POST | Configuration management |
| `/api/collections` | GET/POST | Collection management |

Full API docs: [docs/API.md](docs/API.md)

### Project Structure

```
patent-hub/
  src/
    main.rs          # Web server entry point
    lib.rs           # Library exports + Android JNI / iOS FFI
    ai.rs            # AI client with failover
    db.rs            # SQLite database with FTS5
    patent.rs        # Data models
    routes/          # API route handlers
    bin/
      skill-router.rs  # Standalone CLI tool
      mobile.rs        # Mobile entry point
      mcp-server.rs    # MCP server
  templates/         # HTML templates (7 pages)
  static/            # CSS, JS (i18n)
  ios-app/           # iOS Swift + WKWebView
  harmonyos/         # HarmonyOS ArkTS + WebView
  tests/             # Integration tests
  docs/              # Documentation
```

### Tech Stack

- **Backend**: Rust + Axum + SQLite (embedded, zero-config)
- **Frontend**: Vanilla HTML/CSS/JS (no build tools needed)
- **AI**: Any OpenAI-compatible API with automatic failover
- **Search**: SQLite FTS5 + SerpAPI + Google Patents
- **Mobile**: Rust cdylib/staticlib + JNI (Android) / FFI (iOS) + WebView
- **i18n**: Shared JS translation system

### Credits

The analysis pipeline uses the following algorithms and was inspired by the architectural philosophy of the following project:

- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- Term frequency-inverse document frequency for matching claims against prior art (implemented in similarity.rs)
- [Jaccard Similarity](https://en.wikipedia.org/wiki/Jaccard_index) -- Set similarity metric for text overlap detection and contradiction analysis (implemented in similarity.rs, contradiction.rs)
- [Cosine Similarity](https://en.wikipedia.org/wiki/Cosine_similarity) -- Vector space similarity computation, used as part of the TF-IDF pipeline
- [Harness Research](https://github.com/Nimo1987/harness-research) -- Architectural inspiration: state-machine pipeline pattern and separation of LLM vs. code responsibilities

### License

[MIT](LICENSE)
