//! Patent Hub 移动端入口 - 纯 Rust
//!
//! 启动内嵌 Axum 服务器 + WebView，完全不依赖 Java。
//! 编译目标: aarch64-linux-android

use std::thread;
use std::time::Duration;

fn main() {
    // 1. 确定数据库路径（Android 沙箱内）
    let db_path = if cfg!(target_os = "android") {
        // Android: 使用 app 私有目录
        std::env::var("HOME")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/data/local/tmp".to_string())
            + "/patent_hub.db"
    } else {
        "patent_hub.db".to_string()
    };

    println!("[Patent Hub Mobile] 数据库路径: {}", db_path);

    // 2. 后台线程启动 Axum 服务器
    let db_path_clone = db_path.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()
            .expect("无法创建 tokio 运行时");
        rt.block_on(async {
            println!("[Patent Hub Mobile] 正在启动服务器...");
            if let Err(e) = patent_hub::start_server(&db_path_clone).await {
                eprintln!("[Patent Hub Mobile] 服务器错误: {}", e);
            }
        });
    });

    // 3. 等待服务器启动
    println!("[Patent Hub Mobile] 等待服务器就绪...");
    for i in 0..30 {
        thread::sleep(Duration::from_millis(200));
        if let Ok(_) = std::net::TcpStream::connect("127.0.0.1:3000") {
            println!("[Patent Hub Mobile] 服务器已就绪 ({}ms)", (i + 1) * 200);
            break;
        }
    }

    // 4. 打开 WebView（桌面端用浏览器，Android 端用 wry）
    #[cfg(not(target_os = "android"))]
    {
        let url = "http://127.0.0.1:3000/search";
        println!("[Patent Hub Mobile] 打开: {}", url);
        let _ = open::that(url);
        // 保持主线程运行
        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    }

    #[cfg(target_os = "android")]
    {
        start_android_webview();
    }
}

#[cfg(target_os = "android")]
fn start_android_webview() {
    use wry::WebViewBuilder;
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use tao::event::{Event, WindowEvent};

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Patent Hub")
        .build(&event_loop)
        .unwrap();

    let _webview = WebViewBuilder::new()
        .with_url("http://127.0.0.1:3000/search")
        .build(&window)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
