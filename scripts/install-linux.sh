#!/bin/bash
# Linux installation script

set -e

echo "Installing Patent Hub on Linux..."

# Detect distro
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    DISTRO="unknown"
fi

echo "Detected distro: $DISTRO"

# Install dependencies
case "$DISTRO" in
    ubuntu|debian)
        echo "Installing dependencies..."
        sudo apt update
        sudo apt install -y build-essential pkg-config libssl-dev
        ;;
    fedora|rhel|centos)
        echo "Installing dependencies..."
        sudo dnf install -y gcc pkg-config openssl-devel
        ;;
    arch|manjaro)
        echo "Installing dependencies..."
        sudo pacman -S --needed base-devel openssl
        ;;
    *)
        echo "Unknown distro. Please install: gcc, pkg-config, openssl-dev"
        ;;
esac

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
fi

# Check Ollama
if ! command -v ollama &> /dev/null; then
    echo "Ollama not found. Installing..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

# Build
echo "Building Patent Hub..."
cargo build --release

# Setup config
if [ ! -f .env ]; then
    cp .env.example .env
    echo "Created .env file. Please edit it with your API keys."
fi

# Create systemd service (optional)
read -p "Install as systemd service (auto-start on boot)? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    WORK_DIR="$(pwd)"
    SERVICE_FILE="/etc/systemd/system/patent-hub.service"
    
    sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=Patent Hub Service
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$WORK_DIR
ExecStart=$WORK_DIR/target/release/patent-hub
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
    
    sudo systemctl daemon-reload
    sudo systemctl enable patent-hub
    sudo systemctl start patent-hub
    
    echo "Systemd service installed and started."
    echo "Check status: sudo systemctl status patent-hub"
fi

echo ""
echo "Installation complete!"
echo "Run: ./target/release/patent-hub"
echo "Or: cargo run --release"
echo "Then visit: http://127.0.0.1:3000"
