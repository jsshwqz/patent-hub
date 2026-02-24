# 自动创建 GitHub Release 说明

## 📋 准备工作

### 1. 获取 GitHub Personal Access Token

1. 访问：https://github.com/settings/tokens
2. 点击 "Generate new token (classic)"
3. 设置 Token 名称：`patent-hub-release`
4. 勾选权限：
   - ✅ `repo` (完整的仓库访问权限)
5. 点击 "Generate token"
6. **立即复制 Token**（只显示一次）

### 2. 设置环境变量

#### Windows (PowerShell)
```powershell
$env:GITHUB_TOKEN = "ghp_your_token_here"
```

#### Linux/Mac (Bash)
```bash
export GITHUB_TOKEN="ghp_your_token_here"
```

### 3. 确认文件存在

确保以下文件存在：
- `patent-hub-v0.1.0-windows-x86_64.zip` (发布包)

---

## 🚀 执行方式

### 方式 1：PowerShell 脚本（Windows 推荐）

```powershell
# 1. 设置 Token
$env:GITHUB_TOKEN = "ghp_your_token_here"

# 2. 运行脚本
.\auto_create_release.ps1
```

### 方式 2：Bash 脚本（Linux/Mac）

```bash
# 1. 设置 Token
export GITHUB_TOKEN="ghp_your_token_here"

# 2. 添加执行权限
chmod +x auto_create_release.sh

# 3. 运行脚本
./auto_create_release.sh
```

### 方式 3：手动 API 调用

```bash
# 设置变量
GITHUB_TOKEN="ghp_your_token_here"
OWNER="jsshwqz"
REPO="patent-hub"
TAG="v0.1.0"

# 创建 Release
curl -X POST \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  -d '{"tag_name":"'$TAG'","name":"Patent Hub v0.1.0","body":"Release notes","draft":false}' \
  https://api.github.com/repos/$OWNER/$REPO/releases
```

---

## 📝 脚本功能

### auto_create_release.ps1 (PowerShell)

**功能**：
1. ✅ 检查 ZIP 文件是否存在
2. ✅ 检查 GITHUB_TOKEN 是否设置
3. ✅ 创建 GitHub Release
4. ✅ 上传 ZIP 文件
5. ✅ 显示下载链接

**输出示例**：
```
========================================
   自动创建 GitHub Release
========================================

仓库: jsshwqz/patent-hub
Tag: v0.1.0

✓ 找到发布包: patent-hub-v0.1.0-windows-x86_64.zip (3.42 MB)
✓ 检测到 GITHUB_TOKEN

正在创建 Release...
✓ Release 创建成功!

Release ID: 123456789
Release URL: https://github.com/jsshwqz/patent-hub/releases/tag/v0.1.0

正在上传 patent-hub-v0.1.0-windows-x86_64.zip...
✓ 文件上传成功!

文件名: patent-hub-v0.1.0-windows-x86_64.zip
文件大小: 3.42 MB
下载链接: https://github.com/jsshwqz/patent-hub/releases/download/v0.1.0/patent-hub-v0.1.0-windows-x86_64.zip

========================================
   Release 创建完成!
========================================

用户可以从以下地址下载:
https://github.com/jsshwqz/patent-hub/releases/tag/v0.1.0
```

---

## ⚠️ 常见问题

### Q1: 提示 "GITHUB_TOKEN 未设置"

**解决方案**：
```powershell
# 设置 Token
$env:GITHUB_TOKEN = "ghp_your_token_here"

# 验证是否设置成功
echo $env:GITHUB_TOKEN
```

### Q2: 提示 "Token 权限不足"

**原因**：Token 没有 `repo` 权限

**解决方案**：
1. 重新生成 Token
2. 确保勾选 `repo` 权限
3. 使用新的 Token

### Q3: 提示 "Release 已存在"

**原因**：Tag v0.1.0 的 Release 已经存在

**解决方案**：

#### 方案 1：删除旧 Release
```powershell
# 获取 Release ID
$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/jsshwqz/patent-hub/releases" -Headers @{"Authorization"="Bearer $env:GITHUB_TOKEN"}
$releaseId = ($releases | Where-Object {$_.tag_name -eq "v0.1.0"}).id

# 删除 Release
Invoke-RestMethod -Uri "https://api.github.com/repos/jsshwqz/patent-hub/releases/$releaseId" -Method Delete -Headers @{"Authorization"="Bearer $env:GITHUB_TOKEN"}

# 重新运行脚本
.\auto_create_release.ps1
```

#### 方案 2：使用新版本号
修改脚本中的 `$tag = "v0.1.1"`

### Q4: 提示 "ZIP 文件未找到"

**原因**：发布包不存在

**解决方案**：
```powershell
# 检查文件是否存在
Test-Path "patent-hub-v0.1.0-windows-x86_64.zip"

# 如果不存在，需要先构建和打包
cargo build --release
# 然后打包...
```

### Q5: 上传文件失败

**原因**：文件太大或网络问题

**解决方案**：
1. 检查网络连接
2. 检查文件大小（GitHub 限制单个文件 2GB）
3. 重试上传

---

## 🔍 验证 Release

### 1. 访问 Release 页面
```
https://github.com/jsshwqz/patent-hub/releases
```

### 2. 检查内容
- ✅ Tag: v0.1.0
- ✅ 标题: Patent Hub v0.1.0 - AI 智能专利检索分析系统
- ✅ 说明: 完整的 Release Notes
- ✅ 文件: patent-hub-v0.1.0-windows-x86_64.zip
- ✅ 标记为 "Latest"

### 3. 测试下载
```powershell
# 下载文件
Invoke-WebRequest -Uri "https://github.com/jsshwqz/patent-hub/releases/download/v0.1.0/patent-hub-v0.1.0-windows-x86_64.zip" -OutFile "test-download.zip"

# 检查文件大小
(Get-Item "test-download.zip").Length / 1MB
```

---

## 📊 Release 统计

创建 Release 后，可以查看：
- 下载次数
- Star 数量
- Fork 数量
- Issue 数量

访问：https://github.com/jsshwqz/patent-hub/releases

---

## 🔄 更新 Release

如果需要更新 Release 内容：

### 更新说明
```powershell
$releaseId = "123456789"  # 从创建输出获取
$newBody = "更新的说明内容"

Invoke-RestMethod -Uri "https://api.github.com/repos/jsshwqz/patent-hub/releases/$releaseId" `
  -Method Patch `
  -Headers @{"Authorization"="Bearer $env:GITHUB_TOKEN"} `
  -Body (@{"body"=$newBody} | ConvertTo-Json)
```

### 添加文件
```powershell
# 获取 upload_url
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/jsshwqz/patent-hub/releases/$releaseId" -Headers @{"Authorization"="Bearer $env:GITHUB_TOKEN"}
$uploadUrl = $release.upload_url -replace '\{\?name,label\}', "?name=new-file.zip"

# 上传新文件
$bytes = [System.IO.File]::ReadAllBytes("new-file.zip")
Invoke-RestMethod -Uri $uploadUrl -Method Post -Headers @{"Authorization"="Bearer $env:GITHUB_TOKEN";"Content-Type"="application/zip"} -Body $bytes
```

---

## 🎯 最佳实践

### 1. Token 安全
- ❌ 不要提交 Token 到代码仓库
- ❌ 不要分享 Token
- ✅ 使用环境变量
- ✅ 定期更换 Token

### 2. Release 命名
- ✅ 使用语义化版本：v0.1.0, v1.0.0
- ✅ 标题清晰明了
- ✅ 说明详细完整

### 3. 文件管理
- ✅ 文件名包含版本号
- ✅ 文件名包含平台信息
- ✅ 提供多平台版本

### 4. 发布流程
1. 更新 CHANGELOG.md
2. 更新版本号
3. 构建和测试
4. 打包发布文件
5. 创建 Release
6. 验证下载链接
7. 宣传推广

---

## 📞 获取帮助

如果遇到问题：
1. 查看 GitHub API 文档：https://docs.github.com/en/rest/releases
2. 查看脚本输出的错误信息
3. 提交 Issue：https://github.com/jsshwqz/patent-hub/issues

---

**文档版本**: 1.0  
**更新日期**: 2026-02-24
