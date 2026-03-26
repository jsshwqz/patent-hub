# Patent Hub

[![CI](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/jsshwqz/patent-hub)](https://github.com/jsshwqz/patent-hub/releases)

> **Patent Hub** -- 专利检索、AI 智能分析与管理平台，支持中英双语。

[English](#english) | [Gitee 镜像](https://gitee.com/jsshwqz/patent-hub)

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
patent-hub/
  src/
    main.rs          # Web 服务器入口
    lib.rs           # 库导出 + Android JNI 入口
    ai.rs            # AI 客户端（多服务商容灾）
    db.rs            # SQLite 数据库 + FTS5 全文搜索
    patent.rs        # 数据模型
    routes/          # API 路由处理器
    bin/
      skill-router.rs  # 独立 CLI 工具
      mobile.rs        # 移动端入口
      mcp-server.rs    # MCP 服务器
  templates/         # HTML 模板（7 个页面）
  static/            # CSS、JS（含国际化）
  tests/             # 集成测试
  docs/              # 文档
```

---

## 技术栈

- **后端**：Rust + Axum + SQLite（内嵌，零配置）
- **前端**：原生 HTML/CSS/JS（无需构建工具）
- **AI**：任意 OpenAI 兼容 API，自动容灾切换
- **搜索**：SQLite FTS5 + SerpAPI + Google Patents
- **移动端**：Rust cdylib + JNI + Android WebView
- **国际化**：JS 翻译系统，中英双语

---

## 致谢

本项目的分析流水线设计受以下项目和理论启发：

- [Harness Research](https://github.com/Nimo1987/harness-research) -- 状态机驱动的研究引擎，LLM/代码职责分离、CRAAP 混合评分、质量门机制
- [CRAAP 框架](https://library.csuchico.edu/help/source-or-information-good) -- 信息源评估框架（Currency, Relevance, Authority, Accuracy, Purpose）
- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- 文本相似度匹配算法，用于权利要求与现有技术匹配
- [Shannon 信息熵](https://en.wikipedia.org/wiki/Entropy_(information_theory)) -- 信息多样性度量

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

For English documentation, see [docs/](docs/) directory.

### Quick Start

```bash
git clone https://github.com/jsshwqz/patent-hub.git
cd patent-hub
cp .env.example .env
cargo run --release --bin patent-hub
# Open http://127.0.0.1:3000/search
```

### Key Features

- Global patent search with AI-powered analysis
- Multi-provider AI failover (Zhipu GLM, DeepSeek, OpenRouter, Gemini, OpenAI)
- Idea validation with multi-round dialogue
- Patent comparison, PDF export, collections
- Android APK with embedded server (local-first)
- Zero-config embedded SQLite with FTS5
