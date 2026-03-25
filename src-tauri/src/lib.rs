use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
