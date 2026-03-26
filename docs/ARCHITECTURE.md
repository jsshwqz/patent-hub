# 架构设计 / Architecture

## 概述 / Overview

Patent Hub 采用经典的三层架构设计，注重跨平台兼容性和可扩展性。

```
┌─────────────────────────────────────────┐
│         Frontend (HTML/JS)              │
│  - Search UI                            │
│  - Patent Detail                        │
│  - Comparison                           │
└─────────────────┬───────────────────────┘
                  │ HTTP/JSON
┌─────────────────▼───────────────────────┐
│      Backend (Rust + Axum)              │
│  ┌─────────────────────────────────┐   │
│  │  Routes Layer                   │   │
│  │  - /api/search                  │   │
│  │  - /patent/:id                  │   │
│  │  - /api/ai/compare              │   │
│  └──────────┬──────────────────────┘   │
│             │                           │
│  ┌──────────▼──────────────────────┐   │
│  │  Business Logic                 │   │
│  │  - Search orchestration         │   │
│  │  - AI integration               │   │
│  │  - Data processing              │   │
│  └──────┬──────────────┬───────────┘   │
│         │              │                │
│  ┌──────▼──────┐  ┌───▼────────────┐   │
│  │  Database   │  │  External APIs │   │
│  │  (SQLite)   │  │  - SerpAPI     │   │
│  │             │  │  - AI Service  │   │
│  └─────────────┘  └────────────────┘   │
└─────────────────────────────────────────┘
```

## 核心模块 / Core Modules

### 1. main.rs - 应用入口 / Application Entry

- 服务器初始化 / Server initialization
- 路由注册 / Route registration
- 数据库设置 / Database setup
- 静态文件服务 / Static file serving

### 2. routes.rs - API 层 / API Layer

HTTP endpoints:
- `GET /` - 首页 / Home page
- `GET /search` - 搜索页面 / Search page
- `POST /api/search` - 执行搜索 / Execute search
- `POST /api/search/online` - 在线搜索 / Online search
- `POST /api/search/stats` - 统计分析 / Search stats
- `POST /api/search/export` - 导出 CSV / Export CSV
- `POST /api/search/analyze` - AI 分析检索结果 / AI analyze search results
- `GET /patent/:id` - 专利详情页 / Patent detail page
- `POST /api/ai/chat` - AI 对话 / AI chat
- `POST /api/ai/summarize` - AI 摘要 / AI summarize
- `POST /api/ai/compare` - 专利对比 / Patent comparison
- `POST /api/patent/fetch` - 抓取专利 / Fetch patent by number
- `POST /api/patents/import` - 批量导入 / Import patents
- `GET /api/patent/enrich/:id` - 丰富专利信息 / Enrich patent
- `GET /api/patent/similar/:id` - 相似推荐 / Similar recommendations
- `POST /api/upload/compare` - 上传文件对比 / Upload file compare
- `GET /api/settings` - 获取设置 / Get settings
- `POST /api/settings/serpapi` - 保存 SerpAPI / Save SerpAPI key
- `POST /api/settings/ai` - 保存 AI 配置 / Save AI config

### 3. db.rs - 数据层 / Data Layer

Database operations:
- `init()` - 初始化数据库 / Initialize schema
- `insert_patent()` - 保存专利 / Save patent
- `search_smart()` - 智能本地搜索 / Smart local search
- `search_like()` - 模糊搜索 / Fallback local search
- `search_fts()` - 全文搜索 / Full-text search
- `get_patent()` - 获取专利 / Get patent by ID

Schema:
```sql
CREATE TABLE patents (
    id TEXT PRIMARY KEY,
    patent_number TEXT NOT NULL,
    title TEXT,
    abstract_text TEXT,
    -- ... more fields
);
```

### 4. ai.rs - AI 集成 / AI Integration

AI service abstraction:
- 兼容 OpenAI API / OpenAI-compatible API support
- 可配置 URL 和模型 / Configurable base URL and model
- 重试逻辑 / Retry logic
- 错误处理 / Error handling

Supported providers:
- Ollama（本地） / Ollama (local)
- OpenAI
- DeepSeek
- 任何兼容 OpenAI 的 API / Any OpenAI-compatible API

### 5. patent.rs - 数据模型 / Data Models

Core structures:
```rust
pub struct Patent {
    pub id: String,
    pub patent_number: String,
    pub title: String,
    pub abstract_text: String,
    pub applicant: String,
    pub inventor: String,
    pub filing_date: String,
    pub publication_date: String,
    pub country: String,
}

pub struct SearchRequest {
    pub query: String,
    pub page: usize,
    pub page_size: usize,
    pub country: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_type: Option<String>,
    pub sort_by: Option<String>,
}
```

## 跨平台设计 / Cross-Platform Design

### 文件路径 / File Paths

Use `std::path::PathBuf` for all paths:
```rust
use std::path::PathBuf;

let db_path = PathBuf::from("patent_hub.db");
```

### 环境变量 / Environment Variables

Use `dotenvy` for .env file support:
```rust
dotenvy::dotenv().ok();
let api_key = std::env::var("AI_API_KEY")?;
```

### 网络 / Network

Use `reqwest` with `rustls-tls` (no OpenSSL dependency):
```toml
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

### 数据库 / Database

Use `rusqlite` with `bundled` feature (no system SQLite):
```toml
rusqlite = { version = "0.29", features = ["bundled"] }
```

## 部署选项 / Deployment Options

### 1. 独立二进制 / Standalone Binary

```bash
cargo build --release
./target/release/patent-hub
```

Pros:
- 无依赖 / No dependencies
- 快速启动 / Fast startup
- 易于分发 / Easy distribution

### 2. Docker 容器 / Docker Container

```bash
docker build -t patent-hub .
docker run -p 3000:3000 patent-hub
```

Pros:
- 一致环境 / Consistent environment
- 易于扩展 / Easy scaling
- 隔离 / Isolated

### 3. 系统服务 / System Service

- Windows: 任务计划程序 / Task Scheduler
- macOS: LaunchAgent
- Linux: systemd

Pros:
- 自动启动 / Auto-start
- 后台运行 / Background running
- 系统集成 / System integration

## 性能考虑 / Performance Considerations

### 数据库 / Database

- 常查询字段建索引 / Use indexes on frequently queried fields
- 连接池（未来） / Connection pooling (future)
- 批量插入 / Batch inserts

### 缓存 / Caching

- 内存缓存常用查询 / In-memory cache for frequent queries
- HTTP 响应缓存 / HTTP response caching
- AI 响应缓存 / AI response caching

### 异步 / Async

- Tokio 异步运行时 / Tokio runtime for async I/O
- 非阻塞数据库操作 / Non-blocking database operations
- 并发 API 请求 / Concurrent API requests

## 安全 / Security

### API 密钥 / API Keys

- 存储在 .env 文件 / Store in .env file
- 不提交到 git / Never commit to git
- 使用环境变量 / Use environment variables

### 输入验证 / Input Validation

- 清理用户输入 / Sanitize user input
- SQL 注入防护 / SQL injection prevention
- XSS 防护 / XSS protection

### 跨域 / CORS

- 在 tower-http 配置 / Configured in tower-http
- 生产环境限制来源 / Restrict origins in production

## 未来增强 / Future Enhancements

### 可扩展性 / Scalability

- PostgreSQL 支持 / PostgreSQL support
- Redis 缓存 / Redis caching
- 负载均衡 / Load balancing

### 功能 / Features

- 用户认证 / User authentication
- 多租户支持 / Multi-tenant support
- 实时更新 / Real-time updates
- GraphQL API

### 监控 / Monitoring

- 指标收集 / Metrics collection
- 日志 / Logging
- 健康检查 / Health checks
- 告警 / Alerting
