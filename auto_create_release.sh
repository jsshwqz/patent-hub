#!/bin/bash
# 自动创建 GitHub Release
# 使用 GitHub REST API 和 curl

set -e

OWNER="jsshwqz"
REPO="patent-hub"
TAG="v0.1.0"
NAME="Patent Hub v0.1.0 - AI 智能专利检索分析系统"
ZIP_FILE="patent-hub-v0.1.0-windows-x86_64.zip"

# Release 说明（简化版）
BODY="## Patent Hub v0.1.0 - 首个公开发布版本

### 核心特性
- 专利检索（在线 + 本地）
- AI 智能分析（支持智谱 GLM）
- 专利对比分析
- 文件上传对比（独有功能）
- 数据统计图表
- Excel 导出

### 国内用户友好
- 支持智谱 GLM（国内直连）
- 大部分功能无需代理
- 详细的国内用户指南

### 快速开始
1. 下载 ZIP 包
2. 解压到任意目录
3. 配置 .env 文件
4. 运行 start.bat
5. 访问 http://localhost:3000

详见: https://github.com/jsshwqz/patent-hub/blob/main/README.md"

echo "========================================"
echo "   自动创建 GitHub Release"
echo "========================================"
echo ""
echo "仓库: $OWNER/$REPO"
echo "Tag: $TAG"
echo ""

# 检查 ZIP 文件
if [ ! -f "$ZIP_FILE" ]; then
    echo "错误: 未找到 $ZIP_FILE"
    echo "请先构建并打包项目"
    exit 1
fi

ZIP_SIZE=$(du -h "$ZIP_FILE" | cut -f1)
echo "✓ 找到发布包: $ZIP_FILE ($ZIP_SIZE)"
echo ""

# 检查 GITHUB_TOKEN
if [ -z "$GITHUB_TOKEN" ]; then
    echo "错误: 未设置 GITHUB_TOKEN 环境变量"
    echo ""
    echo "请设置 GitHub Personal Access Token:"
    echo "  export GITHUB_TOKEN='ghp_your_token_here'"
    echo ""
    echo "获取 Token:"
    echo "1. 访问: https://github.com/settings/tokens"
    echo "2. 点击 'Generate new token (classic)'"
    echo "3. 勾选 'repo' 权限"
    echo "4. 生成并复制 Token"
    echo ""
    exit 1
fi

echo "✓ 检测到 GITHUB_TOKEN"
echo ""

# 创建 Release
echo "正在创建 Release..."

RELEASE_DATA=$(cat <<EOF
{
  "tag_name": "$TAG",
  "name": "$NAME",
  "body": $(echo "$BODY" | jq -Rs .),
  "draft": false,
  "prerelease": false
}
EOF
)

RESPONSE=$(curl -s -X POST \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  -d "$RELEASE_DATA" \
  "https://api.github.com/repos/$OWNER/$REPO/releases")

# 检查是否成功
if echo "$RESPONSE" | grep -q '"id"'; then
    echo "✓ Release 创建成功!"
    echo ""
    
    RELEASE_ID=$(echo "$RESPONSE" | grep '"id"' | head -1 | sed 's/.*: \([0-9]*\).*/\1/')
    UPLOAD_URL=$(echo "$RESPONSE" | grep '"upload_url"' | sed 's/.*: "\(.*\){.*/\1/')
    HTML_URL=$(echo "$RESPONSE" | grep '"html_url"' | head -1 | sed 's/.*: "\(.*\)".*/\1/')
    
    echo "Release ID: $RELEASE_ID"
    echo "Release URL: $HTML_URL"
    echo ""
    
    # 上传 ZIP 文件
    echo "正在上传 $ZIP_FILE..."
    
    UPLOAD_RESPONSE=$(curl -s -X POST \
      -H "Authorization: Bearer $GITHUB_TOKEN" \
      -H "Content-Type: application/zip" \
      -H "Accept: application/vnd.github+json" \
      --data-binary @"$ZIP_FILE" \
      "${UPLOAD_URL}?name=$ZIP_FILE")
    
    if echo "$UPLOAD_RESPONSE" | grep -q '"browser_download_url"'; then
        echo "✓ 文件上传成功!"
        echo ""
        
        DOWNLOAD_URL=$(echo "$UPLOAD_RESPONSE" | grep '"browser_download_url"' | sed 's/.*: "\(.*\)".*/\1/')
        
        echo "下载链接: $DOWNLOAD_URL"
        echo ""
        echo "========================================"
        echo "   Release 创建完成!"
        echo "========================================"
        echo ""
        echo "用户可以从以下地址下载:"
        echo "$HTML_URL"
        echo ""
    else
        echo "错误: 文件上传失败"
        echo "$UPLOAD_RESPONSE"
        exit 1
    fi
else
    echo "错误: Release 创建失败"
    echo "$RESPONSE"
    echo ""
    echo "可能的原因:"
    echo "1. Token 权限不足（需要 'repo' 权限）"
    echo "2. Release 已存在（需要先删除旧的 Release）"
    echo "3. 网络连接问题"
    echo ""
    exit 1
fi
