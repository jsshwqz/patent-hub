use super::{efld, AppState};
use crate::patent::*;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::{Arc, RwLock};

pub async fn api_fetch_patent(
    State(s): State<AppState>,
    Json(req): Json<FetchPatentRequest>,
) -> Json<serde_json::Value> {
    let src = req.source.as_deref().unwrap_or("epo");
    match fetch_patent(&req.patent_number, src).await {
        Ok(p) => {
            if let Err(e) = s.db.insert_patent(&p) {
                tracing::warn!("Failed to cache patent {}: {}", p.patent_number, e);
            }
            Json(json!({"status":"ok","patent":p}))
        }
        Err(e) => Json(json!({"status":"error","message":e.to_string()})),
    }
}

pub async fn api_enrich_patent(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    println!("[ENRICH] patent id={}", id);
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"status":"error","message":"Patent not found"})),
    };
    // Only skip if we have claims AND images already
    let has_images = patent.images.len() > 5;
    if patent.claims.len() > 50 && has_images {
        return Json(json!({"status":"ok","message":"Already enriched","patent":patent}));
    }

    let api_key = s.config.read().unwrap().serpapi_key.clone();
    if api_key.is_empty() {
        return Json(json!({"status":"error","message":"SERPAPI_KEY not configured"}));
    }

    // SerpAPI Google Patents Details only supports "en" - "zh" returns no results
    let patent_id_param = format!("patent/{}/en", patent.patent_number);
    let url = format!(
        "https://serpapi.com/search.json?engine=google_patents_details&patent_id={}&api_key={}",
        urlencoding::encode(&patent_id_param),
        api_key
    );
    println!("[ENRICH] Fetching details for {}", patent.patent_number);
    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                println!("[ENRICH] Response len={}", body.len());
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(err) = json.get("error") {
                        return Json(
                            json!({"status":"error","message":format!("SerpAPI: {}", err)}),
                        );
                    }
                    let abstract_text = json["abstract"].as_str().unwrap_or("").to_string();
                    let claims_arr = json["claims"].as_array();
                    let claims = claims_arr
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        })
                        .unwrap_or_default();
                    let mut description = json["description"].as_str().unwrap_or("").to_string();
                    // SerpAPI returns description_link for long patents - try to fetch it
                    if description.is_empty() {
                        if let Some(desc_link) = json["description_link"].as_str() {
                            println!("[ENRICH] Fetching description from link: {}", desc_link);
                            if let Ok(desc_resp) = client
                                .get(desc_link)
                                .header("User-Agent", "Mozilla/5.0")
                                .send()
                                .await
                            {
                                if let Ok(desc_text) = desc_resp.text().await {
                                    // The link returns HTML, extract text content
                                    let clean = desc_text
                                        .replace("<br>", "\n")
                                        .replace("<br/>", "\n")
                                        .replace("<p>", "\n")
                                        .replace("</p>", "\n");
                                    // Strip remaining HTML tags
                                    let re = regex::Regex::new(r"<[^>]+>")
                                        .unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
                                    description = re.replace_all(&clean, "").trim().to_string();
                                    println!("[ENRICH] Got description len={}", description.len());
                                }
                            }
                        }
                    }
                    let ipc_codes = json["classifications"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| {
                                    // Handle both string and object formats
                                    v.as_str()
                                        .map(|s| s.to_string())
                                        .or_else(|| v["code"].as_str().map(|s| s.to_string()))
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    let family_id = json["family_id"].as_str().map(|s| s.to_string());
                    let mut updated = patent.clone();
                    if !abstract_text.is_empty() {
                        updated.abstract_text = abstract_text;
                    }
                    if !claims.is_empty() {
                        updated.claims = claims;
                    }
                    if !description.is_empty() {
                        updated.description = description;
                    }
                    if !ipc_codes.is_empty() {
                        updated.ipc_codes = ipc_codes;
                    }
                    if family_id.is_some() {
                        updated.family_id = family_id;
                    }
                    if let Some(events) = json["legal_events"].as_array() {
                        let status_parts: Vec<String> = events
                            .iter()
                            .take(3)
                            .filter_map(|e| {
                                let title = e["title"].as_str().unwrap_or("");
                                let date = e["date"].as_str().unwrap_or("");
                                if !title.is_empty() {
                                    Some(format!("{} ({})", title, date))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !status_parts.is_empty() {
                            updated.legal_status = status_parts.join("; ");
                        }
                    }
                    // Extract images
                    if let Some(images) = json["images"].as_array() {
                        let img_urls: Vec<&str> =
                            images.iter().filter_map(|v| v.as_str()).collect();
                        if !img_urls.is_empty() {
                            updated.images = serde_json::to_string(&img_urls).unwrap_or_default();
                        }
                    }
                    // Extract PDF URL
                    if let Some(pdf) = json["pdf"].as_str() {
                        updated.pdf_url = pdf.to_string();
                    }
                    if let Err(e) = s.db.insert_patent(&updated) {
                        tracing::warn!(
                            "Failed to save enriched patent {}: {}",
                            updated.patent_number,
                            e
                        );
                    }
                    println!(
                        "[ENRICH] Updated patent {} with claims_len={} desc_len={}",
                        updated.patent_number,
                        updated.claims.len(),
                        updated.description.len()
                    );
                    return Json(json!({"status":"ok","patent":updated}));
                }
            }
        }
        Err(e) => println!("[ENRICH] Request error: {}", e),
    }
    Json(json!({"status":"error","message":"Failed to fetch patent details"}))
}

/// Free patent detail fetch from Google Patents (no API key, no VPN needed)
pub async fn api_enrich_patent_free(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    println!("[ENRICH-FREE] patent id={}", id);
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"status":"error","message":"Patent not found"})),
    };
    // Already has full text
    if patent.description.len() > 50 && patent.claims.len() > 50 {
        return Json(json!({"status":"ok","message":"Already enriched","patent":patent}));
    }

    let pn = &patent.patent_number;
    let lang = if patent.country == "CN" || pn.starts_with("CN") {
        "zh"
    } else {
        "en"
    };

    // Try Google Patents HTML page directly
    let url = format!("https://patents.google.com/patent/{}/{}", pn, lang);
    println!("[ENRICH-FREE] Fetching {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    let resp = match client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(json!({"status":"error","message":format!("请求失败: {}", e)})),
    };

    if !resp.status().is_success() {
        return Json(json!({"status":"error","message":format!("HTTP {}", resp.status())}));
    }

    let html = match resp.text().await {
        Ok(t) => t,
        Err(e) => return Json(json!({"status":"error","message":format!("读取失败: {}", e)})),
    };

    println!("[ENRICH-FREE] HTML len={}", html.len());

    // Parse sections from Google Patents HTML
    let mut updated = patent.clone();

    // Extract abstract
    if patent.abstract_text.is_empty() {
        if let Some(abs) = extract_section(&html, "abstract") {
            updated.abstract_text = abs;
        }
    }

    // Extract claims
    if patent.claims.is_empty() || patent.claims.len() < 50 {
        if let Some(claims) = extract_section(&html, "claims") {
            updated.claims = claims;
        }
    }

    // Extract description
    if patent.description.is_empty() || patent.description.len() < 50 {
        if let Some(desc) = extract_section(&html, "description") {
            updated.description = desc;
        }
    }

    // Extract classifications
    if patent.ipc_codes.is_empty() {
        if let Some(ipc) = extract_classifications(&html) {
            updated.ipc_codes = ipc;
        }
    }

    // Save enriched data
    if let Err(e) = s.db.insert_patent(&updated) {
        tracing::warn!(
            "Failed to save enriched patent {}: {}",
            updated.patent_number,
            e
        );
    }

    println!(
        "[ENRICH-FREE] Updated {} | abstract={} claims={} desc={}",
        updated.patent_number,
        updated.abstract_text.len(),
        updated.claims.len(),
        updated.description.len()
    );

    Json(json!({"status":"ok","patent":updated}))
}

/// Extract a section from Google Patents HTML
fn extract_section(html: &str, section: &str) -> Option<String> {
    // Google Patents uses <section itemprop="abstract/claims/description">
    let marker = format!("itemprop=\"{}\"", section);
    let start = html.find(&marker)?;
    let section_html = &html[start..];

    // Find the closing </section>
    let end = section_html
        .find("</section>")
        .unwrap_or(section_html.len().min(200_000));
    let content = &section_html[..end];

    // Extract text from <div class="abstract">, <claim-text>, or <div class="description-paragraph">
    let mut parts: Vec<String> = Vec::new();

    // Generic: extract text between > and < for content divs
    let mut pos = 0;
    let bytes = content.as_bytes();
    while pos < bytes.len() {
        // Look for text content between tags
        if bytes[pos] == b'>' {
            pos += 1;
            let text_start = pos;
            while pos < bytes.len() && bytes[pos] != b'<' {
                pos += 1;
            }
            if pos > text_start {
                let text = &content[text_start..pos].trim();
                if !text.is_empty() && text.len() > 1 {
                    // Skip tag attributes and class names
                    if !text.starts_with("class=")
                        && !text.starts_with("itemprop=")
                        && !text.contains("data-")
                    {
                        parts.push(text.to_string());
                    }
                }
            }
        } else {
            pos += 1;
        }
    }

    if parts.is_empty() {
        return None;
    }

    let result = parts.join("\n\n");
    if result.len() < 10 {
        return None;
    }
    Some(result)
}

/// Extract IPC/CPC classifications from HTML
fn extract_classifications(html: &str) -> Option<String> {
    let mut codes = Vec::new();
    let marker = "itemprop=\"Code\"";
    let mut search_start = 0;
    while let Some(pos) = html[search_start..].find(marker) {
        let abs_pos = search_start + pos;
        // Look for content="..." after the marker
        if let Some(content_pos) = html[abs_pos..].find("content=\"") {
            let val_start = abs_pos + content_pos + 9;
            if let Some(val_end) = html[val_start..].find('"') {
                let code = &html[val_start..val_start + val_end];
                // Valid IPC/CPC codes contain letters and digits (e.g., F02K7/06)
                let is_valid = !code.is_empty()
                    && code.len() >= 3
                    && code != "true"
                    && code != "false"
                    && code.chars().any(|c| c.is_ascii_alphabetic())
                    && code.chars().any(|c| c.is_ascii_digit());
                if is_valid && !codes.contains(&code.to_string()) {
                    codes.push(code.to_string());
                }
            }
        }
        search_start = abs_pos + marker.len();
        if codes.len() >= 20 {
            break;
        }
    }
    if codes.is_empty() {
        None
    } else {
        Some(codes.join(", "))
    }
}

pub async fn api_recommend_similar(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };

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
        sort_by: None,
        ipc: None,
        cpc: None,
    };

    match super::api_search_online(State(s), Json(req)).await {
        Json(result) => {
            if let Some(patents) = result.get("patents").and_then(|p| p.as_array()) {
                let filtered: Vec<_> = patents
                    .iter()
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

async fn fetch_patent(num: &str, source: &str) -> anyhow::Result<Patent> {
    let client = reqwest::Client::new();
    match source {
        "uspto" => fetch_uspto(&client, num).await,
        _ => fetch_epo(&client, num).await,
    }
}

async fn fetch_epo(client: &reqwest::Client, num: &str) -> anyhow::Result<Patent> {
    let url = format!(
        "https://ops.epo.org/3.2/rest-services/published-data/publication/epodoc/{num}/biblio"
    );
    let raw = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?
        .text()
        .await?;
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    Ok(Patent {
        id: uuid::Uuid::new_v4().to_string(),
        patent_number: num.to_string(),
        title: efld(&json, "invention-title"),
        abstract_text: efld(&json, "abstract"),
        description: String::new(),
        claims: String::new(),
        applicant: efld(&json, "applicant"),
        inventor: efld(&json, "inventor"),
        filing_date: efld(&json, "date-of-filing"),
        publication_date: efld(&json, "date-of-publication"),
        grant_date: None,
        ipc_codes: efld(&json, "classification-ipc"),
        cpc_codes: String::new(),
        priority_date: String::new(),
        country: num.chars().take(2).collect(),
        kind_code: String::new(),
        family_id: None,
        legal_status: String::new(),
        citations: "[]".into(),
        cited_by: "[]".into(),
        source: "epo".into(),
        raw_json: raw,
        created_at: chrono::Utc::now().to_rfc3339(),
        images: "[]".into(),
        pdf_url: String::new(),
    })
}

async fn fetch_uspto(client: &reqwest::Client, num: &str) -> anyhow::Result<Patent> {
    let clean = num.replace("US", "").replace('-', "");
    let url = format!("https://api.patentsview.org/patents/query?q={{\"patent_number\":\"{clean}\"}}&f=[\"patent_number\",\"patent_title\",\"patent_abstract\",\"patent_date\",\"assignee_organization\"]");
    let raw = client.get(&url).send().await?.text().await?;
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let p = json["patents"].as_array().and_then(|a| a.first());
    Ok(Patent {
        id: uuid::Uuid::new_v4().to_string(),
        patent_number: num.to_string(),
        title: p
            .and_then(|v| v["patent_title"].as_str())
            .unwrap_or("")
            .into(),
        abstract_text: p
            .and_then(|v| v["patent_abstract"].as_str())
            .unwrap_or("")
            .into(),
        description: String::new(),
        claims: String::new(),
        applicant: p
            .and_then(|v| v["assignee_organization"].as_str())
            .unwrap_or("")
            .into(),
        inventor: String::new(),
        filing_date: p
            .and_then(|v| v["patent_date"].as_str())
            .unwrap_or("")
            .into(),
        publication_date: String::new(),
        grant_date: None,
        ipc_codes: String::new(),
        cpc_codes: String::new(),
        priority_date: String::new(),
        country: "US".into(),
        kind_code: String::new(),
        family_id: None,
        legal_status: String::new(),
        citations: "[]".into(),
        cited_by: "[]".into(),
        source: "uspto".into(),
        raw_json: raw,
        created_at: chrono::Utc::now().to_rfc3339(),
        images: "[]".into(),
        pdf_url: String::new(),
    })
}

/// Generate a printable HTML page for PDF export (browser Print → Save as PDF)
pub async fn api_patent_pdf(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> impl IntoResponse {
    let mut patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                "<h1>Patent not found</h1>".to_string(),
            );
        }
    };

    // Auto-enrich if missing full text
    if patent.description.len() < 50 || patent.claims.len() < 50 {
        println!(
            "[PDF] Auto-enriching patent {} before PDF generation",
            patent.patent_number
        );
        // Try SerpAPI enrich
        let api_key = s.config.read().unwrap().serpapi_key.clone();
        if !api_key.is_empty() {
            let lang = if patent.country == "CN" || patent.patent_number.starts_with("CN") {
                "zh"
            } else {
                "en"
            };
            let patent_id_param = format!("patent/{}/{}", patent.patent_number, lang);
            let url = format!(
                "https://serpapi.com/search.json?engine=google_patents_details&patent_id={}&api_key={}",
                urlencoding::encode(&patent_id_param), api_key
            );
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default();
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(body) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if json.get("error").is_none() {
                            let desc = json["description"].as_str().unwrap_or("");
                            if !desc.is_empty() {
                                patent.description = desc.to_string();
                            }
                            let abs = json["abstract"].as_str().unwrap_or("");
                            if !abs.is_empty() && patent.abstract_text.is_empty() {
                                patent.abstract_text = abs.to_string();
                            }
                            if let Some(claims_arr) = json["claims"].as_array() {
                                let claims = claims_arr
                                    .iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n\n");
                                if !claims.is_empty() {
                                    patent.claims = claims;
                                }
                            }
                            if let Some(images) = json["images"].as_array() {
                                let img_urls: Vec<&str> =
                                    images.iter().filter_map(|v| v.as_str()).collect();
                                if !img_urls.is_empty() {
                                    patent.images =
                                        serde_json::to_string(&img_urls).unwrap_or_default();
                                }
                            }
                            if let Some(pdf) = json["pdf"].as_str() {
                                patent.pdf_url = pdf.to_string();
                            }
                            // Save enriched data
                            let _ = s.db.insert_patent(&patent);
                            println!(
                                "[PDF] Enriched: desc={} claims={}",
                                patent.description.len(),
                                patent.claims.len()
                            );
                        }
                    }
                }
            }
        }
    }

    let esc = |s: &str| -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };

    let desc_html = if patent.description.is_empty() {
        "<p style='color:#999;'>说明书内容未加载。请先在详情页点击「加载全文」后再导出。</p>"
            .to_string()
    } else {
        format!(
            "<pre style='white-space:pre-wrap;font-family:SimSun,serif;line-height:1.8;'>{}</pre>",
            esc(&patent.description)
        )
    };

    // Build images HTML
    let images_html = {
        let imgs: Vec<String> = serde_json::from_str(&patent.images).unwrap_or_default();
        if imgs.is_empty() {
            String::new()
        } else {
            let mut html = String::from("<h2>附图</h2>\n");
            for (i, url) in imgs.iter().enumerate() {
                let proxy_url = format!("/api/patent/image-proxy?url={}", urlencoding::encode(url));
                html.push_str(&format!(
                    "<div style='text-align:center;margin:20px 0;page-break-inside:avoid;'>\
                     <img src='{}' alt='图 {}' style='max-width:100%;max-height:700px;'>\
                     <p style='color:#666;font-size:12px;'>图 {}</p></div>\n",
                    proxy_url,
                    i + 1,
                    i + 1
                ));
            }
            html
        }
    };

    let claims_html = if patent.claims.is_empty() {
        "<p style='color:#999;'>权利要求内容未加载。</p>".to_string()
    } else {
        format!(
            "<pre style='white-space:pre-wrap;font-family:SimSun,serif;line-height:1.8;'>{}</pre>",
            esc(&patent.claims)
        )
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<title>{number} - {title}</title>
<style>
@media print {{
    body {{ margin: 15mm; }}
    .no-print {{ display: none !important; }}
    h1 {{ font-size: 18pt; }}
    h2 {{ font-size: 14pt; page-break-before: always; }}
    h2:first-of-type {{ page-break-before: avoid; }}
}}
body {{ font-family: SimSun, 'Times New Roman', serif; max-width: 800px; margin: 0 auto; padding: 20px; color: #222; line-height: 1.6; }}
h1 {{ text-align: center; border-bottom: 2px solid #333; padding-bottom: 10px; }}
.meta-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
.meta-table td {{ border: 1px solid #ccc; padding: 6px 12px; }}
.meta-table td:first-child {{ background: #f5f5f5; font-weight: bold; width: 120px; }}
h2 {{ color: #1a5276; border-bottom: 1px solid #ccc; padding-bottom: 5px; margin-top: 30px; }}
.btn-print {{ display: inline-block; padding: 10px 30px; background: #238636; color: white; border: none; border-radius: 6px; font-size: 16px; cursor: pointer; margin: 20px auto; }}
.btn-print:hover {{ background: #2ea043; }}
.top-actions {{ text-align: center; margin-bottom: 20px; }}
</style>
</head>
<body>
<div class="no-print top-actions">
    <button class="btn-print" onclick="window.print()">🖨️ 打印 / 保存为 PDF</button>
    <button class="btn-print" style="background:#555;" onclick="window.close()">关闭</button>
</div>
<h1>{title}</h1>
<p style="text-align:center;color:#666;font-size:14px;">专利号: {number}</p>
<table class="meta-table">
<tr><td>申请人</td><td>{applicant}</td></tr>
<tr><td>发明人</td><td>{inventor}</td></tr>
<tr><td>申请日</td><td>{filing_date}</td></tr>
<tr><td>公开日</td><td>{pub_date}</td></tr>
<tr><td>国家/地区</td><td>{country}</td></tr>
<tr><td>IPC 分类</td><td>{ipc}</td></tr>
<tr><td>法律状态</td><td>{legal}</td></tr>
</table>

<h2>摘要</h2>
<p>{abstract_text}</p>

<h2>权利要求书</h2>
{claims_html}

<h2>说明书</h2>
{desc_html}

{images_html}

<div class="no-print top-actions" style="margin-top:40px;">
    <button class="btn-print" onclick="window.print()">🖨️ 打印 / 保存为 PDF</button>
</div>
</body>
</html>"#,
        title = esc(&patent.title),
        number = esc(&patent.patent_number),
        applicant = esc(&patent.applicant),
        inventor = esc(&patent.inventor),
        filing_date = esc(&patent.filing_date),
        pub_date = esc(&patent.publication_date),
        country = esc(&patent.country),
        ipc = esc(&patent.ipc_codes),
        legal = esc(&patent.legal_status),
        abstract_text = esc(&patent.abstract_text),
        claims_html = claims_html,
        desc_html = desc_html,
        images_html = images_html,
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

/// Proxy for patent images (bypass GFW blocking of patentimages.storage.googleapis.com)
pub async fn api_patent_image_proxy(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(u) => u.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "text/plain")],
                vec![],
            );
        }
    };

    // SSRF 防护：仅允许已知专利图片域名 / SSRF protection: allowlist of patent image domains
    const ALLOWED_DOMAINS: &[&str] = &[
        "patentimages.storage.googleapis.com",
        "worldwide.espacenet.com",
        "image.patent.k.sogou.com",
    ];
    let allowed = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .map(|host| ALLOWED_DOMAINS.contains(&host))
        .unwrap_or(false);
    if !allowed {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain")],
            b"Only patent image URLs from allowed domains".to_vec(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                return (
                    StatusCode::BAD_GATEWAY,
                    [(header::CONTENT_TYPE, "text/plain")],
                    format!("Upstream: {}", resp.status()).into_bytes(),
                );
            }
            let content_type = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/png")
                .to_string();
            match resp.bytes().await {
                Ok(bytes) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, content_type.leak() as &'static str)],
                    bytes.to_vec(),
                ),
                Err(_) => (
                    StatusCode::BAD_GATEWAY,
                    [(header::CONTENT_TYPE, "text/plain")],
                    b"Failed to read image".to_vec(),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            [(header::CONTENT_TYPE, "text/plain")],
            format!("Proxy error: {}", e).into_bytes(),
        ),
    }
}

// ── 法律状态查询（三级降级链）/ Legal Status Query (3-level degradation chain) ──

/// GET /api/patent/:id/legal-status — 查询专利法律状态
pub async fn api_patent_legal_status(
    State(s): State<AppState>,
    Path(patent_number): Path<String>,
) -> Json<serde_json::Value> {
    let config = s.config.clone();
    match fetch_legal_status(&patent_number, &config).await {
        Ok(result) => {
            // 更新本地 Patent 记录的 legal_status 字段
            if !result.current_status.is_empty() {
                let status_text = result
                    .events
                    .iter()
                    .take(5)
                    .map(|e| format!("{} ({})", e.title, e.date))
                    .collect::<Vec<_>>()
                    .join("; ");
                let _ = s.db.update_patent_legal_status(&patent_number, &status_text);
            }
            Json(json!({"status": "ok", "result": result}))
        }
        Err(e) => Json(json!({"status": "error", "message": e.to_string()})),
    }
}

/// 三级法律状态查询链 / 3-level legal status query chain
async fn fetch_legal_status(
    patent_number: &str,
    config: &Arc<RwLock<super::AppConfig>>,
) -> anyhow::Result<LegalStatusResult> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // 第 1 级：Google Patents（已有 SerpAPI）
    if let Ok(result) = fetch_legal_from_google_patents(patent_number, config).await {
        if !result.events.is_empty() {
            return Ok(result);
        }
    }

    // 第 2 级：Lens.org
    if let Ok(result) = fetch_legal_from_lens(patent_number, config).await {
        if !result.events.is_empty() {
            return Ok(result);
        }
    }

    // 第 3 级：搜狗搜索国知局公告
    if let Ok(result) = fetch_legal_from_sogou(patent_number).await {
        if !result.events.is_empty() {
            return Ok(result);
        }
    }

    // 全部失败，返回未知状态
    Ok(LegalStatusResult {
        patent_number: patent_number.to_string(),
        current_status: "未知".to_string(),
        events: vec![],
        source: "none".to_string(),
        updated_at: now,
    })
}

/// 第 1 级：Google Patents Details（通过 SerpAPI）
async fn fetch_legal_from_google_patents(
    patent_number: &str,
    config: &Arc<RwLock<super::AppConfig>>,
) -> anyhow::Result<LegalStatusResult> {
    let serpapi_key = config.read().unwrap().serpapi_key.clone();
    if serpapi_key.is_empty() {
        return Err(anyhow::anyhow!("SerpAPI key not configured"));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let url = format!(
        "https://serpapi.com/search.json?engine=google_patents_details&patent_id={}&api_key={}",
        urlencoding::encode(patent_number),
        serpapi_key
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("SerpAPI returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut events = Vec::new();

    if let Some(legal_events) = json["legal_events"].as_array() {
        for e in legal_events {
            let title = e["title"].as_str().unwrap_or("").to_string();
            let date = e["date"].as_str().unwrap_or("").to_string();
            let desc = e["description"].as_str().unwrap_or("").to_string();
            if !title.is_empty() {
                events.push(LegalEvent {
                    date,
                    title,
                    description: desc,
                });
            }
        }
    }

    let current_status = infer_current_status(&events);

    Ok(LegalStatusResult {
        patent_number: patent_number.to_string(),
        current_status,
        events,
        source: "google_patents".to_string(),
        updated_at: now,
    })
}

/// 第 2 级：Lens.org API
async fn fetch_legal_from_lens(
    patent_number: &str,
    config: &Arc<RwLock<super::AppConfig>>,
) -> anyhow::Result<LegalStatusResult> {
    let lens_key = config.read().unwrap().lens_api_key.clone();
    if lens_key.is_empty() {
        return Err(anyhow::anyhow!("Lens.org key not configured"));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let body = json!({
        "query": { "terms": { "lens_id": [patent_number] } },
        "include": ["legal_status", "publication_type", "biblio"],
        "size": 1
    });

    let resp = client
        .post("https://api.lens.org/patent/search")
        .header("Authorization", format!("Bearer {}", lens_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Lens.org returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut events = Vec::new();

    if let Some(data) = json["data"].as_array().and_then(|a| a.first()) {
        if let Some(status) = data["legal_status"]["patent_status"].as_str() {
            events.push(LegalEvent {
                date: now.clone(),
                title: status.to_string(),
                description: String::new(),
            });
        }
    }

    let current_status = if events.is_empty() {
        "未知".to_string()
    } else {
        events[0].title.clone()
    };

    Ok(LegalStatusResult {
        patent_number: patent_number.to_string(),
        current_status,
        events,
        source: "lens".to_string(),
        updated_at: now,
    })
}

/// 第 3 级：搜狗搜索国知局公告页面
async fn fetch_legal_from_sogou(patent_number: &str) -> anyhow::Result<LegalStatusResult> {
    // 搜狗必须直连（不走代理），否则触发反爬虫
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()?;

    let query = format!("{} 法律状态 专利", patent_number);
    let url = format!(
        "https://www.sogou.com/web?query={}&num=10",
        urlencoding::encode(&query)
    );

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await?;

    let html = resp.text().await?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut events = Vec::new();

    // 从搜索结果摘要中提取法律状态关键词
    let status_keywords = [
        "授权", "有效", "无效", "驳回", "撤回", "实审", "公开",
        "审查中", "失效", "届满", "缴费", "转让", "许可",
        "Grant", "Active", "Expired", "Rejected", "Withdrawn",
    ];

    // 从整个页面中提取法律状态关键词（搜索词已限定专利号，结果都是相关的）
    // 先移除 script/style 标签及其内容，避免 JS/CSS 噪声
    let no_script = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>")
        .unwrap()
        .replace_all(&html, " ");
    let no_style = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>")
        .unwrap()
        .replace_all(&no_script, " ");
    // Strip HTML tags for clean text matching
    let clean_html = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&no_style, " ")
        .to_string();

    for kw in &status_keywords {
        if clean_html.contains(kw) {
            if let Some(pos) = clean_html.find(kw) {
                // 用 char_indices 安全提取上下文，避免 UTF-8 边界 panic
                let chars: Vec<(usize, char)> = clean_html.char_indices().collect();
                let char_pos = chars.iter().position(|(i, _)| *i == pos).unwrap_or(0);
                let ctx_start_idx = char_pos.saturating_sub(30);
                let ctx_end_idx = (char_pos + 30).min(chars.len() - 1);
                let byte_start = chars[ctx_start_idx].0;
                let byte_end = if ctx_end_idx + 1 < chars.len() {
                    chars[ctx_end_idx + 1].0
                } else {
                    clean_html.len()
                };
                let context = clean_html[byte_start..byte_end].trim().to_string();

                // 过滤非有效中文描述：非中文字符占比 > 70% 则跳过
                let chinese_count = context.chars().filter(|c| *c >= '\u{4e00}' && *c <= '\u{9fff}').count();
                let total_non_space = context.chars().filter(|c| !c.is_whitespace()).count();
                if total_non_space > 0 && (chinese_count as f64 / total_non_space as f64) < 0.3 {
                    continue;
                }

                events.push(LegalEvent {
                    date: now.clone(),
                    title: kw.to_string(),
                    description: context,
                });
            }
        }
    }

    // 去重
    let mut seen = std::collections::HashSet::new();
    events.retain(|e| seen.insert(e.title.clone()));

    let current_status = infer_current_status(&events);

    Ok(LegalStatusResult {
        patent_number: patent_number.to_string(),
        current_status,
        events,
        source: "sogou".to_string(),
        updated_at: now,
    })
}

/// 从法律事件推断当前状态 / Infer current status from legal events
fn infer_current_status(events: &[LegalEvent]) -> String {
    for e in events {
        let t = e.title.as_str();
        if t.contains("无效") || t.contains("Expired") || t.contains("失效") {
            return "无效".to_string();
        }
        if t.contains("驳回") || t.contains("Rejected") {
            return "驳回".to_string();
        }
        if t.contains("撤回") || t.contains("Withdrawn") {
            return "撤回".to_string();
        }
        if t.contains("授权") || t.contains("Grant") || t.contains("Active") {
            return "有效".to_string();
        }
        if t.contains("实审") || t.contains("审查") {
            return "审查中".to_string();
        }
        if t.contains("公开") {
            return "公开".to_string();
        }
    }
    "未知".to_string()
}
