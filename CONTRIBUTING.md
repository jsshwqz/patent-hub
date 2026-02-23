# 贡献指南 / Contributing Guide

感谢你对 Patent Hub 的关注！欢迎任何形式的贡献。

## 如何贡献

### 报告问题

在 [Issues](../../issues) 页面提交：
- Bug 报告
- 功能建议
- 文档改进

### 提交代码

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -m 'Add some feature'`
4. 推送分支：`git push origin feature/your-feature`
5. 提交 Pull Request

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 添加必要的注释和文档
- 确保所有测试通过：`cargo test`

### 跨平台支持

我们致力于支持多平台：
- Windows (主要开发平台)
- macOS
- Linux

提交代码时请考虑跨平台兼容性。

## 开发环境

### 必需工具

- Rust 1.70+
- SQLite 3

### 可选工具

- Ollama (本地 AI)
- SerpAPI 账号 (在线搜索)

## 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name

# 开发模式运行
cargo run
```

## 发布流程

1. 更新版本号 (Cargo.toml)
2. 更新 CHANGELOG.md
3. 创建 Git tag
4. 编译发布版本：`cargo build --release`

## 联系方式

有任何问题欢迎在 Issues 中讨论。
