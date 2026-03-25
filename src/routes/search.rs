use super::{build_online_query, escape_csv, parse_search_type, AppState};
use crate::patent::*;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;

pub async fn api_search(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<SearchResult> {
    let search_type = parse_search_type(req.search_type.as_deref());
    let (mut patents, total, detected_type) = match s.db.search_smart(
        &req.query,
        search_type.as_ref(),
        req.country.as_deref(),
        req.date_from.as_deref(),
        req.date_to.as_deref(),
        req.page,
        req.page_size,
    ) {
        Ok((patents, total, search_type)) => (patents, total, search_type),
        Err(e) => {
            tracing::error!("search_smart failed: {}", e);
            (vec![], 0, SearchType::Mixed)
        }
    };

    // IPC/CPC post-filtering: batch-fetch patents to avoid N+1 queries
    let ipc_filter = req.ipc.as_deref().unwrap_or("").trim().to_lowercase();
    let cpc_filter = req.cpc.as_deref().unwrap_or("").trim().to_lowercase();
    if !ipc_filter.is_empty() || !cpc_filter.is_empty() {
        let mut cache: std::collections::HashMap<String, crate::patent::Patent> =
            std::collections::HashMap::new();
        for p in &patents {
            if let Ok(Some(full)) = s.db.get_patent(&p.id) {
                cache.insert(p.id.clone(), full);
            }
        }
        patents.retain(|p| {
            let matches_ipc = if ipc_filter.is_empty() {
                true
            } else if let Some(full) = cache.get(&p.id) {
                full.ipc_codes.to_lowercase().contains(&ipc_filter)
            } else {
                false
            };
            let matches_cpc = if cpc_filter.is_empty() {
                true
            } else if let Some(full) = cache.get(&p.id) {
                full.cpc_codes.to_lowercase().contains(&cpc_filter)
            } else {
                false
            };
            matches_ipc && matches_cpc
        });
    }

    // Deduplication: remove patents with same base number (e.g. CN123456A vs CN123456B)
    let pre_dedup_count = patents.len();
    let mut seen_base_numbers = std::collections::HashSet::new();
    patents.retain(|p| {
        let base = normalize_patent_number(&p.patent_number);
        seen_base_numbers.insert(base)
    });
    let dedup_removed = pre_dedup_count - patents.len();

    if let Some(sort_by) = req.sort_by.as_deref() {
        match sort_by {
            "new" => patents.sort_by(|a, b| b.filing_date.cmp(&a.filing_date)),
            "old" => patents.sort_by(|a, b| a.filing_date.cmp(&b.filing_date)),
            _ => sort_by_relevance(&mut patents),
        }
    } else {
        match detected_type {
            SearchType::Inventor | SearchType::Applicant => {
                sort_by_relevance(&mut patents);
                patents.retain(|p| p.relevance_score.unwrap_or(0.0) >= 50.0);
            }
            _ => sort_by_relevance(&mut patents),
        }
    }

    // Build category statistics for large result sets
    let categories = if patents.len() >= 10 {
        let mut by_applicant: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut by_country: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for p in &patents {
            let app = if p.applicant.is_empty() { "未知".to_string() } else {
                // Normalize applicant name (take first 20 chars to group variants)
                p.applicant.chars().take(20).collect()
            };
            *by_applicant.entry(app).or_insert(0) += 1;
            let country = if p.country.is_empty() { "未知".to_string() } else { p.country.clone() };
            *by_country.entry(country).or_insert(0) += 1;
        }
        let mut groups: Vec<CategoryGroup> = Vec::new();
        // Top applicants
        let mut app_list: Vec<_> = by_applicant.into_iter().collect();
        app_list.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, count) in app_list.iter().take(5) {
            if *count >= 2 {
                groups.push(CategoryGroup { label: format!("申请人: {}", name), count: *count });
            }
        }
        // Countries
        let mut country_list: Vec<_> = by_country.into_iter().collect();
        country_list.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, count) in country_list.iter().take(5) {
            groups.push(CategoryGroup { label: format!("国家: {}", name), count: *count });
        }
        if groups.is_empty() { None } else { Some(groups) }
    } else {
        None
    };

    let search_type_str = match detected_type {
        SearchType::Applicant => "applicant",
        SearchType::Inventor => "inventor",
        SearchType::PatentNumber => "patent_number",
        SearchType::Keyword => "keyword",
        SearchType::Mixed => "mixed",
    };

    let final_total = if !ipc_filter.is_empty() || !cpc_filter.is_empty() {
        patents.len()
    } else if dedup_removed > 0 {
        total.saturating_sub(dedup_removed)
    } else {
        total
    };

    Json(SearchResult {
        patents,
        total: final_total,
        page: req.page,
        page_size: req.page_size,
        search_type: Some(search_type_str.to_string()),
        categories,
        dedup_removed,
    })
}

fn sort_by_relevance(patents: &mut [PatentSummary]) {
    patents.sort_by(|a, b| {
        let sa = a.relevance_score.unwrap_or(0.0);
        let sb = b.relevance_score.unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Normalize patent number for deduplication:
/// "CN123456A", "CN123456B", "CN123456C" -> "CN123456"
fn normalize_patent_number(num: &str) -> String {
    let num = num.trim().to_uppercase();
    // Strip trailing kind code (A, A1, A2, B, B1, B2, C, U, etc.)
    let trimmed = num.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c.is_ascii_digit());
    // But we need to be smarter: kind code is usually 1-2 chars at end after digits
    // E.g. "CN113557390B" -> strip "B", "US20110265482A1" -> strip "A1"
    let bytes = num.as_bytes();
    let mut end = bytes.len();
    // Strip trailing kind code (1-2 chars that are letters or letter+digit)
    if end > 2 {
        let last = bytes[end - 1];
        let second_last = bytes[end - 2];
        if last.is_ascii_digit() && second_last.is_ascii_alphabetic() {
            // e.g., A1, B2
            end -= 2;
        } else if last.is_ascii_alphabetic() && second_last.is_ascii_digit() {
            // e.g., ...0B
            end -= 1;
        }
    }
    let _ = trimmed; // suppress unused warning
    num[..end].to_string()
}

pub async fn api_search_online(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<serde_json::Value> {
    println!(
        "[ONLINE] query='{}' page={} country={:?}",
        req.query, req.page, req.country
    );
    let online_search_type = parse_search_type(req.search_type.as_deref());

    let api_key = s.config.read().unwrap().serpapi_key.clone();
    if !api_key.is_empty() && api_key != "your-serpapi-key-here" {
        let client = reqwest::Client::new();
        let serp_page = if req.page < 1 { 1 } else { req.page };
        let search_query = build_online_query(
            &req.query,
            online_search_type.as_ref(),
            req.date_from.as_deref(),
            req.date_to.as_deref(),
        );
        let country_param = match req.country.as_deref() {
            Some(c) if !c.is_empty() => format!("&country={}", c),
            _ => String::new(),
        };
        let sort_param = match req.sort_by.as_deref() {
            Some("new") => "&sort=new",
            Some("old") => "&sort=old",
            _ => "",
        };
        let url = format!(
            "https://serpapi.com/search.json?engine=google_patents&q={}&page={}{}{}&api_key={}",
            urlencoding::encode(&search_query),
            serp_page,
            country_param,
            sort_param,
            api_key
        );
        println!(
            "[ONLINE] SerpAPI query='{}' page={} country_param='{}'",
            search_query, serp_page, country_param
        );
        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                println!("[ONLINE] SerpAPI status: {}", status);
                if let Ok(body) = resp.text().await {
                    println!("[ONLINE] SerpAPI body len={}", body.len());
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(err) = json.get("error") {
                            println!("[ONLINE] SerpAPI error: {}", err);
                        } else {
                            let total = json["search_information"]["total_results"]
                                .as_u64()
                                .unwrap_or(0)
                                as usize;
                            let mut patents = Vec::new();
                            if let Some(results) = json["organic_results"].as_array() {
                                println!(
                                    "[ONLINE] SerpAPI results: {}, total: {}",
                                    results.len(),
                                    total
                                );
                                for (idx, r) in results.iter().enumerate() {
                                    let p = serp_to_patent(r);
                                    if !p.title.is_empty() {
                                        if let Err(e) = s.db.insert_patent(&p) {
                                            tracing::warn!("Failed to cache online patent {}: {}", p.patent_number, e);
                                        }
                                        // Hybrid relevance: position + content matching
                                        let position_score = (98.0 - idx as f64 * 3.0).max(30.0);
                                        let content_score = calculate_online_relevance(
                                            &req.query, &p.title, &p.abstract_text, &p.applicant,
                                        );
                                        let score = (position_score * 0.4 + content_score * 0.6).min(100.0);
                                        let source = format!("hybrid(pos:{:.0}+content:{:.0})", position_score, content_score);
                                        patents.push(PatentSummary {
                                            id: p.id.clone(),
                                            patent_number: p.patent_number.clone(),
                                            title: p.title.clone(),
                                            abstract_text: p.abstract_text.clone(),
                                            applicant: p.applicant.clone(),
                                            inventor: p.inventor.clone(),
                                            filing_date: p.filing_date.clone(),
                                            country: p.country.clone(),
                                            relevance_score: Some(score),
                                            score_source: Some(source),
                                        });
                                    }
                                }
                            }
                            if !patents.is_empty() {
                                return Json(json!({
                                    "patents": patents,
                                    "total": total,
                                    "page": req.page,
                                    "page_size": 10,
                                    "source": "serpapi"
                                }));
                            }
                            println!("[ONLINE] SerpAPI returned empty; fallback to local DB");
                        }
                    }
                }
            }
            Err(e) => println!("[ONLINE] SerpAPI request error: {}", e),
        }
    } else {
        println!("[ONLINE] No SERPAPI_KEY configured");
    }

    // Fallback 2: Google Patents direct (free, no API key, no VPN needed)
    println!("[ONLINE] Trying Google Patents direct (free)...");
    match search_google_patents_direct(&req, online_search_type.as_ref()).await {
        Ok((patents, total)) if !patents.is_empty() => {
            // Cache results to local DB
            for p in &patents {
                let full = Patent {
                    id: p.id.clone(),
                    patent_number: p.patent_number.clone(),
                    title: p.title.clone(),
                    abstract_text: p.abstract_text.clone(),
                    description: String::new(),
                    claims: String::new(),
                    applicant: p.applicant.clone(),
                    inventor: p.inventor.clone(),
                    filing_date: p.filing_date.clone(),
                    publication_date: String::new(),
                    grant_date: None,
                    ipc_codes: String::new(),
                    cpc_codes: String::new(),
                    priority_date: String::new(),
                    country: p.country.clone(),
                    kind_code: String::new(),
                    family_id: None,
                    legal_status: String::new(),
                    citations: "[]".into(),
                    cited_by: "[]".into(),
                    source: "google_patents_free".into(),
                    raw_json: String::new(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    images: "[]".into(),
                    pdf_url: String::new(),
                };
                let _ = s.db.insert_patent(&full);
            }
            return Json(json!({
                "patents": patents,
                "total": total,
                "page": req.page,
                "page_size": 10,
                "source": "google_patents_free"
            }));
        }
        Ok(_) => println!("[ONLINE] Google Patents direct returned empty"),
        Err(e) => println!("[ONLINE] Google Patents direct error: {}", e),
    }

    // Fallback 3: local DB search
    println!("[ONLINE] Falling back to local DB");
    let local = s
        .db
        .search_smart(
            &req.query,
            online_search_type.as_ref(),
            req.country.as_deref(),
            req.date_from.as_deref(),
            req.date_to.as_deref(),
            req.page,
            req.page_size,
        )
        .ok()
        .map(|(p, t, _)| (p, t));
    if let Some((patents, total)) = local {
        if total > 0 {
            return Json(json!({
                "patents": patents,
                "total": total,
                "page": req.page,
                "page_size": req.page_size,
                "source": "local"
            }));
        }
    }
    let enc = urlencoding::encode(&req.query);
    Json(json!({
        "patents": [], "total": 0, "page": 1, "page_size": 20,
        "google_url": format!("https://patents.google.com/?q={enc}&oq={enc}"),
        "message": "未找到结果，可尝试在 Google Patents 上搜索"
    }))
}

pub async fn api_search_stats(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<serde_json::Value> {
    let search_type = parse_search_type(req.search_type.as_deref());
    let all_results = match s.db.search_smart(
        &req.query,
        search_type.as_ref(),
        req.country.as_deref(),
        req.date_from.as_deref(),
        req.date_to.as_deref(),
        1,
        10000,
    ) {
        Ok((p, _, _)) => p,
        Err(_) => vec![],
    };

    let mut applicant_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut country_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut year_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for p in &all_results {
        let applicant = if p.applicant.is_empty() {
            "未知".to_string()
        } else {
            p.applicant.clone()
        };
        *applicant_counts.entry(applicant).or_insert(0) += 1;

        let country = if p.country.is_empty() {
            "未知".to_string()
        } else {
            p.country.clone()
        };
        *country_counts.entry(country).or_insert(0) += 1;

        let year = p.filing_date.chars().take(4).collect::<String>();
        if year.len() == 4 {
            *year_counts.entry(year).or_insert(0) += 1;
        }
    }

    let mut applicants: Vec<_> = applicant_counts.into_iter().collect();
    applicants.sort_by(|a, b| b.1.cmp(&a.1));
    let top_applicants: Vec<_> = applicants.into_iter().take(10).collect();

    let mut countries: Vec<_> = country_counts.into_iter().collect();
    countries.sort_by(|a, b| b.1.cmp(&a.1));

    let mut years: Vec<_> = year_counts.into_iter().collect();
    years.sort_by(|a, b| a.0.cmp(&b.0));

    Json(json!({
        "total": all_results.len(),
        "applicants": top_applicants,
        "countries": countries,
        "years": years,
    }))
}

pub async fn api_export_csv(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> axum::response::Response {
    let search_type = parse_search_type(req.search_type.as_deref());
    let all_results = match s.db.search_smart(
        &req.query,
        search_type.as_ref(),
        req.country.as_deref(),
        req.date_from.as_deref(),
        req.date_to.as_deref(),
        1,
        10000,
    ) {
        Ok((p, _, _)) => p,
        Err(_) => vec![],
    };

    if all_results.is_empty() {
        return (StatusCode::NOT_FOUND, "No results to export").into_response();
    }

    let mut csv_data =
        String::from("专利号,标题,申请人,发明人,申请日,公开日,国家/地区,摘要\n");
    for p in all_results {
        let abstract_preview: String = p.abstract_text.chars().take(150).collect();
        let row = format!(
            "{},{},{},{},{},{},{},{}\n",
            escape_csv(&p.patent_number),
            escape_csv(&p.title),
            escape_csv(&p.applicant),
            escape_csv(&p.inventor),
            escape_csv(&p.filing_date),
            escape_csv(&p.filing_date),
            escape_csv(&p.country),
            escape_csv(&abstract_preview)
        );
        csv_data.push_str(&row);
    }

    let filename = format!(
        "patents_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        format!("\u{FEFF}{}", csv_data),
    )
        .into_response()
}

pub async fn api_export_xlsx(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    let search_type = parse_search_type(req.search_type.as_deref());
    let all_results = match s.db.search_smart(
        &req.query,
        search_type.as_ref(),
        req.country.as_deref(),
        req.date_from.as_deref(),
        req.date_to.as_deref(),
        1,
        10000,
    ) {
        Ok((p, _, _)) => p,
        Err(_) => vec![],
    };

    if all_results.is_empty() {
        return (StatusCode::NOT_FOUND, "No results to export").into_response();
    }

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();

    // Header style
    let header_format = rust_xlsxwriter::Format::new().set_bold();

    let headers = ["Patent No.", "Title", "Applicant", "Inventor", "Filing Date", "Country", "Abstract"];
    for (col, h) in headers.iter().enumerate() {
        let _ = sheet.write_string_with_format(0, col as u16, *h, &header_format);
    }

    for (row, p) in all_results.iter().enumerate() {
        let r = (row + 1) as u32;
        let _ = sheet.write_string(r, 0, &p.patent_number);
        let _ = sheet.write_string(r, 1, &p.title);
        let _ = sheet.write_string(r, 2, &p.applicant);
        let _ = sheet.write_string(r, 3, &p.inventor);
        let _ = sheet.write_string(r, 4, &p.filing_date);
        let _ = sheet.write_string(r, 5, &p.country);
        let abstract_preview: String = p.abstract_text.chars().take(200).collect();
        let _ = sheet.write_string(r, 6, &abstract_preview);
    }

    // Set column widths
    let _ = sheet.set_column_width(0, 18);
    let _ = sheet.set_column_width(1, 40);
    let _ = sheet.set_column_width(2, 25);
    let _ = sheet.set_column_width(3, 20);
    let _ = sheet.set_column_width(4, 12);
    let _ = sheet.set_column_width(5, 8);
    let _ = sheet.set_column_width(6, 50);

    match workbook.save_to_buffer() {
        Ok(buffer) => {
            let filename = format!(
                "patents_{}.xlsx",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
                ],
                buffer,
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate Excel: {}", e),
        )
            .into_response(),
    }
}


pub(crate) fn serp_to_patent(r: &serde_json::Value) -> Patent {
    let pub_num = r["publication_number"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let country = pub_num.chars().take(2).collect::<String>();
    Patent {
        id: uuid::Uuid::new_v4().to_string(),
        patent_number: pub_num,
        title: r["title"].as_str().unwrap_or("").to_string(),
        abstract_text: r["snippet"].as_str().unwrap_or("").to_string(),
        description: String::new(),
        claims: String::new(),
        applicant: r["assignee"].as_str().unwrap_or("").to_string(),
        inventor: r["inventor"].as_str().unwrap_or("").to_string(),
        filing_date: r["filing_date"].as_str().unwrap_or("").to_string(),
        publication_date: r["publication_date"].as_str().unwrap_or("").to_string(),
        grant_date: r["grant_date"].as_str().map(|s| s.to_string()),
        ipc_codes: String::new(),
        cpc_codes: String::new(),
        priority_date: r["priority_date"].as_str().unwrap_or("").to_string(),
        country,
        kind_code: String::new(),
        family_id: None,
        legal_status: String::new(),
        citations: "[]".into(),
        cited_by: "[]".into(),
        source: "serpapi".into(),
        raw_json: r.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        images: "[]".into(),
        pdf_url: String::new(),
    }
}

// ── Google Patents Direct Search (free, no API key, no VPN) ──────────────────

async fn search_google_patents_direct(
    req: &SearchRequest,
    search_type: Option<&SearchType>,
) -> Result<(Vec<PatentSummary>, usize), String> {
    let query = build_online_query(
        &req.query,
        search_type,
        req.date_from.as_deref(),
        req.date_to.as_deref(),
    );
    let country_filter = match req.country.as_deref() {
        Some(c) if !c.is_empty() => format!(" country:{}", c),
        _ => String::new(),
    };
    let full_query = format!("{}{}", query, country_filter);
    let page = req.page.saturating_sub(1);
    let num = 10;

    let url = format!(
        "https://patents.google.com/xhr/query?url=q%3D{}&exp=&num={}&page={}",
        urlencoding::encode(&full_query),
        num,
        page
    );
    println!("[FREE] Google Patents direct URL: {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("JSON parse: {}", e))?;

    let total = json["results"]["total_num_results"]
        .as_u64()
        .unwrap_or(0) as usize;

    let mut patents = Vec::new();
    if let Some(clusters) = json["results"]["cluster"].as_array() {
        for cluster in clusters {
            if let Some(results) = cluster["result"].as_array() {
                for r in results {
                    let pat = &r["patent"];
                    let id_str = r["id"].as_str().unwrap_or("");
                    // id format: "patent/CN113557390B/zh" -> extract patent number
                    let parts: Vec<&str> = id_str.split('/').collect();
                    let patent_number = if parts.len() >= 2 { parts[1] } else { id_str };
                    let country = patent_number.chars().take(2).collect::<String>();

                    // Clean HTML tags from title and snippet
                    let title = strip_html_tags(
                        pat["title"].as_str().unwrap_or(""),
                    );
                    let snippet = strip_html_tags(
                        pat["snippet"].as_str().unwrap_or(""),
                    );

                    let filing_date = pat["filing_date"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let applicant = pat["assignee_localized"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|v| v["value"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let inventor = pat["inventor_localized"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|v| v["value"].as_str())
                        .unwrap_or("")
                        .to_string();

                    if !title.is_empty() {
                        let content_score = calculate_online_relevance(
                            &query, &title, &snippet, &applicant,
                        );
                        patents.push(PatentSummary {
                            id: uuid::Uuid::new_v4().to_string(),
                            patent_number: patent_number.to_string(),
                            title,
                            abstract_text: snippet,
                            applicant,
                            inventor,
                            filing_date,
                            country,
                            relevance_score: Some(content_score),
                            score_source: Some("content-match".to_string()),
                        });
                    }
                }
            }
        }
    }

    println!("[FREE] Google Patents direct: {} results, total {}", patents.len(), total);
    Ok((patents, total))
}

/// Calculate content-based relevance for online search results.
fn calculate_online_relevance(query: &str, title: &str, abstract_text: &str, applicant: &str) -> f64 {
    let q = query.trim().to_lowercase();
    let t = title.trim().to_lowercase();
    let a = abstract_text.trim().to_lowercase();
    let app = applicant.trim().to_lowercase();

    let mut score = 30.0;

    // Title matching (most important)
    if t == q { score += 50.0; }
    else if t.contains(&q) { score += 35.0; }
    else {
        // Word-level matching in title
        let q_words: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
        if !q_words.is_empty() {
            let matches = q_words.iter().filter(|w| t.contains(*w)).count();
            score += (matches as f64 / q_words.len() as f64) * 30.0;
        }
        // Chinese character matching in title
        let q_chars: Vec<char> = q.chars().filter(|c| *c > '\u{4E00}' && *c < '\u{9FFF}').collect();
        if !q_chars.is_empty() {
            let matches = q_chars.iter().filter(|c| t.contains(**c)).count();
            score += (matches as f64 / q_chars.len() as f64) * 25.0;
        }
    }

    // Abstract matching (secondary)
    if a.contains(&q) { score += 15.0; }
    else {
        let q_words: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
        if !q_words.is_empty() {
            let matches = q_words.iter().filter(|w| a.contains(*w)).count();
            score += (matches as f64 / q_words.len() as f64) * 10.0;
        }
    }

    // Applicant matching (bonus)
    if app.contains(&q) { score += 5.0; }

    score.min(100.0)
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}
