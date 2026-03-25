pub mod ai;
pub mod db;
pub mod patent;
pub mod skill_router;

// Re-export routes module for embedded server use
mod error;
mod routes;

/// Start the patent-hub web server. Can be called from Tauri or standalone.
/// This function blocks until the server shuts down.
pub async fn start_server(db_path: &str) -> anyhow::Result<()> {
    use axum::{
        extract::DefaultBodyLimit,
        http::HeaderValue,
        routing::{get, post},
        Router,
    };
    use std::net::SocketAddr;
    use std::sync::{Arc, RwLock};
    use tower_http::services::ServeDir;
    use tower_http::set_header::SetResponseHeaderLayer;

    let db = db::Database::init(db_path)?;
    let config = routes::AppConfig::from_env();
    let state = routes::AppState {
        db: Arc::new(db),
        config: Arc::new(RwLock::new(config)),
    };

    let app = Router::new()
        .route("/", get(routes::index_page))
        .route("/search", get(routes::search_page))
        .route("/patent/:id", get(routes::patent_detail_page))
        .route("/ai", get(routes::ai_page))
        .route("/compare", get(routes::compare_page))
        .route("/idea", get(routes::idea_page))
        .route("/settings", get(routes::settings_page))
        .route("/api/settings", get(routes::api_get_settings))
        .route("/api/settings/serpapi", post(routes::api_save_serpapi))
        .route("/api/settings/ai", post(routes::api_save_ai))
        .route("/api/settings/fallbacks", post(routes::api_save_fallbacks))
        .route("/api/search", post(routes::api_search))
        .route("/api/search/stats", post(routes::api_search_stats))
        .route("/api/search/export", post(routes::api_export_csv))
        .route("/api/search/export/xlsx", post(routes::api_export_xlsx))
        .route("/api/search/online", post(routes::api_search_online))
        .route("/api/search/analyze", post(routes::api_ai_analyze_results))
        .route("/api/patent/fetch", post(routes::api_fetch_patent))
        .route("/api/patent/enrich/:id", get(routes::api_enrich_patent))
        .route("/api/patent/enrich-free/:id", get(routes::api_enrich_patent_free))
        .route("/api/patent/pdf/:id", get(routes::api_patent_pdf))
        .route("/api/patent/image-proxy", get(routes::api_patent_image_proxy))
        .route("/api/patent/similar/:id", get(routes::api_recommend_similar))
        .route("/api/ai/chat", post(routes::api_ai_chat))
        .route("/api/ai/summarize", post(routes::api_ai_summarize))
        .route("/api/ai/compare", post(routes::api_ai_compare))
        .route("/api/ai/claims", post(routes::api_ai_claims_analysis))
        .route("/api/ai/risk", post(routes::api_ai_risk_assessment))
        .route("/api/ai/compare-matrix", post(routes::api_ai_compare_matrix))
        .route("/api/ai/batch-summarize", post(routes::api_ai_batch_summarize))
        .route("/api/idea/submit", post(routes::api_idea_submit))
        .route("/api/idea/analyze", post(routes::api_idea_analyze))
        .route("/api/idea/list", get(routes::api_idea_list))
        .route("/api/idea/:id", get(routes::api_idea_get))
        .route("/api/idea/:id/chat", post(routes::api_idea_chat))
        .route("/api/idea/:id/messages", get(routes::api_idea_messages))
        .route("/api/idea/:id/summarize", post(routes::api_idea_summarize_discussion))
        .route("/api/ipc/tree", get(routes::api_ipc_tree))
        .route("/api/ipc/:code/patents", get(routes::api_ipc_patents))
        .route("/api/patents/import", post(routes::api_import_patents))
        .route("/api/collections", get(routes::api_list_collections).post(routes::api_create_collection))
        .route("/api/collections/:id", axum::routing::delete(routes::api_delete_collection))
        .route("/api/collections/:id/patents", get(routes::api_get_collection_patents))
        .route("/api/collections/:id/add", post(routes::api_add_to_collection))
        .route("/api/collections/:id/remove/:patent_id", axum::routing::delete(routes::api_remove_from_collection))
        .route("/api/patents/:id/tags", get(routes::api_get_patent_tags).post(routes::api_add_tag))
        .route("/api/patents/:id/tags/:tag", axum::routing::delete(routes::api_remove_tag))
        .route("/api/patents/:id/collections", get(routes::api_get_patent_collections))
        .route("/api/tags", get(routes::api_list_all_tags))
        .route("/api/upload/compare", post(routes::api_upload_compare))
        .nest_service("/static", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
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
    println!("Patent Hub server starting at http://{addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
