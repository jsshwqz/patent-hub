# 移动设备访问指南 / Mobile Access Guide

## 概述 / Overview

Patent Hub 支持从手机、平板等移动设备访问。只需确保设备与电脑在同一 WiFi 网络下。

Patent Hub supports access from mobile devices (phones, tablets). Just ensure your device is on the same WiFi network as your computer.

## 快速开始 / Quick Start

### 1. 启动服务器 / Start Server

在电脑上启动 Patent Hub：

```bash
# Windows
.\启动服务器.bat

# macOS/Linux
./target/release/patent-hub
```

启动后会显示：
```
Patent Hub running at http://0.0.0.0:3000
Local access: http://127.0.0.1:3000
Mobile access: http://192.168.1.100:3000
```

### 2. 手机访问 / Mobile Access

在手机浏览器输入显示的 IP 地址，例如：
```
http://192.168.1.100:3000
```

## 详细步骤 / Detailed Steps

### Windows 用户

#### 方法 1：查看启动信息（推荐）

启动服务器后，终端会自动显示移动访问地址。

#### 方法 2：手动查找 IP

1. 按 `Win + R`，输入 `cmd`
2. 输入命令：`ipconfig`
3. 找到 "无线局域网适配器 WLAN" 或 "以太网适配器"
4. 记下 "IPv4 地址"，例如：`192.168.1.100`
5. 在手机浏览器访问：`http://192.168.1.100:3000`

### macOS 用户

#### 方法 1：查看启动信息（推荐）

启动服务器后，终端会自动显示移动访问地址。

#### 方法 2：手动查找 IP

1. 打开 "系统偏好设置" > "网络"
2. 选择当前连接的网络（WiFi 或以太网）
3. 查看 IP 地址，例如：`192.168.1.100`
4. 在手机浏览器访问：`http://192.168.1.100:3000`

或使用命令行：
```bash
ifconfig | grep "inet " | grep -v 127.0.0.1
```

### Linux 用户

#### 方法 1：查看启动信息（推荐）

启动服务器后，终端会自动显示移动访问地址。

#### 方法 2：手动查找 IP

```bash
# 方法 1
ip addr show

# 方法 2
hostname -I

# 方法 3
ifconfig
```

找到类似 `192.168.1.100` 的地址，在手机访问：`http://192.168.1.100:3000`

## 添加到手机主屏幕 / Add to Home Screen

### iOS (iPhone/iPad)

1. 在 Safari 浏览器打开 Patent Hub
2. 点击底部分享按钮 📤
3. 向下滚动，选择 "添加到主屏幕"
4. 输入名称（如 "Patent Hub"）
5. 点击 "添加"

现在可以像 App 一样从主屏幕打开！

### Android

1. 在 Chrome 浏览器打开 Patent Hub
2. 点击右上角菜单 ⋮
3. 选择 "添加到主屏幕" 或 "安装应用"
4. 输入名称（如 "Patent Hub"）
5. 点击 "添加"

### HarmonyOS (鸿蒙)

1. 在浏览器打开 Patent Hub
2. 点击菜单按钮
3. 选择 "添加到桌面" 或 "添加书签到桌面"
4. 输入名称
5. 确认添加

## 常见问题 / FAQ

### Q: 手机无法访问？

检查以下几点：
1. **同一网络**：确保手机和电脑连接到同一个 WiFi
2. **防火墙**：Windows 防火墙可能阻止访问
   - 打开 "Windows 安全中心" > "防火墙和网络保护"
   - 点击 "允许应用通过防火墙"
   - 添加 `patent-hub.exe` 并允许专用和公用网络
3. **IP 地址正确**：确认输入的 IP 地址正确
4. **端口号**：确保包含 `:3000`

### Q: 如何关闭移动访问？

如果只想本地访问，修改 `src/main.rs`：

```rust
// 改为只监听本地
let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
```

然后重新编译：
```bash
cargo build --release
```

### Q: 可以修改端口吗？

可以。修改 `src/main.rs` 中的端口号：

```rust
let addr = SocketAddr::from(([0, 0, 0, 0], 8080)); // 改为 8080
```

### Q: 手机访问速度慢？

- 确保 WiFi 信号良好
- 尝试关闭其他占用网络的应用
- 考虑使用 5GHz WiFi（如果支持）

### Q: 可以从外网访问吗？

默认不支持外网访问（安全考虑）。如需外网访问：

1. **使用内网穿透**（推荐）：
   - [frp](https://github.com/fatedier/frp)
   - [ngrok](https://ngrok.com/)
   - [Cloudflare Tunnel](https://www.cloudflare.com/products/tunnel/)

2. **配置路由器端口转发**（需要公网 IP）：
   - 登录路由器管理界面
   - 设置端口转发：外部端口 → 内部 IP:3000
   - 注意安全风险！

### Q: 支持 HTTPS 吗？

当前版本不支持。如需 HTTPS：

1. 使用反向代理（Nginx、Caddy）
2. 配置 SSL 证书
3. 或使用 Cloudflare Tunnel（自动 HTTPS）

## 性能优化 / Performance Tips

### 移动端优化建议

1. **使用 WiFi**：避免使用移动数据（流量消耗大）
2. **关闭后台应用**：释放手机内存
3. **清理浏览器缓存**：如果页面加载慢
4. **使用现代浏览器**：Chrome、Safari、Edge

### 服务器优化

1. **使用有线连接**：电脑使用网线比 WiFi 更稳定
2. **关闭省电模式**：确保电脑性能全开
3. **使用 SSD**：数据库读写更快

## 安全建议 / Security Tips

1. **仅在可信网络使用**：不要在公共 WiFi 上开放访问
2. **定期更新**：保持软件最新版本
3. **不要暴露到公网**：除非配置了认证和 HTTPS
4. **使用强密码**：如果添加了认证功能
5. **定期备份数据**：备份 `patent_hub.db` 文件

## 多设备协同 / Multi-Device Collaboration

Patent Hub 支持多设备同时访问：

- 电脑浏览器：查看详细信息、导出数据
- 手机：随时随地搜索、查看专利
- 平板：舒适的阅读体验

所有设备共享同一个数据库，搜索历史和收藏同步。

## 故障排除 / Troubleshooting

### Windows 防火墙配置

```powershell
# 允许 3000 端口
netsh advfirewall firewall add rule name="Patent Hub" dir=in action=allow protocol=TCP localport=3000
```

### macOS 防火墙配置

1. 系统偏好设置 > 安全性与隐私 > 防火墙
2. 点击 "防火墙选项"
3. 添加 `patent-hub` 并允许传入连接

### Linux 防火墙配置

```bash
# UFW
sudo ufw allow 3000/tcp

# firewalld
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 3000 -j ACCEPT
```

## 技术细节 / Technical Details

### 网络绑定

服务器绑定到 `0.0.0.0:3000`，意味着：
- `127.0.0.1:3000` - 本地访问
- `192.168.x.x:3000` - 局域网访问
- `0.0.0.0` 监听所有网络接口

### 支持的浏览器

- iOS: Safari 14+
- Android: Chrome 90+, Firefox 88+
- HarmonyOS: 系统浏览器, Chrome
- 其他: 任何现代浏览器

### 响应式设计

Patent Hub 的界面会自动适配移动设备屏幕大小。

## 反馈 / Feedback

如有问题或建议，欢迎在 [GitHub Issues](../../issues) 反馈。
