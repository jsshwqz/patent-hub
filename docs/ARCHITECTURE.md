# Architecture / 架构设计

## Overview / 概述

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
│  │  - /api/patent/:id              │   │
│  │  - /api/compare                 │   │
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

## Core Modules / 核心模块

### 1. main.rs - Application Entry / 应用入口

- Server initialization / 服务器初始化
- Route registration / 路由注册
- Database setup / 数据库设置
- Static file serving / 静态文件服务

### 2. routes.rs - API Layer / API 层

HTTP endpoints:
- `GET /` - Home page / 首页
- `GET /search` - Search page / 搜索页面
- `POST /api/search` - Execute search / 执行搜索
- `GET /api/patent/:id` - Patent details / 专利详情
- `POST /api/ai/analyze` - AI analysis / AI 分析
- `POST /api/ai/compare` - Patent comparison / 专利对比
- `POST /api/export` - Export data / 导出数据

### 3. db.rs - Data Layer / 数据层

Database operations:
- `init_db()` - Initialize schema / 初始化数据库
- `save_patent()` - Save patent / 保存专利
- `search_patents()` - Local search / 本地搜索
- `get_patent()` - Get patent by ID / 获取专利
- `get_search_history()` - Search history / 搜索历史

Schema:
```sql
CREATE TABLE patents (
    id TEXT PRIMARY KEY,
    patent_id TEXT UNIQUE,
    title TEXT,
    abstract TEXT,
    -- ... more fields
);

CREATE TABLE search_history (
    id INTEGER PRIMARY KEY,
    query TEXT,
    timestamp DATETIME,
    result_count INTEGER
);
```

### 4. ai.rs - AI Integration / AI 集成

AI service abstraction:
- OpenAI-compatible API support / 兼容 OpenAI API
- Configurable base URL and model / 可配置 URL 和模型
- Retry logic / 重试逻辑
- Error handling / 错误处理

Supported providers:
- Ollama (local) / Ollama（本地）
- OpenAI
- DeepSeek
- Any OpenAI-compatible API / 任何兼容 OpenAI 的 API

### 5. patent.rs - Data Models / 数据模型

Core structures:
```rust
pub struct Patent {
    pub id: String,
    pub patent_id: String,
    pub title: String,
    pub abstract: Option<String>,
    pub applicant: Option<String>,
    pub inventor: Option<String>,
    pub filing_date: Option<String>,
    pub publication_date: Option<String>,
    pub country: Option<String>,
    pub url: Option<String>,
}

pub struct SearchRequest {
    pub query: String,
    pub mode: String,
    pub country: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}
```

## Cross-Platform Design / 跨平台设计

### File Paths / 文件路径

Use `std::path::PathBuf` for all paths:
```rust
use std::path::PathBuf;

let db_path = PathBuf::from("patent_hub.db");
```

### Environment Variables / 环境变量

Use `dotenvy` for .env file support:
```rust
dotenvy::dotenv().ok();
let api_key = std::env::var("AI_API_KEY")?;
```

### Network / 网络

Use `reqwest` with `rustls-tls` (no OpenSSL dependency):
```toml
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

### Database / 数据库

Use `rusqlite` with `bundled` feature (no system SQLite):
```toml
rusqlite = { version = "0.29", features = ["bundled"] }
```

## Deployment Options / 部署选项

### 1. Standalone Binary / 独立二进制

```bash
cargo build --release
./target/release/patent-hub
```

Pros:
- No dependencies / 无依赖
- Fast startup / 快速启动
- Easy distribution / 易于分发

### 2. Docker Container / Docker 容器

```bash
docker build -t patent-hub .
docker run -p 3000:3000 patent-hub
```

Pros:
- Consistent environment / 一致环境
- Easy scaling / 易于扩展
- Isolated / 隔离

### 3. System Service / 系统服务

- Windows: Task Scheduler / 任务计划程序
- macOS: LaunchAgent
- Linux: systemd

Pros:
- Auto-start / 自动启动
- Background running / 后台运行
- System integration / 系统集成

## Performance Considerations / 性能考虑

### Database / 数据库

- Use indexes on frequently queried fields / 常查询字段建索引
- Connection pooling (future) / 连接池（未来）
- Batch inserts / 批量插入

### Caching / 缓存

- In-memory cache for frequent queries / 内存缓存常用查询
- HTTP response caching / HTTP 响应缓存
- AI response caching / AI 响应缓存

### Async / 异步

- Tokio runtime for async I/O / Tokio 异步运行时
- Non-blocking database operations / 非阻塞数据库操作
- Concurrent API requests / 并发 API 请求

## Security / 安全

### API Keys / API 密钥

- Store in .env file / 存储在 .env 文件
- Never commit to git / 不提交到 git
- Use environment variables / 使用环境变量

### Input Validation / 输入验证

- Sanitize user input / 清理用户输入
- SQL injection prevention / SQL 注入防护
- XSS protection / XSS 防护

### CORS / 跨域

- Configured in tower-http / 在 tower-http 配置
- Restrict origins in production / 生产环境限制来源

## Future Enhancements / 未来增强

### Scalability / 可扩展性

- PostgreSQL support / PostgreSQL 支持
- Redis caching / Redis 缓存
- Load balancing / 负载均衡

### Features / 功能

- User authentication / 用户认证
- Multi-tenant support / 多租户支持
- Real-time updates / 实时更新
- GraphQL API

### Monitoring / 监控

- Metrics collection / 指标收集
- Logging / 日志
- Health checks / 健康检查
- Alerting / 告警
