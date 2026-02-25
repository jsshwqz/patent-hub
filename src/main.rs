mod db;
mod ai;
mod patent;
mod routes;

use axum::{Router, routing::get, routing::post};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

/// Try to start Ollama if not already running
fn ensure_ollama() {
    let base = std::env::var("AI_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
    if !base.contains("localhost:11434") && !base.contains("127.0.0.1:11434") {
        return; // Using a remote AI service, skip Ollama
    }
    // Check if Ollama is already running
    match std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_secs(1),
    ) {
        Ok(_) => println!("Ollama is already running"),
        Err(_) => {
            println!("Starting Ollama service...");
            match std::process::Command::new("ollama")
                .arg("serve")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => {
                    // Wait a moment for Ollama to start
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    println!("Ollama service started");
                }
                Err(e) => println!("Could not start Ollama: {} (install from https://ollama.com)", e),
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();
    ensure_ollama();
    let db = db::Database::init("patent_hub.db")?;
    let state = routes::AppState { 
        db: std::sync::Arc::new(db),
    };
    let app = Router::new()
        .route("/", get(routes::index_page))
        .route("/search", get(routes::search_page))
        .route("/patent/:id", get(routes::patent_detail_page))
        .route("/ai", get(routes::ai_page))
        .route("/compare", get(routes::compare_page))
        .route("/settings", get(routes::settings_page))
        .route("/api/settings", get(routes::api_get_settings))
        .route("/api/settings/serpapi", post(routes::api_save_serpapi))
        .route("/api/settings/ai", post(routes::api_save_ai))
        .route("/api/search", post(routes::api_search))
        .route("/api/search/stats", post(routes::api_search_stats))
        .route("/api/search/export", post(routes::api_export_csv))
        .route("/api/patent/fetch", post(routes::api_fetch_patent))
        .route("/api/ai/chat", post(routes::api_ai_chat))
        .route("/api/ai/summarize", post(routes::api_ai_summarize))
        .route("/api/ai/compare", post(routes::api_ai_compare))
        .route("/api/patents/import", post(routes::api_import_patents))
        .route("/api/search/online", post(routes::api_search_online))
        .route("/api/patent/enrich/:id", get(routes::api_enrich_patent))
        .route("/api/patent/similar/:id", get(routes::api_recommend_similar))
        .route("/api/upload/compare", post(routes::api_upload_compare))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);
    // Bind to 0.0.0.0 to allow access from other devices (mobile, etc.)
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Patent Hub running at http://{addr}");
    println!("Local access: http://127.0.0.1:3000");
    
    // Try to get local IP for mobile access
    match local_ip_address::local_ip() {
        Ok(local_ip) => {
            println!("Mobile access: http://{}:3000", local_ip);
            println!("Scan QR code on mobile:");
            println!("  ┌─────────────────────────────┐");
            println!("  │ Use your phone camera to    │");
            println!("  │ visit: http://{}:3000 │", local_ip);
            println!("  └─────────────────────────────┘");
        }
        Err(_) => {
            println!("Could not detect local IP. Check your network settings.");
        }
    }
    
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}