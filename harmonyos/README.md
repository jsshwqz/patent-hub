# Patent Hub 鸿蒙版 (HarmonyOS)

Patent Hub 的 HarmonyOS 原生客户端，通过 Web 组件加载本地 Patent Hub 服务。

## 项目信息

- **包名**: com.patenthub.app
- **版本**: 0.3.5 (versionCode: 30500)
- **最低 SDK**: HarmonyOS 5.0.0 (API 12)
- **支持设备**: 手机、平板

## 项目结构

```
harmonyos/
├── AppScope/                        # 应用级配置
│   ├── app.json5                    # 应用配置（包名、版本）
│   └── resources/base/
│       ├── element/string.json      # 应用名称字符串
│       └── media/                   # 应用图标
├── entry/                           # 主模块
│   ├── src/main/
│   │   ├── ets/
│   │   │   ├── entryability/
│   │   │   │   └── EntryAbility.ets # 主 Ability（全屏设置）
│   │   │   └── pages/
│   │   │       ├── Splash.ets       # 启动页（2秒后跳转）
│   │   │       └── Index.ets        # 主页（WebView）
│   │   ├── module.json5             # 模块配置
│   │   └── resources/base/
│   │       ├── element/
│   │       │   ├── string.json      # 字符串资源
│   │       │   └── color.json       # 颜色资源（深色主题）
│   │       └── profile/
│   │           └── main_pages.json  # 页面路由
│   ├── build-profile.json5          # 模块构建配置
│   └── hvigorfile.ts                # 模块构建脚本
├── build-profile.json5              # 项目构建配置
├── hvigorfile.ts                    # 项目构建脚本
└── README.md                        # 本文件
```

## 功能特性

- **WebView 容器**: 加载本地 Patent Hub 服务 (http://127.0.0.1:3000/search)
- **启动页**: 品牌展示，2秒自动跳转
- **全屏沉浸式**: 状态栏与导航栏融合深色主题
- **返回键处理**: WebView 内优先后退浏览历史，无历史时退出应用
- **加载进度条**: 页面加载时顶部显示蓝色进度条
- **错误处理**: 连接失败时显示错误页面和重试按钮
- **JavaScript & DOM 存储**: 完整 Web 功能支持

## 前置条件

1. 安装 [DevEco Studio](https://developer.huawei.com/consumer/cn/deveco-studio/) 4.0 或更高版本
2. 配置 HarmonyOS SDK (API 12+)
3. Patent Hub 本地服务已启动并运行在 `http://127.0.0.1:3000`

## 构建步骤

### 使用 DevEco Studio

1. 打开 DevEco Studio
2. 选择 `File > Open`，打开 `harmonyos/` 目录
3. 等待项目同步完成
4. 连接鸿蒙设备或启动模拟器
5. 点击 `Run > Run 'entry'` 构建并安装

### 使用命令行

```bash
cd harmonyos/

# 构建 debug 版本
hvigorw assembleHap --mode module -p product=default -p buildMode=debug

# 构建 release 版本
hvigorw assembleHap --mode module -p product=default -p buildMode=release
```

构建产物位于 `entry/build/default/outputs/` 目录。

## 使用说明

1. 先在设备上启动 Patent Hub 后端服务
2. 打开专利中心 APP
3. 启动页展示后自动进入主界面
4. 如果服务未启动，会显示错误页面，启动服务后点击「重新加载」

## 调试

- DevEco Studio 中使用 `Log` 面板，过滤 `PatentHub` 标签查看日志
- Web 组件内容可通过 `chrome://inspect` 远程调试（需开启调试模式）

## 注意事项

- 应用需要 `ohos.permission.INTERNET` 网络权限（已在 module.json5 中声明）
- 本应用为 WebView 容器，核心业务逻辑在 Patent Hub Web 端
- 深色主题配色与 Patent Hub Web 端保持一致 (#0d1117 背景)
