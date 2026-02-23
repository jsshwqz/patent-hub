#!/bin/bash
# macOS installation script

set -e

echo "Installing Patent Hub on macOS..."

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
fi

# Check Ollama
if ! command -v ollama &> /dev/null; then
    echo "Ollama not found. Install it from https://ollama.com/download"
    echo "Or run: brew install ollama"
fi

# Build
echo "Building Patent Hub..."
cargo build --release

# Setup config
if [ ! -f .env ]; then
    cp .env.example .env
    echo "Created .env file. Please edit it with your API keys."
fi

# Create LaunchAgent for auto-start (optional)
read -p "Install as LaunchAgent (auto-start on login)? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    PLIST_PATH="$HOME/Library/LaunchAgents/com.patent-hub.plist"
    WORK_DIR="$(pwd)"
    
    cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.patent-hub</string>
    <key>ProgramArguments</key>
    <array>
        <string>${WORK_DIR}/target/release/patent-hub</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${WORK_DIR}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${WORK_DIR}/patent-hub.log</string>
    <key>StandardErrorPath</key>
    <string>${WORK_DIR}/patent-hub.error.log</string>
</dict>
</plist>
EOF
    
    launchctl load "$PLIST_PATH"
    echo "LaunchAgent installed. Patent Hub will start on login."
fi

echo ""
echo "Installation complete!"
echo "Run: ./target/release/patent-hub"
echo "Or: cargo run --release"
echo "Then visit: http://127.0.0.1:3000"
