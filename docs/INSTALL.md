# 安装指南 / Installation Guide

## Windows

### 方式 1：预编译版本（推荐）

1. 下载最新 Release 的 `patent-hub-windows.zip`
2. 解压到任意目录
3. 复制 `.env.example` 为 `.env` 并配置 API 密钥
4. 双击 `启动服务器.bat` 或运行 `patent-hub.exe`

### 方式 2：从源码编译

```powershell
# 安装 Rust
# 访问 https://rustup.rs/ 下载安装

# 克隆仓库
git clone https://github.com/your-username/patent-hub.git
cd patent-hub

# 配置环境变量
copy .env.example .env
# 编辑 .env 文件配置 API 密钥

# 编译运行
cargo build --release
.\target\release\patent-hub.exe
```

## macOS

### 方式 1：Homebrew（计划中）

```bash
# 待发布到 Homebrew
brew install patent-hub
```

### 方式 2：从源码编译

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆仓库
git clone https://github.com/your-username/patent-hub.git
cd patent-hub

# 配置环境变量
cp .env.example .env
# 编辑 .env 文件配置 API 密钥

# 编译运行
cargo build --release
./target/release/patent-hub
```

## Linux

### Ubuntu/Debian

```bash
# 安装依赖
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 克隆仓库
git clone https://github.com/your-username/patent-hub.git
cd patent-hub

# 配置环境变量
cp .env.example .env
# 编辑 .env 文件配置 API 密钥

# 编译运行
cargo build --release
./target/release/patent-hub
```

### Fedora/RHEL

```bash
# 安装依赖
sudo dnf install gcc pkg-config openssl-devel

# 其余步骤同 Ubuntu
```

### Arch Linux

```bash
# 安装依赖
sudo pacman -S base-devel openssl

# 其余步骤同 Ubuntu
```

## Docker（推荐用于服务器部署）

```bash
# 构建镜像
docker build -t patent-hub .

# 运行容器
docker run -d \
  -p 3000:3000 \
  -v $(pwd)/.env:/app/.env \
  -v $(pwd)/patent_hub.db:/app/patent_hub.db \
  --name patent-hub \
  patent-hub
```

## 配置说明

### 必需配置

编辑 `.env` 文件：

```env
# AI 服务（必需）
AI_BASE_URL=http://localhost:11434/v1  # Ollama 本地
AI_API_KEY=ollama
AI_MODEL=qwen2.5:7b

# 在线搜索（可选）
SERPAPI_KEY=your-serpapi-key-here
```

### AI 服务选项

#### 1. Ollama（推荐，免费本地）

```bash
# 安装 Ollama
# Windows: https://ollama.com/download
# macOS: brew install ollama
# Linux: curl -fsSL https://ollama.com/install.sh | sh

# 下载模型
ollama pull qwen2.5:7b
```

配置：
```env
AI_BASE_URL=http://localhost:11434/v1
AI_API_KEY=ollama
AI_MODEL=qwen2.5:7b
```

#### 2. OpenAI

```env
AI_BASE_URL=https://api.openai.com/v1
AI_API_KEY=sk-your-key
AI_MODEL=gpt-4o
```

#### 3. 其他兼容 API

```env
AI_BASE_URL=https://api.deepseek.com/v1
AI_API_KEY=your-key
AI_MODEL=deepseek-chat
```

### SerpAPI（可选）

用于在线搜索 Google Patents：
1. 访问 https://serpapi.com/ 注册
2. 获取 API Key
3. 配置到 `.env` 文件

## 验证安装

访问 http://127.0.0.1:3000 应该看到搜索界面。

## 故障排除

### 端口被占用

修改 `src/main.rs` 中的端口号：
```rust
let addr = SocketAddr::from(([127, 0, 0, 1], 3000)); // 改为其他端口
```

### 数据库错误

删除 `patent_hub.db` 文件，程序会自动重建。

### AI 连接失败

检查：
1. AI 服务是否运行（如 Ollama）
2. API Key 是否正确
3. 网络连接是否正常

## 开机自启动

### Windows

运行 `安装开机自启动.bat`

### macOS

创建 LaunchAgent：
```bash
# 待补充
```

### Linux (systemd)

创建服务文件 `/etc/systemd/system/patent-hub.service`：
```ini
[Unit]
Description=Patent Hub Service
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/path/to/patent-hub
ExecStart=/path/to/patent-hub/target/release/patent-hub
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

启用服务：
```bash
sudo systemctl enable patent-hub
sudo systemctl start patent-hub
```
