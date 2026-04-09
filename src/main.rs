//! # 创研台 InnoForge 主程序 / Main Application
//!
//! Axum Web 服务器入口，注册所有路由并启动 HTTP 服务。
//! Axum web server entry point, registers all routes and starts HTTP service.
//!
//! 默认监听 `0.0.0.0:3000`，自动打开浏览器。
//! Listens on `0.0.0.0:3000` by default, auto-opens browser.

mod ai;
mod db;
mod error;
mod experiment;
mod orchestrator;
mod patent;
pub mod pipeline;
mod routes;

use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Response, StatusCode},
    routing::{get, post},
    Router,
};
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> Response<Body> {
    match StaticAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime.as_ref())
                .header("Cache-Control", "public, max-age=3600")
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv_override();
    tracing_subscriber::fmt::init();

    // Use app data directory on Android, current dir otherwise
    let db_path = if cfg!(target_os = "android") {
        let data_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/data/local/tmp".to_string());
        format!("{}/innoforge.db", data_dir)
    } else {
        // 兼容旧版数据库文件名
        if std::path::Path::new("patent_hub.db").exists() && !std::path::Path::new("innoforge.db").exists() {
            let _ = std::fs::rename("patent_hub.db", "innoforge.db");
        }
        "innoforge.db".to_string()
    };
    let db = db::Database::init(&db_path)?;

    // 启动时将卡在 analyzing 的创意重置为 error（上次 pipeline 中断）
    match db.reset_stale_analyzing() {
        Ok(n) if n > 0 => tracing::warn!("Reset {} stale analyzing ideas to error", n),
        Ok(_) => {}
        Err(e) => tracing::error!("Failed to reset stale analyzing ideas: {}", e),
    }

    let config = routes::AppConfig::from_db_and_env(Some(&db));
    let state = routes::AppState {
        db: Arc::new(db),
        config: Arc::new(RwLock::new(config)),
        pipeline_channels: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    };

    // 启动管道通道超时清理（每 60s 检查，5 分钟未完成的自动移除）
    state.spawn_channel_cleaner();

    let app = Router::new()
        // 页面路由 / Page routes
        .route("/", get(routes::index_page))
        .route("/search", get(routes::search_page))
        .route("/patent/:id", get(routes::patent_detail_page))
        .route("/ai", get(routes::ai_page))
        .route("/compare", get(routes::compare_page))
        .route("/idea", get(routes::idea_page))
        .route("/settings", get(routes::settings_page))
        // 设置 API / Settings API
        .route("/api/settings", get(routes::api_get_settings))
        .route("/api/settings/serpapi", post(routes::api_save_serpapi))
        .route("/api/settings/bing", post(routes::api_save_bing))
        .route("/api/settings/lens", post(routes::api_save_lens))
        .route("/api/settings/cnipr", post(routes::api_save_cnipr))
        .route("/api/settings/ai", post(routes::api_save_ai))
        .route("/api/settings/fallbacks", post(routes::api_save_fallbacks))
        // 搜索 API / Search API
        .route("/api/search", post(routes::api_search))
        .route("/api/search/stats", post(routes::api_search_stats))
        .route("/api/search/export", post(routes::api_export_csv))
        .route("/api/search/export/xlsx", post(routes::api_export_xlsx))
        .route("/api/search/online", post(routes::api_search_online))
        .route("/api/search/analyze", post(routes::api_ai_analyze_results))
        // 专利 API / Patent API
        .route("/api/patent/fetch", post(routes::api_fetch_patent))
        .route("/api/patent/enrich/:id", get(routes::api_enrich_patent))
        .route(
            "/api/patent/enrich-free/:id",
            get(routes::api_enrich_patent_free),
        )
        .route("/api/patent/pdf/:id", get(routes::api_patent_pdf))
        .route(
            "/api/patent/image-proxy",
            get(routes::api_patent_image_proxy),
        )
        .route(
            "/api/patent/similar/:id",
            get(routes::api_recommend_similar),
        )
        .route(
            "/api/patent/:id/legal-status",
            get(routes::api_patent_legal_status),
        )
        // AI 接口 / AI API
        .route("/api/ai/chat", post(routes::api_ai_chat))
        .route("/api/ai/chat/stream", post(routes::api_ai_chat_stream))
        .route("/api/ai/summarize", post(routes::api_ai_summarize))
        .route("/api/ai/compare", post(routes::api_ai_compare))
        .route("/api/ai/claims", post(routes::api_ai_claims_analysis))
        .route("/api/ai/risk", post(routes::api_ai_risk_assessment))
        .route(
            "/api/ai/compare-matrix",
            post(routes::api_ai_compare_matrix),
        )
        .route(
            "/api/ai/batch-summarize",
            post(routes::api_ai_batch_summarize),
        )
        // 创意验证 API / Idea API
        .route("/api/idea/submit", post(routes::api_idea_submit))
        .route("/api/idea/analyze", post(routes::api_idea_analyze))
        .route("/api/idea/pipeline", post(routes::api_idea_pipeline))
        .route("/api/ideas/batch-compare", post(routes::api_ideas_batch_compare))
        .route("/api/idea/list", get(routes::api_idea_list))
        .route("/api/idea/:id", get(routes::api_idea_get))
        .route("/api/idea/:id/delete", post(routes::api_idea_delete))
        .route("/api/idea/:id/progress", get(routes::api_idea_progress))
        .route("/api/idea/:id/resume", post(routes::api_idea_resume))
        .route("/api/idea/:id/report", get(routes::api_idea_report))
        .route("/api/idea/:id/report.html", get(routes::api_idea_report_html))
        .route("/api/idea/:id/evidence", get(routes::api_idea_evidence))
        .route("/api/idea/:id/chat", post(routes::api_idea_chat))
        .route("/api/idea/:id/messages", get(routes::api_idea_messages))
        .route(
            "/api/idea/:id/summarize",
            post(routes::api_idea_summarize_discussion),
        )
        // 特征卡片 API / Feature cards API
        .route(
            "/api/ideas/:id/feature-cards",
            get(routes::api_get_feature_cards).post(routes::api_create_feature_card),
        )
        .route("/api/feature-cards/diff", get(routes::api_feature_card_diff))
        // 版本管理 + 迭代 API / Version management + iterate API
        .route("/api/idea/:id/claim-tree", get(routes::api_idea_claim_tree))
        .route("/api/idea/:id/iterate", post(routes::api_idea_iterate))
        .route("/api/idea/:id/versions", get(routes::api_idea_versions))
        .route("/api/idea/:id/branches", get(routes::api_idea_branches))
        .route("/api/idea/:id/findings", get(routes::api_idea_findings))
        // IPC 分类 API / IPC Classification API
        .route("/api/ipc/tree", get(routes::api_ipc_tree))
        .route("/api/ipc/:code/patents", get(routes::api_ipc_patents))
        // 导入 API / Import API
        .route("/api/patents/import", post(routes::api_import_patents))
        // 收藏夹 API / Collections API
        .route(
            "/api/collections",
            get(routes::api_list_collections).post(routes::api_create_collection),
        )
        .route(
            "/api/collections/:id",
            axum::routing::delete(routes::api_delete_collection),
        )
        .route(
            "/api/collections/:id/patents",
            get(routes::api_get_collection_patents),
        )
        .route(
            "/api/collections/:id/add",
            post(routes::api_add_to_collection),
        )
        .route(
            "/api/collections/:id/remove/:patent_id",
            axum::routing::delete(routes::api_remove_from_collection),
        )
        // 标签 API / Tags API
        .route(
            "/api/patents/:id/tags",
            get(routes::api_get_patent_tags).post(routes::api_add_tag),
        )
        .route(
            "/api/patents/:id/tags/:tag",
            axum::routing::delete(routes::api_remove_tag),
        )
        .route(
            "/api/patents/:id/collections",
            get(routes::api_get_patent_collections),
        )
        .route("/api/tags", get(routes::api_list_all_tags))
        // 文件上传 / File upload
        .route("/api/upload/compare", post(routes::api_upload_compare))
        .route("/api/upload/extract", post(routes::api_upload_extract))
        // 静态资源（内嵌二进制）/ Static files (embedded in binary)
        .route("/static/*path", get(serve_static))
        // 备用前端路径（桌面端已拆到独立仓库 innoforge-desktop）
        // 请求体大小限制（10MB）/ Body size limit (10MB)
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        // 跨域支持（MCP 客户端等需要）/ CORS for MCP clients and external frontends
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        // 安全响应头 / Security headers
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("创研台 InnoForge running at http://{addr}");
    println!("Local access: http://127.0.0.1:3000");

    // 自动打开浏览器（设置 INNOFORGE_NO_OPEN 可禁用）
    // Auto-open browser (disabled when INNOFORGE_NO_OPEN is set)
    if std::env::var("INNOFORGE_NO_OPEN").is_err() {
        let url = "http://127.0.0.1:3000/";
        if let Err(e) = open::that(url) {
            println!("Could not open browser: {}", e);
            println!("Please visit: {}", url);
        }
    }

    // 显示局域网 IP（方便手机访问）/ Show local IP for mobile access
    if let Ok(local_ip) = local_ip_address::local_ip() {
        println!("Mobile access: http://{}:3000", local_ip);
    }

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
