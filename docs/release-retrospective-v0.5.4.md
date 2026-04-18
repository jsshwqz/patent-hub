# InnoForge v0.5.4 发布复盘（2026-04-18）

## 目标
- 完成 `innoforge` 主仓在 GitHub/Gitee 双端一致发布。
- 补齐多终端三仓（desktop/ios/harmony）的远端发布与一致性校验。

## 本次问题
1. 三端仓初始远端不存在，导致持续 `404 not found`，本地提交无法对外发布。
2. 本机 `gh.exe` 不可执行（平台不兼容），GitHub CLI 方案失效。
3. 三端本地仓仅配置了 Gitee 远端，缺少 GitHub 远端，不满足双端发布规则。

## 已执行修复
1. 通过 GitHub/Gitee API 创建三端远端仓：
- `jsshwqz/patent-hub-desktop`
- `jsshwqz/patent-hub-ios`
- `jsshwqz/patent-hub-harmony`
2. 为三端本地仓补齐双远端（`origin`=Gitee，`github`=GitHub）。
3. 将三端本地已完成提交推送到双端并核对：
- desktop: `ced2ff1`
- ios: `1a34964`
- harmony: `1e3bafb`
4. 主仓双端与标签一致性复核：
- `main` 与 `v0.5.4` 均指向 `346e426`

## 结果
- 主仓 `innoforge`：双端发布成功，一致性通过。
- 三端仓：双端发布成功，一致性通过。
- 工作区状态：主仓与三端仓均为干净状态（无脏改动）。

## 防错清单（后续发布前必做）
1. 先执行远端存在性检查：`git ls-remote <remote> refs/heads/main`。
2. 每个发布仓必须存在双远端：`origin` 与 `github`（或明确等价命名）。
3. 推送后执行双端提交一致性检查，确认 commit SHA 相同。
4. 工具链降级预案：当 `gh` 不可用时，直接切换 API + `git` 推送流程。
