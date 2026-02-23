use axum::{extract::{Path, State}, response::Html, Json};
use std::sync::Arc;
use serde_json::json;
use crate::{ai::AiClient, db::Database, patent::*};
#[derive(Clone)]
pub struct AppState { pub db: Arc<Database>, pub ai_client: reqwest::Client, pub ai_model: String }

pub async fn index_page() -> Html<String> { Html(include_str!("../templates/index.html").to_string()) }
pub async fn search_page() -> Html<String> { Html(include_str!("../templates/search.html").to_string()) }
pub async fn ai_page() -> Html<String> { Html(include_str!("../templates/ai.html").to_string()) }
pub async fn compare_page() -> Html<String> { Html(include_str!("../templates/compare.html").to_string()) }

pub async fn patent_detail_page(Path(id): Path<String>, State(s): State<AppState>) -> Html<String> {
    let t = include_str!("../templates/patent_detail.html");
    match s.db.get_patent(&id) {
        Ok(Some(p)) => Html(t.replace("{{patent_number}}", &p.patent_number).replace("{{title}}", &p.title)
            .replace("{{abstract_text}}", &p.abstract_text).replace("{{description}}", &p.description)
            .replace("{{claims}}", &p.claims).replace("{{applicant}}", &p.applicant)
            .replace("{{inventor}}", &p.inventor).replace("{{filing_date}}", &p.filing_date)
            .replace("{{publication_date}}", &p.publication_date).replace("{{grant_date}}", &p.grant_date.unwrap_or_default())
            .replace("{{ipc_codes}}", &p.ipc_codes).replace("{{cpc_codes}}", &p.cpc_codes)
            .replace("{{country}}", &p.country).replace("{{legal_status}}", &p.legal_status).replace("{{id}}", &p.id)),
        _ => Html("<h1>Not found</h1><a href='/search'>Back</a>".into()),
    }
}

pub async fn api_search(State(s): State<AppState>, Json(req): Json<SearchRequest>) -> Json<SearchResult> {
    match s.db.search_fts(&req.query, req.page, req.page_size) {
        Ok((patents, total)) if !patents.is_empty() => Json(SearchResult { patents, total, page: req.page, page_size: req.page_size }),
        _ => match s.db.search_like(&req.query, req.country.as_deref(), req.page, req.page_size) {
            Ok((patents, total)) => Json(SearchResult { patents, total, page: req.page, page_size: req.page_size }),
            Err(_) => Json(SearchResult { patents: vec![], total: 0, page: 1, page_size: 20 }),
        }
    }
}

pub async fn api_fetch_patent(State(s): State<AppState>, Json(req): Json<FetchPatentRequest>) -> Json<serde_json::Value> {
    let src = req.source.as_deref().unwrap_or("epo");
    match fetch_patent(&req.patent_number, src).await {
        Ok(p) => { let _ = s.db.insert_patent(&p); Json(serde_json::json!({"status":"ok","patent":p})) }
        Err(e) => Json(serde_json::json!({"status":"error","message":e.to_string()})),
    }
}

pub async fn api_ai_chat(State(s): State<AppState>, Json(req): Json<AiChatRequest>) -> Json<AiResponse> {
    let ai = AiClient::new();
    let ctx = req.patent_id.as_ref().and_then(|pid| s.db.get_patent(pid).ok().flatten())
        .map(|p| format!("Patent: {}\nTitle: {}\nAbstract: {}\nClaims: {}", p.patent_number, p.title, p.abstract_text, &p.claims[..p.claims.len().min(3000)]));
    match ai.chat(&req.message, ctx.as_deref()).await {
        Ok(content) => Json(AiResponse { content }),
        Err(e) => Json(AiResponse { content: format!("AI error: {e}") }),
    }
}

pub async fn api_ai_summarize(State(s): State<AppState>, Json(req): Json<FetchPatentRequest>) -> Json<AiResponse> {
    let ai = AiClient::new();
    match s.db.get_patent(&req.patent_number) {
        Ok(Some(p)) => match ai.summarize_patent(&p.title, &p.abstract_text, &p.claims).await {
            Ok(content) => Json(AiResponse { content }),
            Err(e) => Json(AiResponse { content: format!("AI error: {e}") }),
        },
        _ => Json(AiResponse { content: "Patent not found".into() }),
    }
}


pub async fn api_ai_compare(State(s): State<AppState>, Json(req): Json<serde_json::Value>) -> Json<AiResponse> {
    let id1 = req["patent_id1"].as_str().unwrap_or("");
    let id2 = req["patent_id2"].as_str().unwrap_or("");
    
    let p1 = match s.db.get_patent(id1) {
        Ok(Some(p)) => p,
        _ => return Json(AiResponse { content: "专利1未找到".into() }),
    };
    let p2 = match s.db.get_patent(id2) {
        Ok(Some(p)) => p,
        _ => return Json(AiResponse { content: "专利2未找到".into() }),
    };
    
    let ai = AiClient::new();
    let prompt = format!(
        "请对比分析以下两个专利的异同：\n\n\
         【专利1】\n\
         专利号：{}\n\
         标题：{}\n\
         申请人：{}\n\
         摘要：{}\n\
         权利要求（前部分）：{}\n\n\
         【专利2】\n\
         专利号：{}\n\
         标题：{}\n\
         申请人：{}\n\
         摘要：{}\n\
         权利要求（前部分）：{}\n\n\
         请从以下方面对比：\n\
         1. 技术领域是否相同\n\
         2. 解决的技术问题对比\n\
         3. 技术方案的异同点\n\
         4. 创新点对比\n\
         5. 保护范围对比\n\
         6. 是否存在侵权风险（初步判断）",
        p1.patent_number, p1.title, p1.applicant, 
        &p1.abstract_text[..p1.abstract_text.len().min(500)],
        &p1.claims[..p1.claims.len().min(1000)],
        p2.patent_number, p2.title, p2.applicant,
        &p2.abstract_text[..p2.abstract_text.len().min(500)],
        &p2.claims[..p2.claims.len().min(1000)]
    );
    
    match ai.chat(&prompt, None).await {
        Ok(content) => Json(AiResponse { content }),
        Err(e) => Json(AiResponse { content: format!("AI error: {e}") }),
    }
}
pub async fn api_import_patents(State(s): State<AppState>, Json(req): Json<ImportRequest>) -> Json<serde_json::Value> {
    let mut n = 0;
    for p in &req.patents { if s.db.insert_patent(p).is_ok() { n += 1; } }
    Json(serde_json::json!({"status":"ok","imported":n}))
}

pub async fn api_search_stats(State(s): State<AppState>, Json(req): Json<SearchRequest>) -> Json<serde_json::Value> {
    // Get all results (no pagination) for stats
    let all_results = match s.db.search_fts(&req.query, 1, 10000) {
        Ok((p, _)) if !p.is_empty() => p,
        _ => match s.db.search_like(&req.query, req.country.as_deref(), 1, 10000) {
            Ok((p, _)) => p,
            _ => vec![],
        }
    };
    
    // Count by applicant
    let mut applicant_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut country_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut year_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    
    for p in &all_results {
        let applicant = if p.applicant.is_empty() { "未知".to_string() } else { p.applicant.clone() };
        *applicant_counts.entry(applicant).or_insert(0) += 1;
        
        let country = if p.country.is_empty() { "未知".to_string() } else { p.country.clone() };
        *country_counts.entry(country).or_insert(0) += 1;
        
        let year = p.filing_date.chars().take(4).collect::<String>();
        if year.len() == 4 {
            *year_counts.entry(year).or_insert(0) += 1;
        }
    }
    
    // Sort and take top 10
    let mut applicants: Vec<_> = applicant_counts.into_iter().collect();
    applicants.sort_by(|a, b| b.1.cmp(&a.1));
    let top_applicants: Vec<_> = applicants.into_iter().take(10).collect();
    
    let mut countries: Vec<_> = country_counts.into_iter().collect();
    countries.sort_by(|a, b| b.1.cmp(&a.1));
    
    let mut years: Vec<_> = year_counts.into_iter().collect();
    years.sort_by(|a, b| a.0.cmp(&b.0));
    
    Json(serde_json::json!({
        "total": all_results.len(),
        "applicants": top_applicants,
        "countries": countries,
        "years": years,
    }))
}

pub async fn api_export_csv(State(s): State<AppState>, Json(req): Json<SearchRequest>) -> axum::response::Response {
    use axum::response::IntoResponse;
    use axum::http::{header, StatusCode};
    
    // Get all results
    let all_results = match s.db.search_fts(&req.query, 1, 10000) {
        Ok((p, _)) if !p.is_empty() => p,
        _ => match s.db.search_like(&req.query, req.country.as_deref(), 1, 10000) {
            Ok((p, _)) => p,
            _ => vec![],
        }
    };
    
    if all_results.is_empty() {
        return (StatusCode::NOT_FOUND, "No results to export").into_response();
    }
    
    // Build CSV
    let mut csv_data = String::from("专利号,标题,申请人,发明人,申请日,公开日,国家/地区,摘要\n");
    for p in all_results {
        let row = format!("{},{},{},{},{},{},{},{}\n",
            escape_csv(&p.patent_number),
            escape_csv(&p.title),
            escape_csv(&p.applicant),
            "",
            escape_csv(&p.filing_date),
            "",
            escape_csv(&p.country),
            escape_csv(&p.abstract_text[..p.abstract_text.len().min(200)])
        );
        csv_data.push_str(&row);
    }
    
    let filename = format!("patents_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
        ],
        format!("\u{FEFF}{}", csv_data) // Add BOM for Excel UTF-8 support
    ).into_response()
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
pub async fn api_search_online(State(s): State<AppState>, Json(req): Json<SearchRequest>) -> Json<serde_json::Value> {
    println!("[ONLINE] query='{}' page={} country={:?}", req.query, req.page, req.country);

    // Always try SerpAPI first for freshest results
    let api_key = std::env::var("SERPAPI_KEY").unwrap_or_default();
    if !api_key.is_empty() && api_key != "your-serpapi-key-here" {
        let client = reqwest::Client::new();
        let serp_page = if req.page < 1 { 1 } else { req.page };
        let search_query = req.query.clone();
        let country_param = match req.country.as_deref() {
            Some(c) if !c.is_empty() => format!("&country={}", c),
            _ => String::new(),
        };
        let url = format!(
            "https://serpapi.com/search.json?engine=google_patents&q={}&page={}{}&api_key={}",
            urlencoded(&search_query), serp_page, country_param, api_key
        );
        println!("[ONLINE] SerpAPI query='{}' page={} country_param='{}'", search_query, serp_page, country_param);
        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                println!("[ONLINE] SerpAPI status: {}", status);
                if let Ok(body) = resp.text().await {
                    println!("[ONLINE] SerpAPI body len={}", body.len());
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(err) = json.get("error") {
                            println!("[ONLINE] SerpAPI error: {}", err);
                            // Fall through to local search
                        } else {
                            let total = json["search_information"]["total_results"].as_u64().unwrap_or(0) as usize;
                            let mut patents = Vec::new();
                            if let Some(results) = json["organic_results"].as_array() {
                                println!("[ONLINE] SerpAPI results: {}, total: {}", results.len(), total);
                                for r in results {
                                    let p = serp_to_patent(r);
                                    if !p.title.is_empty() {
                                        let _ = s.db.insert_patent(&p);
                                        patents.push(PatentSummary {
                                            id: p.id.clone(), patent_number: p.patent_number.clone(),
                                            title: p.title.clone(), abstract_text: p.abstract_text.clone(),
                                            applicant: p.applicant.clone(), filing_date: p.filing_date.clone(),
                                            country: p.country.clone(),
                                        });
                                    }
                                }
                            }
                            return Json(serde_json::json!({"patents": patents, "total": total, "page": req.page, "page_size": 10, "source": "serpapi"}));
                        }
                    }
                }
            }
            Err(e) => println!("[ONLINE] SerpAPI request error: {}", e),
        }
    } else {
        println!("[ONLINE] No SERPAPI_KEY configured");
    }

    // Fallback: local DB search
    println!("[ONLINE] Falling back to local DB");
    let local = match s.db.search_fts(&req.query, req.page, req.page_size) {
        Ok((p, t)) if !p.is_empty() => Some((p, t)),
        _ => s.db.search_like(&req.query, req.country.as_deref(), req.page, req.page_size).ok()
    };
    if let Some((patents, total)) = local {
        if total > 0 {
            return Json(serde_json::json!({"patents": patents, "total": total, "page": req.page, "page_size": req.page_size, "source": "local"}));
        }
    }
    let enc = urlencoded(&req.query);
    Json(serde_json::json!({
        "patents": [], "total": 0, "page": 1, "page_size": 20,
        "google_url": format!("https://patents.google.com/?q={enc}&oq={enc}"),
        "message": "未找到结果，可尝试在 Google Patents 上搜索"
    }))
}

pub async fn api_enrich_patent(Path(id): Path<String>, State(s): State<AppState>) -> Json<serde_json::Value> {
    println!("[ENRICH] patent id={}", id);
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(serde_json::json!({"status":"error","message":"Patent not found"})),
    };
    // If already has claims (more than 10 chars), no need to enrich
    if patent.claims.len() > 10 {
        return Json(serde_json::json!({"status":"ok","message":"Already enriched","patent":patent}));
    }
    let api_key = std::env::var("SERPAPI_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Json(serde_json::json!({"status":"error","message":"SERPAPI_KEY not configured"}));
    }
    // Use SerpAPI google_patents_details engine
    let lang = if patent.country == "CN" || patent.patent_number.starts_with("CN") { "zh" } else { "en" };
    let patent_id_param = format!("patent/{}/{}", patent.patent_number, lang);
    let url = format!(
        "https://serpapi.com/search.json?engine=google_patents_details&patent_id={}&api_key={}",
        urlencoded(&patent_id_param), api_key
    );
    println!("[ENRICH] Fetching details for {}", patent.patent_number);
    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                println!("[ENRICH] Response len={}", body.len());
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(err) = json.get("error") {
                        return Json(serde_json::json!({"status":"error","message":format!("SerpAPI: {}", err)}));
                    }
                    let abstract_text = json["abstract"].as_str().unwrap_or("").to_string();
                    let claims_arr = json["claims"].as_array();
                    let claims = claims_arr.map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n\n")).unwrap_or_default();
                    let description = json["description"].as_str().unwrap_or("").to_string();
                    let ipc_codes = json["classifications"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                    let family_id = json["family_id"].as_str().map(|s| s.to_string());
                    // Build updated patent
                    let mut updated = patent.clone();
                    if !abstract_text.is_empty() { updated.abstract_text = abstract_text; }
                    if !claims.is_empty() { updated.claims = claims; }
                    if !description.is_empty() { updated.description = description; }
                    if !ipc_codes.is_empty() { updated.ipc_codes = ipc_codes; }
                    if family_id.is_some() { updated.family_id = family_id; }
                    // Extract legal events
                    if let Some(events) = json["legal_events"].as_array() {
                        let status_parts: Vec<String> = events.iter().take(3)
                            .filter_map(|e| {
                                let title = e["title"].as_str().unwrap_or("");
                                let date = e["date"].as_str().unwrap_or("");
                                if !title.is_empty() { Some(format!("{} ({})", title, date)) } else { None }
                            }).collect();
                        if !status_parts.is_empty() { updated.legal_status = status_parts.join("; "); }
                    }
                    let _ = s.db.insert_patent(&updated);
                    println!("[ENRICH] Updated patent {} with claims_len={} desc_len={}", updated.patent_number, updated.claims.len(), updated.description.len());
                    return Json(serde_json::json!({"status":"ok","patent":updated}));
                }
            }
        }
        Err(e) => println!("[ENRICH] Request error: {}", e),
    }
    Json(serde_json::json!({"status":"error","message":"Failed to fetch patent details"}))
}
fn serp_to_patent(r: &serde_json::Value) -> Patent {
    let pub_num = r["publication_number"].as_str().unwrap_or("").to_string();
    let country = pub_num.chars().take(2).collect::<String>();
    Patent {
        id: uuid::Uuid::new_v4().to_string(), patent_number: pub_num,
        title: r["title"].as_str().unwrap_or("").to_string(),
        abstract_text: r["snippet"].as_str().unwrap_or("").to_string(),
        description: String::new(), claims: String::new(),
        applicant: r["assignee"].as_str().unwrap_or("").to_string(),
        inventor: r["inventor"].as_str().unwrap_or("").to_string(),
        filing_date: r["filing_date"].as_str().unwrap_or("").to_string(),
        publication_date: r["publication_date"].as_str().unwrap_or("").to_string(),
        grant_date: r["grant_date"].as_str().map(|s| s.to_string()),
        ipc_codes: String::new(), cpc_codes: String::new(),
        priority_date: r["priority_date"].as_str().unwrap_or("").to_string(),
        country, kind_code: String::new(), family_id: None,
        legal_status: String::new(), citations: "[]".into(), cited_by: "[]".into(),
        source: "serpapi".into(), raw_json: r.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn fetch_patent(num: &str, source: &str) -> anyhow::Result<Patent> {
    let client = reqwest::Client::new();
    match source {
        "uspto" => fetch_uspto(&client, num).await,
        _ => fetch_epo(&client, num).await,
    }
}

async fn fetch_epo(client: &reqwest::Client, num: &str) -> anyhow::Result<Patent> {
    let url = format!("https://ops.epo.org/3.2/rest-services/published-data/publication/epodoc/{num}/biblio");
    let raw = client.get(&url).header("Accept", "application/json").send().await?.text().await?;
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    Ok(Patent {
        id: uuid::Uuid::new_v4().to_string(), patent_number: num.to_string(),
        title: efld(&json, "invention-title"), abstract_text: efld(&json, "abstract"),
        description: String::new(), claims: String::new(),
        applicant: efld(&json, "applicant"), inventor: efld(&json, "inventor"),
        filing_date: efld(&json, "date-of-filing"), publication_date: efld(&json, "date-of-publication"),
        grant_date: None, ipc_codes: efld(&json, "classification-ipc"), cpc_codes: String::new(),
        priority_date: String::new(), country: num.chars().take(2).collect(), kind_code: String::new(),
        family_id: None, legal_status: String::new(), citations: "[]".into(), cited_by: "[]".into(),
        source: "epo".into(), raw_json: raw, created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn fetch_uspto(client: &reqwest::Client, num: &str) -> anyhow::Result<Patent> {
    let clean = num.replace("US", "").replace("-", "");
    let url = format!("https://api.patentsview.org/patents/query?q={{\"patent_number\":\"{clean}\"}}&f=[\"patent_number\",\"patent_title\",\"patent_abstract\",\"patent_date\",\"assignee_organization\"]");
    let raw = client.get(&url).send().await?.text().await?;
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let p = json["patents"].as_array().and_then(|a| a.first());
    Ok(Patent {
        id: uuid::Uuid::new_v4().to_string(), patent_number: num.to_string(),
        title: p.and_then(|v| v["patent_title"].as_str()).unwrap_or("").into(),
        abstract_text: p.and_then(|v| v["patent_abstract"].as_str()).unwrap_or("").into(),
        description: String::new(), claims: String::new(),
        applicant: p.and_then(|v| v["assignee_organization"].as_str()).unwrap_or("").into(),
        inventor: String::new(),
        filing_date: p.and_then(|v| v["patent_date"].as_str()).unwrap_or("").into(),
        publication_date: String::new(), grant_date: None,
        ipc_codes: String::new(), cpc_codes: String::new(),
        priority_date: String::new(), country: "US".into(), kind_code: String::new(),
        family_id: None, legal_status: String::new(),
        citations: "[]".into(), cited_by: "[]".into(),
        source: "uspto".into(), raw_json: raw, created_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn urlencoded(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(*b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn efld(json: &serde_json::Value, field: &str) -> String {
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            if k == field {
                return match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(a) => a.iter().filter_map(|x| x.as_str().or_else(|| x["$"].as_str())).collect::<Vec<_>>().join(", "),
                    _ => v.to_string(),
                };
            }
            let r = efld(v, field);
            if !r.is_empty() { return r; }
        }
    } else if let Some(arr) = json.as_array() {
        for v in arr { let r = efld(v, field); if !r.is_empty() { return r; } }
    }
    String::new()
}
// 推荐相似专利
pub async fn api_recommend_similar(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };
    
    // 使用标题关键词搜索相似专利
    let keywords: Vec<&str> = patent.title.split_whitespace().take(5).collect();
    let query = keywords.join(" ");
    
    let req = SearchRequest { 
        query, 
        page: 1, 
        page_size: 10, 
        country: None, 
        date_from: None, 
        date_to: None,
        search_type: None,
    };
    
    match api_search_online(State(s), Json(req)).await {
        Json(result) => {
            if let Some(patents) = result.get("patents").and_then(|p| p.as_array()) {
                let filtered: Vec<_> = patents.iter()
                    .filter(|p| p.get("id").and_then(|i| i.as_str()) != Some(&id))
                    .take(5)
                    .collect();
                Json(json!({"similar": filtered}))
            } else {
                Json(json!({"similar": []}))
            }
        }
    }
}

// 上传文件对比
pub async fn api_upload_compare(
    State(s): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Json<serde_json::Value> {
    let mut file_content = String::new();
    let mut patent_id = String::new();
    
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            if let Ok(data) = field.bytes().await {
                if let Ok(text) = String::from_utf8(data.to_vec()) {
                    file_content = text;
                } else {
                    return Json(json!({"error": "暂不支持图片 OCR，请上传 TXT 文本文件"}));
                }
            }
        } else if name == "patent_id" {
            if let Ok(text) = field.text().await {
                patent_id = text;
            }
        }
    }
    
    if file_content.is_empty() || patent_id.is_empty() {
        return Json(json!({"error": "缺少文件或专利 ID"}));
    }
    
    let patent = match s.db.get_patent(&patent_id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };
    
    // 使用 AI 对比
    let ai_client = crate::ai::AiClient::new();

    let prompt = format!(

        "请对比以下两份技术文档，分析它们的相似性和差异：\n\n\
        【专利文档】\n标题：{}\n摘要：{}\n权利要求：{}\n\n\
        【上传文档】\n{}\n\n\
        请从以下方面分析：\n\
        1. 技术领域是否相同\n\
        2. 解决的技术问题是否相似\n\
        3. 技术方案的相似度（百分比）\n\
        4. 是否存在侵权风险\n\
        5. 主要差异点",
        patent.title,
        patent.abstract_text,
        patent.claims,
        file_content.chars().take(2000).collect::<String>()
    );
    
    match ai_client.chat(&prompt, None).await {
        Ok(response) => Json(json!({
            "success": true,
            "analysis": response
        })),
        Err(e) => Json(json!({"error": format!("AI 分析失败: {}", e)})),
    }


}




// 日期过滤辅助函数
fn filter_by_date(patents: Vec<PatentSummary>, date_from: Option<&str>, date_to: Option<&str>) -> Vec<PatentSummary> {
    if date_from.is_none() && date_to.is_none() {
        return patents;
    }
    patents.into_iter().filter(|p| {
        if p.filing_date.is_empty() {
            return true; // 保留没有日期的专利
        }
        let date = &p.filing_date;
        if let Some(from) = date_from {
            if date.as_str() < from {
                return false;
            }
        }
        if let Some(to) = date_to {
            if date.as_str() > to {
                return false;
            }
        }
        true
    }).collect()
}
