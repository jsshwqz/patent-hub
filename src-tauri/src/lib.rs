use tauri::Manager;

/// Start the embedded patent-hub web server in a background thread.
fn start_embedded_server() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = patent_hub::start_server("patent_hub.db").await {
                eprintln!("[Patent Hub APP] Server error: {}", e);
            }
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Start the web server before creating the window
    start_embedded_server();

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_millis(2000));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                let _window = app.get_webview_window("main")
                    .expect("main window not found");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Patent Hub");
}
