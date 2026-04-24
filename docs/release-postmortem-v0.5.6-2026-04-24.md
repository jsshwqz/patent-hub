# InnoForge v0.5.6 发布后复盘

日期：2026-04-24

## 发布结论

v0.5.6 已按 0.5.x 修复版本发布。发布标签为 `v0.5.6`，指向提交 `79622c31435962949738b669329ddd5bab1a4b3c`。

GitHub Release：`https://github.com/jsshwqz/innoforge/releases/tag/v0.5.6`

## 已验证证据

- 本地版本：`Cargo.toml` 为 `0.5.6`。
- 本地发布前检查：`git diff --check` 通过。
- 本地单元测试：`cargo test --lib` 通过，31/31。
- 本地静态检查：`cargo clippy --tests -- -D warnings` 通过。
- 主线 CI：GitHub Actions `CI #147` 对提交 `79622c3` 为 `completed/success`。
- 发布工作流：GitHub Actions `Release #57` 为 `completed/success`。
- 发布资产：GitHub Release `v0.5.6` 包含 5 个资产。
- 标签同步：`v0.5.6` 已推送到 GitHub 与 Gitee。

## 发布资产

- `innoforge-linux-aarch64.tar.gz`
- `innoforge-linux-x86_64.tar.gz`
- `innoforge-macos-aarch64.tar.gz`
- `innoforge-macos-x86_64.tar.gz`
- `innoforge-windows-x86_64.zip`

## 本轮暴露的问题

- CI 页面匿名访问偶发加载错误，不能只靠页面直觉判断状态；需要同时使用 Actions API 或作业详情页交叉验证。
- GitHub API 匿名额度会被快速消耗；轮询应控制频率，避免在关键阶段被限流。
- Release 工作流存在 Node.js 20 deprecation warning，当前不影响发布，但后续需要升级 action 依赖或设置 Node 24 兼容策略。
- rust-cache 在 Linux aarch64 作业出现缓存保存 warning，当前不影响构建结果，但后续可单独优化缓存键或忽略该类非阻断警告。

## 后续必须保持的门禁

- 不得在未完成真实页面/按钮/输入输出验证前宣布版本可发布。
- 不得只凭本地构建通过宣布发布成功，必须核对远端 CI、Release 工作流和资产数量。
- 标签发布前必须确认 `vX.Y.Z` 不存在，避免覆盖历史发布。
- 推送前必须确认工作区干净、端口无残留占用、无残留 `innoforge/cargo/rustc` 进程。
- 发布后必须补复盘文档，并把发布证据、风险和改进项归档。

## 改进项

- 将 Release 工作流中的 `actions/checkout@v4`、`softprops/action-gh-release@v2` 兼容性预警纳入下一轮维护任务。
- 给发布监控脚本增加指数退避与 API 限流提示，减少匿名 API 限额被打满。
- 将“真实页面逐项截图测试摘要”和“Release 资产核验”合并到单一发布证据索引文档，降低复查成本。
