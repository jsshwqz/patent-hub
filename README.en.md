# Patent Hub

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/your-username/patent-hub)

A patent search and analysis system built with Rust + Axum, supporting online search, AI analysis, patent comparison, and more.

English | [简体中文](README.md)

## Features

✅ **Online Patent Search** - Search Google Patents via SerpAPI
✅ **Search History** - Auto-save recent 10 searches
✅ **Advanced Filters** - Filter by date range, country/region
✅ **Statistics** - Top 10 applicants, country distribution, trend charts
✅ **Export** - Export to Excel (CSV format)
✅ **AI Analysis** - Patent summary, technical analysis
✅ **Patent Comparison** - AI-powered comparison of two patents
✅ **Similar Recommendations** - Keyword-based similar patent suggestions
✅ **File Comparison** - Upload TXT files to compare with patents

## Quick Start

### Installation

See [Installation Guide](docs/INSTALL.md) for detailed instructions.

#### Windows

```powershell
# Download latest release or build from source
git clone https://github.com/your-username/patent-hub.git
cd patent-hub
cargo build --release
.\target\release\patent-hub.exe
```

#### macOS / Linux

```bash
# Run installation script
./scripts/install-macos.sh  # macOS
./scripts/install-linux.sh  # Linux

# Or build manually
cargo build --release
./target/release/patent-hub
```

#### Docker

```bash
docker build -t patent-hub .
docker run -d -p 3000:3000 -v $(pwd)/.env:/app/.env patent-hub
```

### Configuration

Copy `.env.example` to `.env` and configure:

```env
# AI Service (Required)
AI_BASE_URL=http://localhost:11434/v1
AI_API_KEY=ollama
AI_MODEL=qwen2.5:7b

# SerpAPI (Optional, for online search)
SERPAPI_KEY=your-serpapi-key-here
```

### Access

Visit http://127.0.0.1:3000

## Usage

### Search Patents

1. Enter keywords (e.g., "coffee", "artificial intelligence") or applicant name
2. Choose search mode: Online (recommended) or Local database
3. Optional: Select country/region, set date range
4. Click "Search"

### View Patent Details

Click patent title in search results to view:
- Basic info (patent number, applicant, inventor, etc.)
- Abstract and claims
- AI analysis
- Similar patent recommendations
- File comparison

### Compare Patents

1. Go to "Patent Comparison" page
2. Enter two patent IDs or numbers
3. Click "Start Comparison"
4. AI will analyze similarities and differences

### Export Data

Click "Export Excel" button on search results page to download CSV.

## Tech Stack

- **Backend**: Rust + Axum 0.6
- **Database**: SQLite (rusqlite)
- **AI**: OpenAI-compatible API (Ollama, OpenAI, DeepSeek, etc.)
- **Search**: SerpAPI (Google Patents)
- **Frontend**: Native HTML + JavaScript

## Project Structure

```
patent-hub/
├── src/
│   ├── main.rs          # Main entry
│   ├── routes.rs        # API routes
│   ├── db.rs            # Database operations
│   ├── ai.rs            # AI service
│   └── patent.rs        # Data structures
├── templates/           # HTML templates
├── static/              # Static assets
├── scripts/             # Build & install scripts
├── docs/                # Documentation
└── Dockerfile           # Docker config
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Roadmap

- [ ] Multi-language UI support
- [ ] Advanced search syntax
- [ ] Patent portfolio analysis
- [ ] Citation network visualization
- [ ] **Native Mobile Apps** (Contributions welcome! See [MOBILE_APP.md](docs/MOBILE_APP.md))
  - [ ] Flutter version
  - [ ] React Native version
  - [ ] Native Android
  - [ ] Native iOS
  - [ ] Native HarmonyOS
- [ ] Browser extension
- [ ] PostgreSQL support
- [ ] User authentication

## FAQ

### Q: Server not accessible after reboot?
A: Server needs manual start. Use auto-start scripts or systemd service.

### Q: No search results?
A: Check:
   1. SERPAPI_KEY configured
   2. Network connection
   3. Try "Local database" mode

### Q: AI analysis failed?
A: Check:
   1. AI service running (e.g., Ollama)
   2. API key valid
   3. Network connection

### Q: How to stop server?
A: Press Ctrl+C in terminal

## Support

For issues or suggestions, please open an [Issue](../../issues).
