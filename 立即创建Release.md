# 🚀 立即创建 GitHub Release

## 快速步骤（3 步完成）

### 步骤 1：获取 GitHub Token

1. 访问：https://github.com/settings/tokens
2. 点击 "Generate new token (classic)"
3. 勾选 `repo` 权限
4. 生成并复制 Token（格式：`ghp_xxxxxxxxxxxx`）

### 步骤 2：设置 Token

在 PowerShell 中运行：
```powershell
$env:GITHUB_TOKEN = "ghp_你的Token"
```

### 步骤 3：运行脚本

```powershell
.\auto_create_release.ps1
```

---

## 完整示例

```powershell
# 1. 设置 Token（替换为你的实际 Token）
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

# 2. 验证 Token 已设置
echo $env:GITHUB_TOKEN

# 3. 运行脚本
.\auto_create_release.ps1
```

---

## 预期输出

```
========================================
   自动创建 GitHub Release
========================================

仓库: jsshwqz/patent-hub
Tag: v0.1.0
标题: Patent Hub v0.1.0 - AI 智能专利检索分析系统

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

## 验证 Release

创建成功后，访问：
```
https://github.com/jsshwqz/patent-hub/releases
```

检查：
- ✅ 显示 v0.1.0 Release
- ✅ 标记为 "Latest"
- ✅ 包含 ZIP 文件
- ✅ 可以下载

---

## 如果遇到问题

### 问题 1：Token 未设置
```powershell
# 重新设置
$env:GITHUB_TOKEN = "ghp_你的Token"
```

### 问题 2：Release 已存在
```powershell
# 删除旧 Release（需要先获取 Release ID）
# 或者修改脚本中的版本号为 v0.1.1
```

### 问题 3：ZIP 文件未找到
```powershell
# 检查文件是否存在
Test-Path "patent-hub-v0.1.0-windows-x86_64.zip"

# 如果返回 False，说明文件不存在
# 需要先构建和打包项目
```

---

## 一键执行（推荐）

创建一个 `create_release.bat` 文件：

```batch
@echo off
echo 请输入你的 GitHub Token:
set /p TOKEN=
set GITHUB_TOKEN=%TOKEN%
powershell -ExecutionPolicy Bypass -File auto_create_release.ps1
pause
```

然后双击运行 `create_release.bat`

---

**准备好了吗？开始创建 Release 吧！** 🚀
