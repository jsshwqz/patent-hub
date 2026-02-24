# 自动创建 GitHub Release
# 使用 GitHub REST API

$ErrorActionPreference = "Stop"

$owner = "jsshwqz"
$repo = "patent-hub"
$tag = "v0.1.0"
$name = "Patent Hub v0.1.0 - AI 智能专利检索分析系统"

# Release 说明
$body = @"
## 🎉 Patent Hub v0.1.0 - 首个公开发布版本

### ✨ 核心特性

#### 🔍 专利检索
- **在线搜索** - 接入 Google Patents，覆盖全球专利数据库
- **本地数据库** - SQLite 存储，快速访问历史数据
- **高级筛选** - 按日期范围、国家/地区筛选
- **全文搜索** - 支持关键词全文检索

#### 🤖 AI 智能分析
- **AI 分析** - 智能分析专利技术要点、创新性、应用前景
- **专利对比** - AI 智能对比两个专利的技术差异
- **文件对比** - 上传技术方案文件与专利对比（独有功能）
- **相似推荐** - 智能推荐相关专利

#### 📊 数据分析
- **统计图表** - 申请人 TOP 10、国家分布、申请趋势图
- **Excel 导出** - 一键导出分析结果，无限制
- **搜索历史** - 自动保存搜索记录

#### 🌐 多端访问
- **跨平台** - Windows/Linux/macOS
- **移动设备** - 支持手机/平板访问
- **响应式界面** - 自适应各种屏幕尺寸

---

### 🇨🇳 国内用户友好

#### 无需代理即可使用的功能
- ✅ **AI 分析** - 使用智谱 GLM，国内直连
- ✅ **专利对比** - 使用智谱 GLM，国内直连
- ✅ **文件对比** - 使用智谱 GLM，国内直连
- ✅ **本地数据** - SQLite 本地存储
- ✅ **统计图表** - 本地计算和展示
- ✅ **数据导出** - 本地生成 Excel

#### 需要代理的功能
- ⚠️ **在线搜索** - 访问 Google Patents（可使用代理）

---

### 🚀 快速开始

#### 1. 下载
下载 ``patent-hub-v0.1.0-windows-x86_64.zip``

#### 2. 解压
解压到任意目录

#### 3. 配置
```bash
# 复制配置文件
copy .env.example .env

# 编辑 .env 文件，填入 API 密钥
# 推荐使用智谱 GLM（国内可用）
AI_API_KEY=你的智谱GLM密钥
AI_API_BASE=https://open.bigmodel.cn/api/paas/v4
AI_MODEL=glm-4-flash
```

#### 4. 启动
```bash
# Windows
start.bat

# 或直接运行
patent-hub.exe
```

#### 5. 访问
浏览器打开: http://localhost:3000

---

### 🔑 API 密钥获取

#### 智谱 GLM（推荐 - 国内可用）
1. 访问：https://open.bigmodel.cn/
2. 注册账号（支持手机号）
3. 创建 API Key
4. 免费额度充足

**优势**：
- ✅ 国内直连，速度快
- ✅ 中文理解能力强
- ✅ 价格便宜
- ✅ 免费额度充足

#### SerpAPI（可选 - 需要代理）
1. 访问：https://serpapi.com/
2. 注册账号
3. 获取 API Key
4. 免费额度：100 次/月

---

### 💻 系统要求

- **操作系统**: Windows 10/11, Linux, macOS
- **内存**: 最低 512MB，推荐 1GB+
- **磁盘空间**: 100MB+
- **网络**: 需要互联网连接（AI 功能和在线搜索）

---

### 📦 包含内容

- ``patent-hub.exe`` - 主程序 (7.61 MB)
- ``templates/`` - HTML 模板文件
- ``static/`` - 静态资源 (CSS/JS/图片)
- ``.env.example`` - 环境变量配置示例
- ``README.md`` - 项目说明文档
- ``LICENSE`` - MIT 开源协议
- ``start.bat`` - 一键启动脚本
- ``启动说明.txt`` - 使用指南

---

### 🛠️ 技术栈

- **后端**: Rust + Axum Web 框架
- **数据库**: SQLite
- **模板引擎**: Tera
- **前端**: 响应式 HTML/CSS/JavaScript
- **AI 集成**: OpenAI 兼容接口（支持智谱 GLM）
- **搜索 API**: SerpAPI

---

### 📚 文档

- [安装指南](https://github.com/jsshwqz/patent-hub/blob/main/docs/INSTALL.md)
- [快速开始](https://github.com/jsshwqz/patent-hub/blob/main/docs/QUICK_START.md)
- [国内用户指南](https://github.com/jsshwqz/patent-hub/blob/main/docs/国内用户指南.md)
- [API 文档](https://github.com/jsshwqz/patent-hub/blob/main/docs/API.md)
- [架构说明](https://github.com/jsshwqz/patent-hub/blob/main/docs/ARCHITECTURE.md)

---

### 🐛 已知问题

- 首次启动可能需要几秒钟初始化数据库
- AI 分析速度取决于 API 响应时间
- 在线搜索功能在国内需要代理

---

### 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

- **GitHub Issues**: https://github.com/jsshwqz/patent-hub/issues
- **贡献指南**: https://github.com/jsshwqz/patent-hub/blob/main/CONTRIBUTING.md

---

### 📄 许可证

MIT License - 详见 [LICENSE](https://github.com/jsshwqz/patent-hub/blob/main/LICENSE)

---

### 🙏 致谢

- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Axum](https://github.com/tokio-rs/axum) - Web 框架
- [SerpAPI](https://serpapi.com/) - 搜索 API
- [智谱 AI](https://open.bigmodel.cn/) - AI 模型
- [Chart.js](https://www.chartjs.org/) - 图表库

---

**发布日期**: 2026-02-24  
**版本**: v0.1.0  
**构建**: Release

**Made with ❤️ by Patent Hub Team**
"@

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   自动创建 GitHub Release" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "仓库: $owner/$repo" -ForegroundColor Yellow
Write-Host "Tag: $tag" -ForegroundColor Yellow
Write-Host "标题: $name" -ForegroundColor Yellow
Write-Host ""

# 检查 ZIP 文件
$zipFile = "patent-hub-v0.1.0-windows-x86_64.zip"
if (-not (Test-Path $zipFile)) {
    Write-Host "错误: 未找到 $zipFile" -ForegroundColor Red
    Write-Host "请先构建并打包项目" -ForegroundColor Yellow
    exit 1
}

$zipSize = [math]::Round((Get-Item $zipFile).Length / 1MB, 2)
Write-Host "✓ 找到发布包: $zipFile ($zipSize MB)" -ForegroundColor Green
Write-Host ""

# 检查是否有 GITHUB_TOKEN
$token = $env:GITHUB_TOKEN
if (-not $token) {
    Write-Host "错误: 未设置 GITHUB_TOKEN 环境变量" -ForegroundColor Red
    Write-Host ""
    Write-Host "请设置 GitHub Personal Access Token:" -ForegroundColor Yellow
    Write-Host '  $env:GITHUB_TOKEN = "ghp_your_token_here"' -ForegroundColor Cyan
    Write-Host ""
    Write-Host "获取 Token:" -ForegroundColor Yellow
    Write-Host "1. 访问: https://github.com/settings/tokens" -ForegroundColor Cyan
    Write-Host "2. 点击 'Generate new token (classic)'" -ForegroundColor Cyan
    Write-Host "3. 勾选 'repo' 权限" -ForegroundColor Cyan
    Write-Host "4. 生成并复制 Token" -ForegroundColor Cyan
    Write-Host ""
    exit 1
}

Write-Host "✓ 检测到 GITHUB_TOKEN" -ForegroundColor Green
Write-Host ""

# 创建 Release
Write-Host "正在创建 Release..." -ForegroundColor Yellow

$headers = @{
    "Authorization" = "Bearer $token"
    "Accept" = "application/vnd.github+json"
    "X-GitHub-Api-Version" = "2022-11-28"
}

$releaseData = @{
    tag_name = $tag
    name = $name
    body = $body
    draft = $false
    prerelease = $false
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$owner/$repo/releases" `
        -Method Post `
        -Headers $headers `
        -Body $releaseData `
        -ContentType "application/json"
    
    Write-Host "✓ Release 创建成功!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Release ID: $($response.id)" -ForegroundColor Cyan
    Write-Host "Release URL: $($response.html_url)" -ForegroundColor Cyan
    Write-Host ""
    
    # 上传 ZIP 文件
    Write-Host "正在上传 $zipFile..." -ForegroundColor Yellow
    
    $uploadUrl = $response.upload_url -replace '\{\?name,label\}', "?name=$zipFile"
    $zipBytes = [System.IO.File]::ReadAllBytes((Resolve-Path $zipFile))
    
    $uploadHeaders = @{
        "Authorization" = "Bearer $token"
        "Content-Type" = "application/zip"
        "Accept" = "application/vnd.github+json"
    }
    
    $uploadResponse = Invoke-RestMethod -Uri $uploadUrl `
        -Method Post `
        -Headers $uploadHeaders `
        -Body $zipBytes
    
    Write-Host "✓ 文件上传成功!" -ForegroundColor Green
    Write-Host ""
    Write-Host "文件名: $($uploadResponse.name)" -ForegroundColor Cyan
    Write-Host "文件大小: $([math]::Round($uploadResponse.size / 1MB, 2)) MB" -ForegroundColor Cyan
    Write-Host "下载链接: $($uploadResponse.browser_download_url)" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "   Release 创建完成!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "用户可以从以下地址下载:" -ForegroundColor Yellow
    Write-Host $response.html_url -ForegroundColor Cyan
    Write-Host ""
    
} catch {
    Write-Host "错误: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "错误详情:" -ForegroundColor Yellow
    Write-Host $_.Exception.Message -ForegroundColor Red
    
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host $responseBody -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "可能的原因:" -ForegroundColor Yellow
    Write-Host "1. Token 权限不足（需要 'repo' 权限）" -ForegroundColor Cyan
    Write-Host "2. Release 已存在（需要先删除旧的 Release）" -ForegroundColor Cyan
    Write-Host "3. 网络连接问题" -ForegroundColor Cyan
    Write-Host ""
    exit 1
}
