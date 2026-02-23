# Patent Hub 专利检索系统

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/your-username/patent-hub)

一个基于 Rust + Axum 的专利检索和分析系统，支持在线搜索、AI 分析、专利对比等功能。

[English](README.en.md) | 简体中文

## 功能特性

✅ **在线专利搜索** - 通过 SerpAPI 搜索 Google Patents
✅ **搜索历史** - 自动保存最近 10 条搜索记录
✅ **高级筛选** - 按日期范围、国家/地区筛选
✅ **统计分析** - 申请人 TOP 10、国家分布、申请趋势图
✅ **导出功能** - 导出 Excel (CSV 格式)
✅ **AI 智能分析** - 专利摘要、技术分析
✅ **专利对比** - AI 智能对比两个专利
✅ **相似推荐** - 基于关键词推荐相似专利
✅ **文件对比** - 上传 TXT 文件与专利进行 AI 对比

## 快速开始

### 方式 1：双击启动（推荐）

直接双击 `启动服务器.bat` 文件，服务器会自动启动并打开浏览器。

启动后会显示：
- 本地访问地址：http://127.0.0.1:3000
- 移动设备访问地址：http://192.168.x.x:3000（手机/平板可用）

### 方式 2：命令行启动

```bash
cd patent-hub
cargo run --release
```

### 方式 3：直接运行可执行文件

```bash
cd patent-hub
.\target\release\patent-hub.exe
```

服务器启动后，访问：
- 电脑浏览器：http://127.0.0.1:3000
- 手机/平板：http://你的电脑IP:3000（需在同一 WiFi）

详见 [移动设备访问指南](docs/MOBILE_ACCESS.md)

## 开机自启动

### 安装自启动

双击运行 `安装开机自启动.bat`，Patent Hub 将在每次开机时自动启动。

### 卸载自启动

双击运行 `卸载开机自启动.bat`，移除开机自启动。

## 配置

### API 密钥配置

编辑 `.env` 文件：

```env
# SerpAPI 密钥（用于在线搜索 Google Patents）
SERPAPI_KEY=your-serpapi-key-here

# AI 服务配置（智谱 GLM）
AI_BASE_URL=https://open.bigmodel.cn/api/paas/v4
AI_API_KEY=your-glm-api-key-here
AI_MODEL=glm-4-flash
```

## 使用说明

### 搜索专利

1. 在搜索框输入关键词（如"咖啡"、"人工智能"）或申请人名称
2. 选择搜索模式：在线搜索（推荐）或本地数据库
3. 可选：选择国家/地区、设置日期范围
4. 点击"检索"按钮

### 查看专利详情

点击搜索结果中的专利标题，查看完整信息：
- 基本信息（专利号、申请人、发明人等）
- 摘要和权利要求
- AI 智能分析
- 相似专利推荐
- 上传文件对比

### 专利对比

1. 访问"专利对比"页面
2. 输入两个专利 ID 或专利号
3. 点击"开始对比"
4. AI 将分析两个专利的异同

### 导出数据

在搜索结果页面点击"导出 Excel"按钮，下载 CSV 格式的专利数据。

## 技术栈

- **后端**: Rust + Axum 0.6
- **数据库**: SQLite (rusqlite)
- **AI**: 智谱 GLM-4-Flash
- **搜索**: SerpAPI (Google Patents)
- **前端**: 原生 HTML + JavaScript

## 目录结构

```
patent-hub/
├── src/
│   ├── main.rs          # 主程序入口
│   ├── routes.rs        # API 路由
│   ├── db.rs            # 数据库操作
│   ├── ai.rs            # AI 服务
│   └── patent.rs        # 数据结构
├── templates/           # HTML 模板
│   ├── index.html       # 首页
│   ├── search.html      # 搜索页面
│   ├── patent_detail.html  # 专利详情
│   ├── compare.html     # 专利对比
│   └── ai.html          # AI 助手
├── static/              # 静态资源
│   └── style.css        # 样式表
├── target/release/      # 编译输出
│   └── patent-hub.exe   # 可执行文件
├── 启动服务器.bat       # 启动脚本
├── 安装开机自启动.bat   # 安装自启动
└── 卸载开机自启动.bat   # 卸载自启动
```

## 常见问题

### Q: 电脑重启后无法访问？
A: 服务器需要手动启动。建议：
   1. 双击 `启动服务器.bat` 启动
   2. 或运行 `安装开机自启动.bat` 实现开机自动启动

### Q: 搜索没有结果？
A: 检查：
   1. 是否配置了 SERPAPI_KEY
   2. 网络连接是否正常
   3. 尝试切换到"本地数据库"模式

### Q: AI 分析失败？
A: 检查：
   1. 是否配置了 AI_API_KEY
   2. API 密钥是否有效
   3. 网络连接是否正常

### Q: 如何停止服务器？
A: 在运行服务器的命令行窗口按 Ctrl+C

## 开发

### 编译

```bash
cargo build --release
```

### 运行测试

```bash
cargo test
```

### 开发模式

```bash
cargo run
```

## 开源贡献

欢迎贡献代码、报告问题或提出建议！

- 查看 [贡献指南](CONTRIBUTING.md)
- 提交 [Issue](../../issues)
- 发起 [Pull Request](../../pulls)

## 路线图

- [ ] 多语言界面支持
- [ ] 高级搜索语法
- [ ] 专利组合分析
- [ ] 引用网络可视化
- [ ] **移动端原生 APP**（欢迎贡献！见 [MOBILE_APP.md](docs/MOBILE_APP.md)）
  - [ ] Flutter 版本
  - [ ] React Native 版本
  - [ ] Android 原生
  - [ ] iOS 原生
  - [ ] HarmonyOS 原生
- [ ] 浏览器扩展
- [ ] PostgreSQL 支持
- [ ] 用户认证系统

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 致谢

感谢以下开源项目：
- [Rust](https://www.rust-lang.org/)
- [Axum](https://github.com/tokio-rs/axum)
- [Tokio](https://tokio.rs/)
- [SQLite](https://www.sqlite.org/)
- [Ollama](https://ollama.com/)

## 联系方式

如有问题或建议，欢迎在 [Issues](../../issues) 中讨论。
