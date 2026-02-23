# 快速开始指南 / Quick Start Guide

## 5 分钟上手 Patent Hub

### 第一步：启动服务器

**Windows:**
```
双击 "启动服务器.bat"
```

**macOS/Linux:**
```bash
./target/release/patent-hub
```

### 第二步：打开浏览器

服务器启动后会自动打开浏览器，或手动访问：
```
http://127.0.0.1:3000
```

### 第三步：开始搜索

1. 在搜索框输入关键词，例如：
   - "咖啡机"
   - "人工智能"
   - "王青芝"（申请人名称）

2. 选择搜索模式：
   - **在线搜索**（推荐）：搜索 Google Patents
   - **本地数据库**：搜索已保存的专利

3. 点击"检索"按钮

### 第四步：查看结果

- 点击专利标题查看详情
- 查看统计图表（申请人 TOP 10、国家分布等）
- 导出 Excel 数据

## 手机访问（可选）

### 1. 确保同一 WiFi

手机和电脑连接到同一个 WiFi 网络。

### 2. 查看 IP 地址

服务器启动时会显示：
```
Mobile access: http://192.168.1.100:3000
```

### 3. 手机浏览器访问

在手机浏览器输入显示的地址。

### 4. 添加到主屏幕

- **iOS**: Safari > 分享 > 添加到主屏幕
- **Android**: Chrome > 菜单 > 添加到主屏幕

详见 [移动设备访问指南](MOBILE_ACCESS.md)

## 常用功能

### 搜索专利

```
关键词: 人工智能
国家: CN (中国)
日期: 2020-01-01 到 2024-12-31
```

### AI 分析

1. 打开专利详情页
2. 点击"AI 分析"按钮
3. 查看技术分析、创新点等

### 专利对比

1. 访问"专利对比"页面
2. 输入两个专利 ID
3. 点击"开始对比"
4. 查看 AI 对比报告

### 相似推荐

在专利详情页自动显示相似专利。

### 文件对比

1. 打开专利详情页
2. 上传 TXT 文件
3. 点击"对比"
4. 查看 AI 对比结果

## 配置 API 密钥

### SerpAPI（在线搜索）

1. 访问 https://serpapi.com/ 注册
2. 获取 API Key
3. 编辑 `.env` 文件：
   ```env
   SERPAPI_KEY=your-key-here
   ```

### AI 服务

#### 选项 1：Ollama（免费本地）

```bash
# 安装 Ollama
# Windows: https://ollama.com/download
# macOS: brew install ollama
# Linux: curl -fsSL https://ollama.com/install.sh | sh

# 下载模型
ollama pull qwen2.5:7b
```

配置 `.env`:
```env
AI_BASE_URL=http://localhost:11434/v1
AI_API_KEY=ollama
AI_MODEL=qwen2.5:7b
```

#### 选项 2：智谱 GLM（免费在线）

1. 访问 https://open.bigmodel.cn/ 注册
2. 获取 API Key
3. 配置 `.env`:
   ```env
   AI_BASE_URL=https://open.bigmodel.cn/api/paas/v4
   AI_API_KEY=your-key-here
   AI_MODEL=glm-4-flash
   ```

#### 选项 3：OpenAI

```env
AI_BASE_URL=https://api.openai.com/v1
AI_API_KEY=sk-your-key
AI_MODEL=gpt-4o
```

## 故障排除

### 问题 1：无法启动

**检查：**
- 端口 3000 是否被占用
- 是否有 `.env` 文件

**解决：**
```bash
# 检查端口
netstat -ano | findstr :3000

# 创建配置文件
copy .env.example .env
```

### 问题 2：搜索无结果

**检查：**
- SERPAPI_KEY 是否配置
- 网络连接是否正常

**解决：**
- 配置 SerpAPI 密钥
- 或使用"本地数据库"模式

### 问题 3：AI 分析失败

**检查：**
- AI 服务是否运行（Ollama）
- API Key 是否正确

**解决：**
```bash
# 启动 Ollama
ollama serve

# 测试连接
curl http://localhost:11434/api/tags
```

### 问题 4：手机无法访问

**检查：**
- 是否同一 WiFi
- 防火墙是否阻止

**解决：**
```powershell
# Windows 允许端口
netsh advfirewall firewall add rule name="Patent Hub" dir=in action=allow protocol=TCP localport=3000
```

## 下一步

- 📖 阅读 [完整文档](../README.md)
- 🔧 查看 [安装指南](INSTALL.md)
- 📱 配置 [移动访问](MOBILE_ACCESS.md)
- 🏗️ 了解 [架构设计](ARCHITECTURE.md)
- 🔌 查看 [API 文档](API.md)

## 获取帮助

- [GitHub Issues](../../issues)
- [贡献指南](../CONTRIBUTING.md)
- [常见问题](../README.md#常见问题)

## 视频教程（计划中）

- [ ] 基础使用教程
- [ ] 移动设备配置
- [ ] AI 功能演示
- [ ] 高级搜索技巧
