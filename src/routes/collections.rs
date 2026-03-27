use super::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

pub async fn api_create_collection(
    State(s): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = req["name"].as_str().unwrap_or("").trim();
    let description = req["description"].as_str().unwrap_or("").trim();

    if name.is_empty() || name.len() > 100 {
        return Json(json!({"error": "Collection name is required (max 100 chars)"}));
    }

    let id = uuid::Uuid::new_v4().to_string();
    match s.db.create_collection(&id, name, description) {
        Ok(()) => Json(json!({"status": "ok", "id": id, "name": name})),
        Err(e) => Json(json!({"error": format!("Failed to create collection: {}", e)})),
    }
}

pub async fn api_list_collections(State(s): State<AppState>) -> Json<serde_json::Value> {
    match s.db.list_collections() {
        Ok(cols) => {
            let list: Vec<serde_json::Value> = cols
                .into_iter()
                .map(|(id, name, desc, count, created_at)| {
                    json!({
                        "id": id,
                        "name": name,
                        "description": desc,
                        "patent_count": count,
                        "created_at": created_at,
                    })
                })
                .collect();
            Json(json!({"collections": list}))
        }
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_delete_collection(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.delete_collection(&id) {
        Ok(()) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_add_to_collection(
    State(s): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let patent_id = req["patent_id"].as_str().unwrap_or("").trim();
    if patent_id.is_empty() {
        return Json(json!({"error": "patent_id is required"}));
    }

    // 验证专利 ID 是否存在
    match s.db.get_patent(patent_id) {
        Ok(None) => return Json(json!({"error": "专利不存在"})),
        Err(_) => return Json(json!({"error": "查询专利失败"})),
        Ok(Some(_)) => {}
    }

    match s.db.add_to_collection(patent_id, &collection_id) {
        Ok(()) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_remove_from_collection(
    State(s): State<AppState>,
    Path((collection_id, patent_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match s.db.remove_from_collection(&patent_id, &collection_id) {
        Ok(()) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_get_collection_patents(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_collection_patents(&id) {
        Ok(patents) => Json(json!({"patents": patents})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_add_tag(
    State(s): State<AppState>,
    Path(patent_id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let tag = req["tag"].as_str().unwrap_or("").trim();
    if tag.is_empty() || tag.len() > 50 {
        return Json(json!({"error": "Tag is required (max 50 chars)"}));
    }

    match s.db.add_tag(&patent_id, tag) {
        Ok(()) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_remove_tag(
    State(s): State<AppState>,
    Path((patent_id, tag)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match s.db.remove_tag(&patent_id, &tag) {
        Ok(()) => Json(json!({"status": "ok"})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_get_patent_tags(
    State(s): State<AppState>,
    Path(patent_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_patent_tags(&patent_id) {
        Ok(tags) => Json(json!({"tags": tags})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_list_all_tags(State(s): State<AppState>) -> Json<serde_json::Value> {
    match s.db.list_all_tags() {
        Ok(tags) => {
            let list: Vec<serde_json::Value> = tags
                .into_iter()
                .map(|(tag, count)| json!({"tag": tag, "count": count}))
                .collect();
            Json(json!({"tags": list}))
        }
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}

pub async fn api_get_patent_collections(
    State(s): State<AppState>,
    Path(patent_id): Path<String>,
) -> Json<serde_json::Value> {
    match s.db.get_patent_collections(&patent_id) {
        Ok(collection_ids) => Json(json!({"collection_ids": collection_ids})),
        Err(e) => Json(json!({"error": format!("{}", e)})),
    }
}
