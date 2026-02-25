# 多仓库同步配置

## 当前配置

本项目已配置同时推送到 GitHub 和 Gitee：

- **GitHub**: https://github.com/jsshwqz/patent-hub.git
- **Gitee**: https://gitee.com/jsshwqz/patent-hub.git

## 使用方法

### 一次推送到两个仓库

```bash
git push origin main
```

这个命令会自动同时推送到 GitHub 和 Gitee。

### 单独推送到某个仓库

```bash
# 只推送到 GitHub
git push https://github.com/jsshwqz/patent-hub.git main

# 只推送到 Gitee  
git push gitee main
```

## 配置说明

当前 git remote 配置：
```
origin  https://github.com/jsshwqz/patent-hub.git (fetch)
origin  https://github.com/jsshwqz/patent-hub.git (push)
origin  https://gitee.com/jsshwqz/patent-hub.git (push)
gitee   https://gitee.com/jsshwqz/patent-hub.git (fetch)
gitee   https://gitee.com/jsshwqz/patent-hub.git (push)
```

这意味着：
- `git fetch` 和 `git pull` 从 GitHub 拉取
- `git push origin` 同时推送到 GitHub 和 Gitee
- `git push gitee` 只推送到 Gitee

## 优势

1. **一次推送，双重备份**：代码同时保存在 GitHub 和 Gitee
2. **国内访问友好**：Gitee 在国内访问更快
3. **自动同步**：无需手动推送两次

---
配置时间：2026-02-25
