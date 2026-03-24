# Patent Hub

[![CI](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/jsshwqz/patent-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/jsshwqz/patent-hub)](https://github.com/jsshwqz/patent-hub/releases)

> **Patent Hub** -- Patent search, AI analysis, and management platform with multi-language support.

[English](#english) | [Gitee Mirror](https://gitee.com/jsshwqz/patent-hub)

---

## Features / Overview

- **Patent Search** -- Global patent search with relevance scoring, deduplication, and category statistics
- **AI Analysis** -- AI-powered patent summarization, Q&A, and idea validation with multi-round dialogue
- **Patent Comparison** -- Side-by-side patent comparison with file upload support (PDF, DOCX, images)
- **Patent Drawings** -- View patent technical drawings with local image proxy
- **PDF Export** -- Export patent details (with drawings) to PDF
- **Collections & Tags** -- Organize patents with collections and tags
- **AI Failover** -- Automatic failover across multiple AI providers (Zhipu GLM, OpenRouter, Gemini, OpenAI, NVIDIA, DeepSeek)
- **i18n** -- Full Chinese/English bilingual support
- **Zero Config Database** -- Embedded SQLite with FTS5 full-text search, no installation needed

---

## Quick Start / Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)

### Run from Source

```bash
git clone https://github.com/jsshwqz/patent-hub.git
cd patent-hub
cp .env.example .env
# Edit .env with your API keys (optional)
cargo run --release --bin patent-hub
```

### Run from Release Package

1. Download from [Releases](https://github.com/jsshwqz/patent-hub/releases)
2. Extract the archive
3. Run `start.bat` (Windows) or `./start.sh` (Linux/macOS)
4. Open http://127.0.0.1:3000/search

### Docker

```bash
docker build -t patent-hub .
docker run -p 3000:3000 -v patent-data:/data patent-hub
```

---

## Configuration

All settings can be configured via the **Settings page** (http://localhost:3000/settings) or `.env` file.

| Variable | Required | Description |
|----------|----------|-------------|
| `AI_BASE_URL` | For AI features | Any OpenAI-compatible API endpoint |
| `AI_API_KEY` | For AI features | API key for AI service |
| `AI_MODEL` | For AI features | Model name (e.g., `glm-4-flash`) |
| `SERPAPI_KEY` | For online search | [SerpAPI](https://serpapi.com/) key for Google Patents |
| `FALLBACK_AI_*` | Optional | Up to 5 backup AI providers |
| `HOST` | Optional | Server bind address (default: `0.0.0.0`) |
| `PORT` | Optional | Server port (default: `3000`) |

**No API keys?** The app still works -- search uses local database, AI features show helpful messages.

### Supported AI Providers

| Provider | Free Tier | Base URL |
|----------|-----------|----------|
| Zhipu GLM | 5 RPM | `https://open.bigmodel.cn/api/paas/v4` |
| OpenRouter | Free models | `https://openrouter.ai/api/v1` |
| Google Gemini | 15 RPM | `https://generativelanguage.googleapis.com/v1beta/openai/` |
| DeepSeek | Low cost | `https://api.deepseek.com/v1` |
| OpenAI | Paid | `https://api.openai.com/v1` |
| NVIDIA | Free tier | `https://integrate.api.nvidia.com/v1` |
| Ollama | Local/Free | `http://localhost:11434/v1` |

---

## API Overview

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

---

## Project Structure

```
patent-hub/
  src/
    main.rs          # Web server entry point
    lib.rs           # Library exports
    ai.rs            # AI client with failover
    db.rs            # SQLite database with FTS5
    patent.rs        # Data models
    routes/          # API route handlers
    bin/
      skill-router.rs  # Standalone CLI tool
  templates/         # HTML templates (7 pages)
  static/            # CSS, JS (i18n)
  tests/             # Integration tests (39 tests)
  docs/              # Documentation
```

---

## Tech Stack

- **Backend**: Rust + Axum + SQLite (embedded, zero-config)
- **Frontend**: Vanilla HTML/CSS/JS (no build tools needed)
- **AI**: Any OpenAI-compatible API with automatic failover
- **Search**: SQLite FTS5 + SerpAPI + Google Patents
- **i18n**: Shared JS translation system

---

## License

[MIT](LICENSE)

---

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing`)
3. Commit your changes
4. Push to the branch
5. Open a Pull Request
