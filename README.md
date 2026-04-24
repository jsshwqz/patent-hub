# 创研台 InnoForge

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**仓库地址：** [GitHub](https://github.com/jsshwqz/innoforge) | [Gitee（国内）](https://gitee.com/jsshwqz/innoforge)

很多产品不是死在实现阶段，而是死在更早之前：想法看起来不错，却缺少判断依据；方向似乎可行，却没有清晰方案；产品刚开始推进，才发现已有技术、专利壁垒或知识产权保护缺口。

**创研台 InnoForge** 面向开发者和研发人员，帮助你把最初的想法逐步推进为决策依据、执行方案、产品落地和知识产权保护，减少盲目投入，提高从创意到成果的转化效率。

[English](#english)

---

### 📥 下载

| 平台 | GitHub 下载 | Gitee 下载（国内快） |
|------|------------|-------------------|
| Windows | [📦 下载](https://github.com/jsshwqz/innoforge/releases/latest) | [📦 下载](https://gitee.com/jsshwqz/innoforge/releases) |
| Linux / macOS | [📦 下载](https://github.com/jsshwqz/innoforge/releases/latest) | [📦 下载](https://gitee.com/jsshwqz/innoforge/releases) |
| Android | [📱 下载](https://github.com/jsshwqz/innoforge/releases/latest) | [📱 下载](https://gitee.com/jsshwqz/innoforge/releases) |
| Docker | `docker run -p 3000:3000 jsshwqz/innoforge` | 同左 |

> 下载解压后运行（Windows `start.bat` / Linux `./start.sh`），打开 http://127.0.0.1:3000 即可使用。无需安装数据库。
> 若本机 `3000` 端口被系统占用/保留，可设置环境变量 `INNOFORGE_PORT`（如 `3900`）后启动。

---

## 核心能力

### 1. 创新验证流水线（13 步全自动）

输入你的技术想法，系统自动完成：

```
想法提交 → 关键词提取 → 全球专利检索 → 相似度分析 → 技术可行性评估
→ 侵权风险评估 → 创新点识别 → 改进建议 → 综合评分 → 生成报告
```

每一步的结果都可以查看和追溯，不是黑盒。

### 2. 多源专利检索

同时搜索多个数据源，自动级联回退，确保任何网络环境都能返回结果：

| 数据源 | 覆盖范围 | 国内直连 | 特点 |
|--------|---------|---------|------|
| SerpAPI | 全球专利 | 需 VPN | Google Patents 聚合 |
| Google Patents | 全球专利 | 需 VPN | 免费直查 |
| 本地数据库 | 已缓存数据 | 是 | 离线可用 |

### 3. AI 深度分析

- **专利摘要** -- 一键生成结构化摘要（技术领域、解决问题、核心方案）
- **方案对比** -- 并排对比多个技术方案，支持上传 PDF/DOCX/图片
- **侵权风险评估** -- 逐条分析权利要求，给出风险等级
- **AI 问答** -- 针对具体专利的多轮技术讨论

### 4. 实用功能

- **收藏与标签** -- 管理和组织检索到的专利
- **导出** -- 搜索结果导出为 CSV / Excel
- **PDF 导出** -- 专利详情（含技术附图）导出为 PDF
- **中英双语** -- 完整的中文/英文界面切换
- **AI 自动容灾** -- 支持 7+ 家 AI 服务商自动切换，一个挂了自动用下一个

---

## 快速开始

### 方式一：下载发布包（推荐）

1. 从 [GitHub Releases](https://github.com/jsshwqz/innoforge/releases) 或 [Gitee Releases](https://gitee.com/jsshwqz/innoforge/releases) 下载
2. 解压，运行 `start.bat`（Windows）或 `./start.sh`（Linux/macOS）
3. 打开 http://127.0.0.1:3000

### 方式二：源码编译

```bash
git clone https://github.com/jsshwqz/innoforge.git
# 或 Gitee：git clone https://gitee.com/jsshwqz/innoforge.git

cd innoforge
cp .env.example .env   # 编辑 .env 填入 API 密钥（可选）
cargo run --release --bin innoforge
```

需要 [Rust](https://rustup.rs/) 1.70+。

### 方式三：Docker

```bash
docker build -t innoforge .
docker run -p 3000:3000 -v innoforge-data:/data innoforge
```

### 方式四：Android

从 [Releases](https://github.com/jsshwqz/innoforge/releases) 下载 `innoforge-android.apk`。内嵌服务器，本地运行。

---

## 配置说明

### 终端契约（默认值）

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| 数据库 | `innoforge.db` | SQLite 数据库文件路径 |
| 服务地址 | `127.0.0.1:3000` | 本地监听地址（可用 `INNOFORGE_PORT` 覆盖） |

所有设置均可通过**设置页面**（http://localhost:3000/settings）在线配置，也可编辑 `.env` 文件。

**没有 API 密钥也能用** -- 检索可直接使用本地数据库。无法获得 CNIPR 授权时，建议导入公开公告数据包作为本地主链路（见下方指南）。

### AI 服务商（任选一个）

| 服务商 | 费用 | API 地址 | 备注 |
|--------|------|----------|------|
| 智谱 GLM（推荐） | 免费（glm-4.7-flash） | `https://open.bigmodel.cn/api/paas/v4` | 国内直连，注册即用 |
| DeepSeek | 低价 | `https://api.deepseek.com/v1` | 国内直连 |
| OpenRouter | 部分模型免费 | `https://openrouter.ai/api/v1` | 聚合多家模型 |
| Google Gemini | 15 次/分钟免费 | `https://generativelanguage.googleapis.com/v1beta/openai/` | 需 VPN |
| OpenAI | 付费 | `https://api.openai.com/v1` | 需 VPN |
| NVIDIA | 有免费额度 | `https://integrate.api.nvidia.com/v1` | |
| Ollama | 本地免费 | `http://localhost:11434/v1` | 需本地部署模型 |

### 专利检索 API（可选）

| 变量 | 说明 |
|------|------|
| `SERPAPI_KEY` | [SerpAPI](https://serpapi.com/) 密钥，100次/月免费 |
| `CNIPR_USER` / `CNIPR_PASSWORD` | 兼容保留（当前默认在线链路不依赖该配置） |

---

## API 概览

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/search` | POST | 本地专利搜索 |
| `/api/search/online` | POST | 在线多源检索 |
| `/api/search/export/xlsx` | POST | 导出搜索结果 |
| `/api/ai/summarize` | POST | AI 专利摘要 |
| `/api/ai/compare` | POST | AI 方案对比 |
| `/api/ai/risk` | POST | 侵权风险评估 |
| `/api/ai/chat` | POST | AI 技术问答 |
| `/api/idea/submit` | POST | 提交想法 |
| `/api/idea/pipeline` | POST | 启动 13 步验证流水线 |
| `/api/patent/enrich/:id` | GET | 加载专利全文 + 附图 |
| `/api/patent/pdf/:id` | GET | 导出为 PDF |
| `/api/settings` | GET/POST | 配置管理 |

完整 API 文档：[docs/API.md](docs/API.md)

---

## 项目结构

```
innoforge/
  src/
    main.rs               # Web 服务器入口
    lib.rs                # 库导出 + Android JNI / iOS FFI
    patent.rs             # 数据模型
    ai/                   # AI 客户端（多服务商容灾）
      client.rs           # HTTP 客户端 + 故障转移
      chat.rs             # 对话接口
      patent.rs           # 专利分析（摘要/对比/风险）
      idea.rs             # 创意验证
    db/                   # SQLite 数据库
      patent.rs           # 专利 CRUD + FTS5 全文搜索
      settings.rs         # 配置持久化
      migrations.rs       # 数据库迁移
    routes/               # API 路由处理器
    pipeline/             # 13 步创新验证流水线
      steps/
        deep_reasoning.rs # AI 深度推演
    bin/
      mcp-server.rs       # MCP 协议服务器
  templates/              # HTML 页面模板
  static/                 # 静态资源（CSS/JS）
```

**关联仓库：**
- [innoforge-desktop](https://gitee.com/jsshwqz/innoforge-desktop) -- Tauri 桌面端
- [innoforge-ios](https://gitee.com/jsshwqz/innoforge-ios) -- iOS 原生壳
- [innoforge-harmony](https://gitee.com/jsshwqz/innoforge-harmony) -- 鸿蒙原生壳

---

## 技术栈

- **后端**：Rust + Axum + SQLite（内嵌，零配置）
- **前端**：原生 HTML/CSS/JS（无需构建工具）
- **AI**：兼容任意 OpenAI API + 6 服务商自动容灾
- **搜索**：CNIPR + SerpAPI + Google Patents + 本地库（级联回退）
- **移动端**：Rust cdylib + JNI (Android) / FFI (iOS) + WebView
- **国际化**：中英双语

---

## 致谢

### 架构灵感

- [Harness Research](https://github.com/Nimo1987/harness-research) -- 验证流水线受其启发：状态机驱动的步骤链、LLM 与代码的职责分离、多层搜索降级。本项目所有实现均为原创。

### 使用的算法

- [TF-IDF](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) -- 文本相似度匹配
- [Jaccard 相似系数](https://en.wikipedia.org/wiki/Jaccard_index) -- 重叠检测与矛盾分析
- [余弦相似度](https://en.wikipedia.org/wiki/Cosine_similarity) -- 向量空间相似度

### 第三方服务

- [Chart.js](https://www.chartjs.org/) v4 (MIT) -- 图表可视化
- [SerpAPI](https://serpapi.com/) / [CNIPR](https://open.cnipr.com/) -- 专利数据源

---

## 许可证

[MIT](LICENSE)

---

## 贡献

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing`)
3. 提交更改
4. 发起 Pull Request

---

## 获取更新通知

- **GitHub Watch** -- 仓库右上角点击 **Watch → Custom → Releases**
- **Gitee 关注** -- [Gitee 仓库](https://gitee.com/jsshwqz/innoforge) 点击 **Watch**
- **RSS** -- `https://github.com/jsshwqz/innoforge/releases.atom`

---

<a name="english"></a>
## English

Most products don't fail at the implementation stage -- they fail much earlier: an idea looks promising but lacks supporting evidence; a direction seems viable but has no clear path; development kicks off only to hit existing patents, technical barriers, or IP protection gaps.

**InnoForge** helps developers and R&D teams turn initial ideas into informed decisions, actionable plans, shipped products, and protected intellectual property -- reducing blind investment and improving the conversion rate from concept to outcome.

### Core Features

- **13-Step Validation Pipeline** -- From idea to feasibility report, fully automated: keyword extraction, global patent search, similarity analysis, risk assessment, innovation scoring
- **Multi-Source Patent Search** -- CNIPR (China), SerpAPI (Google Patents), Google Patents Direct, Local DB -- cascading fallback ensures results in any network environment
- **AI Deep Analysis** -- Patent summarization, side-by-side comparison, infringement risk assessment, multi-round Q&A
- **Practical Tools** -- Collections & tags, CSV/Excel/PDF export, bilingual UI (Chinese/English)
- **AI Failover** -- 7+ AI providers with automatic failover (Zhipu GLM, DeepSeek, OpenRouter, Gemini, OpenAI, NVIDIA, Ollama)
- **Zero Config** -- Embedded SQLite + FTS5, no database setup needed

### Quick Start

```bash
# Download from GitHub Releases or Gitee Releases
# Extract, run start.bat (Windows) or ./start.sh (Linux/macOS)
# Open http://127.0.0.1:3000

# Or build from source:
git clone https://github.com/jsshwqz/innoforge.git
cd innoforge && cp .env.example .env
cargo run --release --bin innoforge
```

### Configuration

All settings configurable via **Settings page** (http://localhost:3000/settings) or `.env` file.

**No API keys?** App still works -- search uses local database, AI features show setup guide.

| Provider | Free Tier | Base URL |
|----------|-----------|----------|
| Zhipu GLM (Recommended) | Free (glm-4.7-flash) | `https://open.bigmodel.cn/api/paas/v4` |
| DeepSeek | Low cost | `https://api.deepseek.com/v1` |
| OpenRouter | Free models available | `https://openrouter.ai/api/v1` |
| Google Gemini | 15 RPM free | `https://generativelanguage.googleapis.com/v1beta/openai/` |
| OpenAI | Paid | `https://api.openai.com/v1` |
| NVIDIA | Free credits | `https://integrate.api.nvidia.com/v1` |
| Ollama | Local/Free | `http://localhost:11434/v1` |

### Tech Stack

- **Backend**: Rust + Axum + SQLite (embedded, zero-config)
- **Frontend**: Vanilla HTML/CSS/JS (no build tools)
- **AI**: Any OpenAI-compatible API + 6-provider automatic failover
- **Search**: CNIPR + SerpAPI + Google Patents + Local DB (cascade fallback)
- **Mobile**: Rust cdylib/staticlib + JNI (Android) / FFI (iOS)
- **i18n**: Chinese/English bilingual

### License

[MIT](LICENSE)
