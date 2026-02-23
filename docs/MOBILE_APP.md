# 移动端 APP 开发指南 / Mobile App Development Guide

## 当前状态 / Current Status

Patent Hub 目前提供：
- ✅ **Web 版本**：通过浏览器访问（支持所有设备）
- ✅ **RESTful API**：完整的后端 API（见 [API.md](API.md)）
- 🚧 **原生 APP**：欢迎社区贡献！

## 为什么需要原生 APP？

Web 版本的限制：
- 需要保持服务器运行
- 依赖网络连接
- 无法使用某些原生功能（推送通知、离线缓存等）

原生 APP 的优势：
- 更好的性能和用户体验
- 离线功能支持
- 系统级集成（分享、通知等）
- 独立运行，无需服务器

## 技术方案建议

### 方案 1：Flutter（推荐）

**优势：**
- 一套代码，支持 Android、iOS、HarmonyOS
- 性能接近原生
- 丰富的 UI 组件库
- 活跃的社区

**技术栈：**
```yaml
dependencies:
  flutter:
    sdk: flutter
  http: ^1.1.0          # API 调用
  sqflite: ^2.3.0       # 本地数据库
  provider: ^6.1.0      # 状态管理
  shared_preferences: ^2.2.0  # 本地存储
```

**项目结构：**
```
patent-hub-mobile/
├── lib/
│   ├── main.dart
│   ├── models/          # 数据模型
│   ├── services/        # API 服务
│   ├── screens/         # 页面
│   ├── widgets/         # 组件
│   └── utils/           # 工具
├── android/
├── ios/
└── pubspec.yaml
```

### 方案 2：React Native

**优势：**
- JavaScript/TypeScript 开发
- 热重载
- 大量第三方库

**技术栈：**
```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-native": "^0.73.0",
    "axios": "^1.6.0",
    "react-navigation": "^6.0.0",
    "@react-native-async-storage/async-storage": "^1.21.0"
  }
}
```

### 方案 3：原生开发

**Android (Kotlin):**
```kotlin
// 技术栈
- Jetpack Compose (UI)
- Retrofit (网络)
- Room (数据库)
- Coroutines (异步)
```

**iOS (Swift):**
```swift
// 技术栈
- SwiftUI (UI)
- Alamofire (网络)
- CoreData (数据库)
- Combine (响应式)
```

**HarmonyOS (ArkTS):**
```typescript
// 技术栈
- ArkUI (UI)
- HTTP (网络)
- 关系型数据库
```

## API 集成

Patent Hub 提供完整的 RESTful API，移动端可以直接调用：

### 基础配置

```dart
// Flutter 示例
class ApiConfig {
  // 开发环境：连接本地服务器
  static const String devBaseUrl = 'http://192.168.1.100:3000';
  
  // 生产环境：连接云服务器（如果部署）
  static const String prodBaseUrl = 'https://api.patent-hub.com';
  
  static String get baseUrl => 
    const bool.fromEnvironment('dart.vm.product') 
      ? prodBaseUrl 
      : devBaseUrl;
}
```

### API 调用示例

```dart
// 搜索专利
Future<List<Patent>> searchPatents(String query) async {
  final response = await http.post(
    Uri.parse('${ApiConfig.baseUrl}/api/search'),
    headers: {'Content-Type': 'application/json'},
    body: jsonEncode({
      'query': query,
      'mode': 'online',
      'page': 1,
      'page_size': 20,
    }),
  );
  
  if (response.statusCode == 200) {
    final data = jsonDecode(response.body);
    return (data['patents'] as List)
        .map((p) => Patent.fromJson(p))
        .toList();
  }
  throw Exception('搜索失败');
}

// 获取专利详情
Future<Patent> getPatentDetail(String id) async {
  final response = await http.get(
    Uri.parse('${ApiConfig.baseUrl}/api/patent/$id'),
  );
  
  if (response.statusCode == 200) {
    return Patent.fromJson(jsonDecode(response.body));
  }
  throw Exception('获取详情失败');
}

// AI 分析
Future<String> analyzePatent(String patentId) async {
  final response = await http.post(
    Uri.parse('${ApiConfig.baseUrl}/api/ai/analyze'),
    headers: {'Content-Type': 'application/json'},
    body: jsonEncode({'patent_id': patentId}),
  );
  
  if (response.statusCode == 200) {
    final data = jsonDecode(response.body);
    return data['analysis'];
  }
  throw Exception('AI 分析失败');
}
```

## 功能清单

### 核心功能（MVP）

- [ ] 专利搜索
  - [ ] 关键词搜索
  - [ ] 申请人搜索
  - [ ] 高级筛选（日期、国家）
- [ ] 专利详情
  - [ ] 基本信息展示
  - [ ] 权利要求
  - [ ] 说明书
- [ ] 搜索历史
- [ ] 收藏功能

### 高级功能

- [ ] AI 分析
  - [ ] 专利摘要
  - [ ] 技术分析
  - [ ] 专利对比
- [ ] 相似专利推荐
- [ ] 数据导出
- [ ] 离线缓存
- [ ] 推送通知
- [ ] 多语言支持

### 用户体验

- [ ] 深色模式
- [ ] 手势操作
- [ ] 语音搜索
- [ ] 扫码搜索（专利号）
- [ ] 分享功能
- [ ] 打印/PDF 导出

## 开发步骤

### 1. 环境准备

**Flutter:**
```bash
# 安装 Flutter
# https://flutter.dev/docs/get-started/install

# 创建项目
flutter create patent_hub_mobile
cd patent_hub_mobile

# 添加依赖
flutter pub add http sqflite provider
```

**React Native:**
```bash
# 安装 React Native CLI
npm install -g react-native-cli

# 创建项目
npx react-native init PatentHubMobile
cd PatentHubMobile

# 安装依赖
npm install axios @react-navigation/native
```

### 2. 项目结构

```
patent-hub-mobile/
├── README.md
├── docs/
│   ├── SETUP.md          # 开发环境配置
│   ├── API.md            # API 使用说明
│   └── CONTRIBUTING.md   # 贡献指南
├── lib/ (或 src/)
│   ├── models/
│   │   ├── patent.dart
│   │   └── search_result.dart
│   ├── services/
│   │   ├── api_service.dart
│   │   └── database_service.dart
│   ├── screens/
│   │   ├── home_screen.dart
│   │   ├── search_screen.dart
│   │   ├── detail_screen.dart
│   │   └── compare_screen.dart
│   ├── widgets/
│   │   ├── patent_card.dart
│   │   └── search_bar.dart
│   └── utils/
│       ├── constants.dart
│       └── helpers.dart
└── test/
```

### 3. 数据模型

```dart
// lib/models/patent.dart
class Patent {
  final String id;
  final String patentNumber;
  final String title;
  final String abstractText;
  final String applicant;
  final String inventor;
  final String filingDate;
  final String country;
  
  Patent({
    required this.id,
    required this.patentNumber,
    required this.title,
    required this.abstractText,
    required this.applicant,
    required this.inventor,
    required this.filingDate,
    required this.country,
  });
  
  factory Patent.fromJson(Map<String, dynamic> json) {
    return Patent(
      id: json['id'],
      patentNumber: json['patent_number'],
      title: json['title'],
      abstractText: json['abstract_text'],
      applicant: json['applicant'],
      inventor: json['inventor'],
      filingDate: json['filing_date'],
      country: json['country'],
    );
  }
}
```

### 4. API 服务

```dart
// lib/services/api_service.dart
class ApiService {
  static const String baseUrl = 'http://192.168.1.100:3000';
  
  Future<List<Patent>> searchPatents(String query) async {
    // 实现搜索逻辑
  }
  
  Future<Patent> getPatentDetail(String id) async {
    // 实现获取详情逻辑
  }
  
  Future<String> analyzePatent(String id) async {
    // 实现 AI 分析逻辑
  }
}
```

### 5. UI 实现

```dart
// lib/screens/search_screen.dart
class SearchScreen extends StatefulWidget {
  @override
  _SearchScreenState createState() => _SearchScreenState();
}

class _SearchScreenState extends State<SearchScreen> {
  final ApiService _apiService = ApiService();
  List<Patent> _patents = [];
  bool _isLoading = false;
  
  void _search(String query) async {
    setState(() => _isLoading = true);
    try {
      final patents = await _apiService.searchPatents(query);
      setState(() {
        _patents = patents;
        _isLoading = false;
      });
    } catch (e) {
      setState(() => _isLoading = false);
      // 显示错误
    }
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text('专利搜索')),
      body: Column(
        children: [
          SearchBar(onSearch: _search),
          _isLoading 
            ? CircularProgressIndicator()
            : Expanded(
                child: ListView.builder(
                  itemCount: _patents.length,
                  itemBuilder: (context, index) {
                    return PatentCard(patent: _patents[index]);
                  },
                ),
              ),
        ],
      ),
    );
  }
}
```

## 贡献方式

### 我想贡献移动端 APP！

太好了！欢迎加入：

1. **Fork 仓库**
   ```bash
   git clone https://github.com/your-username/patent-hub.git
   cd patent-hub
   ```

2. **创建移动端分支**
   ```bash
   git checkout -b mobile/flutter-app
   # 或
   git checkout -b mobile/react-native-app
   # 或
   git checkout -b mobile/android-app
   ```

3. **创建项目目录**
   ```bash
   mkdir mobile
   cd mobile
   # 初始化你的移动项目
   ```

4. **开发**
   - 参考 API 文档实现功能
   - 遵循代码规范
   - 编写测试
   - 更新文档

5. **提交 PR**
   ```bash
   git add .
   git commit -m "feat: add Flutter mobile app"
   git push origin mobile/flutter-app
   ```
   
   然后在 GitHub 创建 Pull Request

### 需要帮助？

- 📖 查看 [API 文档](API.md)
- 💬 在 [Discussions](../../discussions) 讨论
- 🐛 提交 [Issue](../../issues)
- 📧 联系维护者

## 部署方案

### 开发环境

移动端连接本地服务器：
```
http://192.168.1.100:3000
```

### 生产环境

#### 选项 1：自托管服务器

将 Patent Hub 部署到云服务器：
- AWS EC2
- 阿里云 ECS
- 腾讯云 CVM
- DigitalOcean

#### 选项 2：Serverless

使用 Serverless 架构：
- AWS Lambda + API Gateway
- Vercel
- Cloudflare Workers

#### 选项 3：容器化

使用 Docker + Kubernetes：
```bash
docker build -t patent-hub .
docker push your-registry/patent-hub
kubectl apply -f k8s/
```

## 发布

### Android

1. 生成签名密钥
2. 配置 `build.gradle`
3. 构建 APK/AAB
4. 发布到 Google Play / 华为应用市场

### iOS

1. 配置 Apple Developer 账号
2. 设置证书和描述文件
3. 构建 IPA
4. 发布到 App Store

### HarmonyOS

1. 注册华为开发者
2. 配置签名
3. 构建 HAP
4. 发布到华为应用市场

## 路线图

### Phase 1: MVP（3-6 个月）
- [ ] Flutter 版本基础功能
- [ ] Android 发布
- [ ] iOS 发布

### Phase 2: 功能完善（6-12 个月）
- [ ] AI 功能集成
- [ ] 离线支持
- [ ] HarmonyOS 版本

### Phase 3: 生态建设（12+ 个月）
- [ ] React Native 版本
- [ ] 原生 Android/iOS 版本
- [ ] 平板优化
- [ ] 桌面端（Electron）

## 资源

### 学习资料

- [Flutter 官方文档](https://flutter.dev/docs)
- [React Native 官方文档](https://reactnative.dev/docs/getting-started)
- [Android 开发文档](https://developer.android.com/)
- [iOS 开发文档](https://developer.apple.com/documentation/)
- [HarmonyOS 开发文档](https://developer.harmonyos.com/)

### 设计资源

- [Material Design](https://material.io/)
- [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [Figma 设计稿](待创建)

### 示例项目

- [Flutter 专利搜索 Demo](待创建)
- [React Native 专利搜索 Demo](待创建)

## 常见问题

### Q: 我不会移动开发，能贡献吗？

A: 当然！你可以：
- 改进 API 文档
- 提供设计建议
- 测试和反馈
- 翻译文档

### Q: 需要什么技术水平？

A: 
- 基础：了解 REST API、JSON
- 进阶：熟悉 Flutter/React Native
- 高级：原生开发经验

### Q: 开发周期多长？

A: 
- MVP（基础功能）：1-2 个月
- 完整版本：3-6 个月
- 持续迭代

### Q: 如何获得支持？

A: 
- GitHub Discussions
- Issue 提问
- 社区交流群（待建立）

## 致谢

感谢所有为移动端开发做出贡献的开发者！

---

**准备好开始了吗？** 

1. 选择你熟悉的技术栈
2. Fork 仓库
3. 开始编码
4. 提交 PR

让我们一起打造最好的开源专利搜索 APP！🚀
