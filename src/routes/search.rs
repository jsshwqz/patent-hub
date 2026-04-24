use super::{build_online_query, escape_csv, parse_search_type, AppState};
use crate::patent::*;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::time::Instant;

const ONLINE_UPSTREAM_TIMEOUT_SECS: u64 = 8;
const ONLINE_TOTAL_BUDGET_SECS: u64 = 22;

pub async fn api_search(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<SearchResult> {
    // 空 query 校验
    if req.query.trim().is_empty() {
        return Json(SearchResult {
            patents: vec![],
            total: 0,
            page: req.page,
            page_size: req.page_size,
            search_type: Some("mixed".into()),
            dedup_removed: 0,
            categories: None,
        });
    }

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
        let base = crate::patent::canonical_patent_key(&p.patent_number);
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
        let mut by_applicant: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut by_country: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for p in &patents {
            let app = if p.applicant.is_empty() {
                "未知".to_string()
            } else {
                // Normalize applicant name (take first 20 chars to group variants)
                p.applicant.chars().take(20).collect()
            };
            *by_applicant.entry(app).or_insert(0) += 1;
            let country = if p.country.is_empty() {
                "未知".to_string()
            } else {
                p.country.clone()
            };
            *by_country.entry(country).or_insert(0) += 1;
        }
        let mut groups: Vec<CategoryGroup> = Vec::new();
        // Top applicants
        let mut app_list: Vec<_> = by_applicant.into_iter().collect();
        app_list.sort_by_key(|item| std::cmp::Reverse(item.1));
        for (name, count) in app_list.iter().take(5) {
            if *count >= 2 {
                groups.push(CategoryGroup {
                    label: format!("申请人: {}", name),
                    count: *count,
                });
            }
        }
        // Countries
        let mut country_list: Vec<_> = by_country.into_iter().collect();
        country_list.sort_by_key(|item| std::cmp::Reverse(item.1));
        for (name, count) in country_list.iter().take(5) {
            groups.push(CategoryGroup {
                label: format!("国家: {}", name),
                count: *count,
            });
        }
        if groups.is_empty() {
            None
        } else {
            Some(groups)
        }
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

fn dedup_patent_summaries(items: Vec<PatentSummary>) -> Vec<PatentSummary> {
    let mut best_by_key: std::collections::HashMap<String, PatentSummary> =
        std::collections::HashMap::new();
    for item in items {
        let key = crate::patent::canonical_patent_key(&item.patent_number);
        let dedup_key = if key.is_empty() {
            format!("TITLE::{}", item.title.trim().to_uppercase())
        } else {
            key
        };
        match best_by_key.get_mut(&dedup_key) {
            None => {
                best_by_key.insert(dedup_key, item);
            }
            Some(existing) => {
                let old_score = existing.relevance_score.unwrap_or(0.0);
                let new_score = item.relevance_score.unwrap_or(0.0);
                let old_info = existing.title.len()
                    + existing.abstract_text.len()
                    + existing.applicant.len()
                    + existing.inventor.len();
                let new_info = item.title.len()
                    + item.abstract_text.len()
                    + item.applicant.len()
                    + item.inventor.len();
                if new_score > old_score || (new_score == old_score && new_info > old_info) {
                    *existing = item;
                }
            }
        }
    }
    let mut out: Vec<PatentSummary> = best_by_key.into_values().collect();
    sort_by_relevance(&mut out);
    out
}

pub async fn api_search_online(
    State(s): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Json<serde_json::Value> {
    println!(
        "[ONLINE] query='{}' page={} country={:?} region={:?}",
        req.query, req.page, req.country, req.region
    );
    let online_search_type = parse_search_type(req.search_type.as_deref())
        .or_else(|| Some(s.db.detect_search_type(&req.query)));

    // 搜索区域判定：用户明确选择 > 自动检测
    let query_trimmed = req.query.trim();
    let looks_like_cn_patent_number = {
        let digits_only: String = query_trimmed
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        digits_only.len() >= 10
            && digits_only.len() <= 15
            && query_trimmed
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.')
    };
    let auto_cn = matches!(req.country.as_deref(), Some("CN"))
        || query_trimmed.starts_with("CN")
        || query_trimmed.starts_with("ZL")
        || looks_like_cn_patent_number
        || query_trimmed
            .chars()
            .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));

    let is_cn_query = match req.region.as_deref() {
        Some("cn") => true,    // 用户明确选国内
        Some("intl") => false, // 用户明确选国外
        _ => auto_cn,          // 自动检测
    };
    let is_intl_query = match req.region.as_deref() {
        Some("intl") => true,
        Some("cn") => false,
        _ => !auto_cn,
    };
    println!(
        "[ONLINE] region resolve: is_cn={} is_intl={}",
        is_cn_query, is_intl_query
    );
    let mut upstream_hint: Option<String> = None;
    let online_start = Instant::now();
    let mut remote_budget_exhausted = false;

    // 当前策略：在线检索仅走国外数据源链路（SerpAPI / Google Patents）
    // 不再进入 CNIPR / 百度 / 搜狗 分支。

    // ── 精确专利号查询：当检测为 PatentNumber 时，先尝试 SerpAPI Details 精确抓取 ──
    let api_key = s
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .serpapi_key
        .clone();
    if matches!(online_search_type.as_ref(), Some(SearchType::PatentNumber))
        && !api_key.is_empty()
        && api_key != "your-serpapi-key-here"
    {
        if let Some(result) = try_exact_patent_lookup(&req.query, &api_key, &s).await {
            return Json(result);
        }
    }

    if !api_key.is_empty() && api_key != "your-serpapi-key-here" {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(ONLINE_UPSTREAM_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
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
        // 中文查询时请求中文结果（并尽量约束语种）
        let lang_param = if is_cn_query {
            "&hl=zh-cn&lr=lang_zh-CN"
        } else {
            ""
        };
        let url = format!(
            "https://serpapi.com/search.json?engine=google_patents&q={}&page={}{}{}{}&api_key={}",
            urlencoding::encode(&search_query),
            serp_page,
            country_param,
            sort_param,
            lang_param,
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
                    if !status.is_success() {
                        if status.as_u16() == 429 {
                            upstream_hint = Some(
                                "SerpAPI 触发限流/额度限制（429），已自动尝试下游回退。"
                                    .to_string(),
                            );
                        } else {
                            upstream_hint = Some(format!(
                                "SerpAPI 请求失败（HTTP {}），已自动尝试下游回退。",
                                status
                            ));
                        }
                    } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
                            println!("[ONLINE] SerpAPI error: {}", err);
                            let err_l = err.to_lowercase();
                            if err_l.contains("too many requests")
                                || err_l.contains("rate limit")
                                || err_l.contains("quota")
                            {
                                upstream_hint = Some(
                                    "SerpAPI 配额或频率受限，已自动尝试下游回退。".to_string(),
                                );
                            } else {
                                let short = err.chars().take(120).collect::<String>();
                                upstream_hint = Some(format!(
                                    "SerpAPI 返回错误：{}，已自动尝试下游回退。",
                                    short
                                ));
                            }
                        } else {
                            let total = json["search_information"]["total_results"]
                                .as_u64()
                                .unwrap_or(0) as usize;
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
                                            tracing::warn!(
                                                "Failed to cache online patent {}: {}",
                                                p.patent_number,
                                                e
                                            );
                                        }
                                        // Hybrid relevance: position + content matching
                                        let position_score = (98.0 - idx as f64 * 3.0).max(30.0);
                                        let content_score = calculate_online_relevance(
                                            &req.query,
                                            &p.title,
                                            &p.abstract_text,
                                            &p.applicant,
                                        );
                                        if !is_online_result_relevant(
                                            &req.query,
                                            &p.title,
                                            &p.abstract_text,
                                            content_score,
                                            is_cn_query,
                                        ) {
                                            continue;
                                        }
                                        let score =
                                            (position_score * 0.4 + content_score * 0.6).min(100.0);
                                        let source = format!(
                                            "hybrid(pos:{:.0}+content:{:.0})",
                                            position_score, content_score
                                        );
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
                            if is_cn_query {
                                let zh_patents: Vec<PatentSummary> = patents
                                    .iter()
                                    .filter(|p| {
                                        contains_cjk(&p.title) || contains_cjk(&p.abstract_text)
                                    })
                                    .cloned()
                                    .collect();
                                if !zh_patents.is_empty() {
                                    patents = zh_patents;
                                }
                            }
                            patents = dedup_patent_summaries(patents);
                            if !patents.is_empty() {
                                let mut out = json!({
                                    "patents": patents,
                                    "total": total,
                                    "page": req.page,
                                    "page_size": 10,
                                    "source": "serpapi"
                                });
                                if let Some(h) = upstream_hint.take() {
                                    out["hint"] = json!(h);
                                }
                                return Json(out);
                            }
                            println!("[ONLINE] SerpAPI returned empty; fallback to local DB");
                            if upstream_hint.is_none() {
                                upstream_hint =
                                    Some("SerpAPI 无结果，已自动尝试下游回退。".to_string());
                            }
                        }
                    } else if upstream_hint.is_none() {
                        upstream_hint =
                            Some("SerpAPI 返回内容无法解析，已自动尝试下游回退。".to_string());
                    }
                }
            }
            Err(e) => {
                println!("[ONLINE] SerpAPI request error: {}", e);
                upstream_hint = Some(format!("SerpAPI 请求异常：{}，已自动尝试下游回退。", e));
            }
        }
    } else {
        println!("[ONLINE] No SERPAPI_KEY configured");
        upstream_hint = Some("未配置有效 SerpAPI Key，已自动尝试下游回退。".to_string());
    }

    if online_start.elapsed().as_secs() >= ONLINE_TOTAL_BUDGET_SECS {
        remote_budget_exhausted = true;
        let msg = format!(
            "在线检索超时预算已用尽（{}s），已跳过后续远端回退并改走本地兜底。",
            ONLINE_TOTAL_BUDGET_SECS
        );
        if upstream_hint.is_none() {
            upstream_hint = Some(msg);
        }
    }

    // Fallback 2: Bing Web Search API（国内可用，优先抓专利站点）
    let bing_api_key = s
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .bing_api_key
        .clone();
    if !remote_budget_exhausted && !bing_api_key.is_empty() {
        println!("[ONLINE] Trying Bing patent fallback...");
        match search_patents_via_bing(
            &req,
            online_search_type.as_ref(),
            is_cn_query,
            &bing_api_key,
        )
        .await
        {
            Ok((patents, total)) if !patents.is_empty() => {
                let patents = dedup_patent_summaries(patents);
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
                        source: "bing_web".into(),
                        raw_json: String::new(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        images: "[]".into(),
                        pdf_url: String::new(),
                    };
                    let _ = s.db.insert_patent(&full);
                }

                let dedup_total = total.min(patents.len());
                let mut out = json!({
                    "patents": patents,
                    "total": dedup_total,
                    "page": req.page,
                    "page_size": 10,
                    "source": "bing_patents"
                });
                if let Some(h) = upstream_hint {
                    out["hint"] = json!(h);
                }
                return Json(out);
            }
            Ok(_) => println!("[ONLINE] Bing patent fallback returned empty"),
            Err(e) => {
                println!("[ONLINE] Bing patent fallback error: {}", e);
                if upstream_hint.is_none() {
                    upstream_hint = Some(format!("Bing 兜底异常：{}。", e));
                }
            }
        }
    } else {
        println!("[ONLINE] No BING_API_KEY configured");
    }

    // Fallback 3: Google Patents direct (free)
    if !remote_budget_exhausted {
        println!("[ONLINE] Trying Google Patents direct (free)...");
        match search_google_patents_direct(&req, online_search_type.as_ref(), is_cn_query).await {
            Ok((patents, total)) if !patents.is_empty() => {
                let patents = dedup_patent_summaries(patents);
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
                let dedup_total = total.min(patents.len());
                return Json(json!({
                    "patents": patents,
                    "total": dedup_total,
                    "page": req.page,
                    "page_size": 10,
                    "source": "google_patents_free"
                }));
            }
            Ok(_) => println!("[ONLINE] Google Patents direct returned empty"),
            Err(e) => println!("[ONLINE] Google Patents direct error: {}", e),
        }
    } else {
        println!("[ONLINE] Skip Google Patents direct due to exhausted online budget");
    }

    // Fallback 4: local DB search
    println!("[ONLINE] Falling back to local DB");
    let local =
        s.db.search_smart(
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
        let patents = dedup_patent_summaries(patents);
        if total > 0 {
            let hint_text = upstream_hint.unwrap_or_else(|| {
                "国外在线源暂时未返回结果，已回退本地缓存。建议配置 SerpAPI 提升命中率。"
                    .to_string()
            });
            let dedup_total = total.min(patents.len());
            return Json(json!({
                "patents": patents,
                "total": dedup_total,
                "page": req.page,
                "page_size": req.page_size,
                "source": "local",
                "hint": hint_text
            }));
        }
    }
    let enc = urlencoding::encode(&req.query);
    let mut out = json!({
        "patents": [], "total": 0, "page": 1, "page_size": 20,
        "google_url": format!("https://patents.google.com/?q={enc}&oq={enc}"),
        "message": "未找到结果，可尝试在 Google Patents 上搜索"
    });
    if let Some(h) = upstream_hint {
        out["hint"] = json!(h);
    }
    Json(out)
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
    applicants.sort_by_key(|item| std::cmp::Reverse(item.1));
    let top_applicants: Vec<_> = applicants.into_iter().take(10).collect();

    let mut countries: Vec<_> = country_counts.into_iter().collect();
    countries.sort_by_key(|item| std::cmp::Reverse(item.1));

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

    let mut csv_data = String::from("专利号,标题,申请人,发明人,申请日,公开日,国家/地区,摘要\n");
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

    let filename = format!("patents_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

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

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();

    // Header style
    let header_format = rust_xlsxwriter::Format::new().set_bold();

    let headers = [
        "Patent No.",
        "Title",
        "Applicant",
        "Inventor",
        "Filing Date",
        "Country",
        "Abstract",
    ];
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
                    (
                        header::CONTENT_TYPE,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                            .to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{}\"", filename),
                    ),
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
    let pub_num = r["publication_number"].as_str().unwrap_or("").to_string();
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

// ── Bing Patent Search Fallback (API key required) ─────────────────────────

async fn search_patents_via_bing(
    req: &SearchRequest,
    search_type: Option<&SearchType>,
    is_cn_query: bool,
    bing_api_key: &str,
) -> Result<(Vec<PatentSummary>, usize), String> {
    let query = build_online_query(
        &req.query,
        search_type,
        req.date_from.as_deref(),
        req.date_to.as_deref(),
    );
    let patent_suffix = if is_cn_query {
        "(site:patents.google.com OR site:worldwide.espacenet.com OR site:patentscope.wipo.int) 专利"
    } else {
        "(site:patents.google.com OR site:worldwide.espacenet.com OR site:patentscope.wipo.int) patent"
    };
    let full_query = format!("{} {}", query, patent_suffix);

    let page = if req.page < 1 { 1 } else { req.page };
    let offset = (page - 1) * 10;
    let mkt = if is_cn_query { "zh-CN" } else { "en-US" };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(ONLINE_UPSTREAM_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get("https://api.bing.microsoft.com/v7.0/search")
        .header("Ocp-Apim-Subscription-Key", bing_api_key)
        .query(&[
            ("q", full_query.as_str()),
            ("count", "10"),
            ("offset", &offset.to_string()),
            ("mkt", mkt),
            ("textDecorations", "false"),
            ("textFormat", "Raw"),
        ])
        .send()
        .await
        .map_err(|e| format!("请求 Bing 失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let brief = body.chars().take(120).collect::<String>();
        return Err(format!("Bing HTTP {}: {}", status, brief));
    }

    let json = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Bing 返回解析失败: {}", e))?;

    let total = json["webPages"]["totalEstimatedMatches"]
        .as_u64()
        .unwrap_or(0) as usize;

    let mut patents = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(items) = json["webPages"]["value"].as_array() {
        for (idx, item) in items.iter().enumerate() {
            let title = item["name"].as_str().unwrap_or("").trim().to_string();
            let snippet = item["snippet"].as_str().unwrap_or("").trim().to_string();
            let url = item["url"].as_str().unwrap_or("").trim().to_string();
            if title.is_empty() || url.is_empty() {
                continue;
            }

            let patent_number = extract_patent_number_from_bing(&title, &snippet, &url);
            let dedup_key = if patent_number.is_empty() {
                url.clone()
            } else {
                patent_number.clone()
            };
            if !seen.insert(dedup_key) {
                continue;
            }

            let country = if patent_number.len() >= 2
                && patent_number
                    .chars()
                    .take(2)
                    .all(|c| c.is_ascii_uppercase())
            {
                patent_number.chars().take(2).collect::<String>()
            } else if is_cn_query {
                "CN".to_string()
            } else {
                String::new()
            };

            let filing_date = item["dateLastCrawled"]
                .as_str()
                .map(|s| s.chars().take(10).collect::<String>())
                .unwrap_or_default();

            let content_score = calculate_online_relevance(&query, &title, &snippet, "");
            if !is_online_result_relevant(&req.query, &title, &snippet, content_score, is_cn_query)
            {
                continue;
            }

            let position_score = (98.0 - idx as f64 * 4.0).max(25.0);
            let score = (position_score * 0.35 + content_score * 0.65).min(100.0);

            patents.push(PatentSummary {
                id: uuid::Uuid::new_v4().to_string(),
                patent_number,
                title,
                abstract_text: snippet,
                applicant: String::new(),
                inventor: String::new(),
                filing_date,
                country,
                relevance_score: Some(score),
                score_source: Some("bing-web".to_string()),
            });
        }
    }

    if is_cn_query {
        let zh_patents: Vec<PatentSummary> = patents
            .iter()
            .filter(|p| contains_cjk(&p.title) || contains_cjk(&p.abstract_text))
            .cloned()
            .collect();
        if !zh_patents.is_empty() {
            patents = zh_patents;
        }
    }

    let final_total = if total > 0 { total } else { patents.len() };
    Ok((patents, final_total))
}

fn extract_patent_number_from_bing(title: &str, snippet: &str, link: &str) -> String {
    if let Some(cap) = regex::Regex::new(r"patents\.google\.com/patent/([A-Z]{2}[A-Z0-9]+)")
        .ok()
        .and_then(|re| re.captures(link))
    {
        return cap
            .get(1)
            .map(|m| m.as_str().to_uppercase())
            .unwrap_or_default();
    }

    let combined = format!("{} {} {}", title, snippet, link).to_uppercase();
    if let Some(m) = regex::Regex::new(r"\b(?:US|CN|EP|WO|JP|KR|DE|FR|GB)\d{6,}[A-Z]?\d?\b")
        .ok()
        .and_then(|re| re.find(&combined))
    {
        return m.as_str().to_string();
    }

    let cn = extract_cn_patent_number(title, snippet, link);
    if !cn.is_empty() {
        return cn.to_uppercase().replace('.', "");
    }

    String::new()
}

// ── Google Patents Direct Search (free, no API key; 注意：国内需要 VPN) ────────

async fn search_google_patents_direct(
    req: &SearchRequest,
    search_type: Option<&SearchType>,
    is_cn_query: bool,
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

    // 中文查询时加 hl=zh 让 Google Patents 返回中文标题/摘要
    let lang_param = if is_cn_query { "&hl=zh" } else { "" };
    let url = format!(
        "https://patents.google.com/xhr/query?url=q%3D{}&exp=&num={}&page={}{}",
        urlencoding::encode(&full_query),
        num,
        page,
        lang_param
    );
    println!("[FREE] Google Patents direct URL: {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(ONLINE_UPSTREAM_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("JSON parse: {}", e))?;

    let total = json["results"]["total_num_results"].as_u64().unwrap_or(0) as usize;

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
                    // Prefer localized (Chinese) title over default English
                    let title = {
                        let localized = pat["title_localized"]
                            .as_array()
                            .and_then(|arr| {
                                // Prefer zh, then first available
                                arr.iter()
                                    .find(|t| {
                                        t["lang"]
                                            .as_str()
                                            .map(|l| l.starts_with("zh"))
                                            .unwrap_or(false)
                                    })
                                    .or_else(|| arr.first())
                            })
                            .and_then(|v| v["value"].as_str())
                            .unwrap_or("");
                        if localized.is_empty() {
                            strip_html_tags(pat["title"].as_str().unwrap_or(""))
                        } else {
                            strip_html_tags(localized)
                        }
                    };
                    let snippet = strip_html_tags(pat["snippet"].as_str().unwrap_or(""));

                    let filing_date = pat["filing_date"].as_str().unwrap_or("").to_string();
                    // Prefer localized (Chinese) assignee/inventor
                    let applicant = pat["assignee_localized"]
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .find(|a| {
                                    a["lang"]
                                        .as_str()
                                        .map(|l| l.starts_with("zh"))
                                        .unwrap_or(false)
                                })
                                .or_else(|| arr.first())
                        })
                        .and_then(|v| v["value"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let inventor = pat["inventor_localized"]
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .find(|a| {
                                    a["lang"]
                                        .as_str()
                                        .map(|l| l.starts_with("zh"))
                                        .unwrap_or(false)
                                })
                                .or_else(|| arr.first())
                        })
                        .and_then(|v| v["value"].as_str())
                        .unwrap_or("")
                        .to_string();

                    if !title.is_empty() {
                        let content_score =
                            calculate_online_relevance(&query, &title, &snippet, &applicant);
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

    println!(
        "[FREE] Google Patents direct: {} results, total {}",
        patents.len(),
        total
    );
    Ok((patents, total))
}

/// Calculate content-based relevance for online search results.
fn calculate_online_relevance(
    query: &str,
    title: &str,
    abstract_text: &str,
    applicant: &str,
) -> f64 {
    let q = query.trim().to_lowercase();
    let t = title.trim().to_lowercase();
    let a = abstract_text.trim().to_lowercase();
    let app = applicant.trim().to_lowercase();

    let mut score = 30.0;

    // Title matching (most important, max +50)
    if t == q {
        score += 50.0;
    } else if t.contains(&q) {
        score += 35.0;
    } else {
        // Word-level matching in title
        let q_words: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
        if !q_words.is_empty() {
            let matches = q_words.iter().filter(|w| t.contains(*w)).count();
            score += (matches as f64 / q_words.len() as f64) * 30.0;
        }
        // Chinese bigram matching (more meaningful than single-char matching)
        let q_chars: Vec<char> = q
            .chars()
            .filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c))
            .collect();
        if q_chars.len() >= 2 {
            // Build bigrams for better semantic matching
            let q_bigrams: Vec<String> = q_chars.windows(2).map(|w| w.iter().collect()).collect();
            let t_chars: Vec<char> = t
                .chars()
                .filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c))
                .collect();
            let t_bigrams: Vec<String> = if t_chars.len() >= 2 {
                t_chars.windows(2).map(|w| w.iter().collect()).collect()
            } else {
                vec![]
            };
            if !q_bigrams.is_empty() && !t_bigrams.is_empty() {
                let matches = q_bigrams.iter().filter(|bg| t_bigrams.contains(bg)).count();
                score += (matches as f64 / q_bigrams.len() as f64) * 25.0;
            }
        } else if !q_chars.is_empty() {
            // Fallback to single-char matching for short queries
            let matches = q_chars.iter().filter(|c| t.contains(**c)).count();
            score += (matches as f64 / q_chars.len() as f64) * 20.0;
        }
    }

    // Abstract matching (secondary, max +15)
    if a.contains(&q) {
        score += 15.0;
    } else {
        let q_words: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
        if !q_words.is_empty() {
            let matches = q_words.iter().filter(|w| a.contains(*w)).count();
            score += (matches as f64 / q_words.len() as f64) * 10.0;
        }
        // Chinese bigram matching in abstract too
        let q_chars: Vec<char> = q
            .chars()
            .filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c))
            .collect();
        if q_chars.len() >= 2 {
            let q_bigrams: Vec<String> = q_chars.windows(2).map(|w| w.iter().collect()).collect();
            let a_chars: Vec<char> = a
                .chars()
                .filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c))
                .collect();
            let a_bigrams: Vec<String> = if a_chars.len() >= 2 {
                a_chars.windows(2).map(|w| w.iter().collect()).collect()
            } else {
                vec![]
            };
            if !q_bigrams.is_empty() && !a_bigrams.is_empty() {
                let matches = q_bigrams.iter().filter(|bg| a_bigrams.contains(bg)).count();
                score += (matches as f64 / q_bigrams.len() as f64) * 8.0;
            }
        }
    }

    // Applicant matching (bonus, max +5)
    if app.contains(&q) {
        score += 5.0;
    }

    score.min(100.0)
}

fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

fn is_online_result_relevant(
    query: &str,
    title: &str,
    abstract_text: &str,
    content_score: f64,
    is_cn_query: bool,
) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    let t = title.to_lowercase();
    let a = abstract_text.to_lowercase();
    let ql = q.to_lowercase();

    // 直接匹配优先保留
    if t.contains(&ql) || a.contains(&ql) {
        return true;
    }

    // 中文查询：门槛更高，避免无关英文噪声
    if is_cn_query {
        if contains_cjk(title) || contains_cjk(abstract_text) {
            return content_score >= 45.0;
        }
        return content_score >= 62.0;
    }

    // 英文/国际查询：保持一定召回
    content_score >= 40.0
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

// ── 搜狗搜索（国内可用，无需 Key，内置免费方案） ──────────────────────────

/// 搜狗搜索结果
#[allow(dead_code)] // url 保留用于后续结果详情链接
struct FreeSearchResult {
    title: String,
    snippet: String,
    url: String,
    score: Option<f64>,
}

/// 通过搜狗搜索实现免费网页搜索（国内可用，无需 API Key）
/// 搜狗对自动化请求相对宽松，适合作为内置免费搜索方案
#[allow(dead_code)] // 兼容保留：当前在线链路已切到国外数据源优先
async fn search_sogou_free(query: &str) -> Result<(Vec<FreeSearchResult>, usize), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(ONLINE_UPSTREAM_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://www.sogou.com/web?query={}&num=10",
        urlencoding::encode(query)
    );
    println!("[SOGOU] Searching: {}", url);

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| format!("搜狗请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("搜狗 HTTP {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // 检查是否触发验证码
    if html.contains("安全验证") || html.contains("captcha") || html.len() < 5000 {
        return Err("搜狗触发了安全验证，请稍后重试".to_string());
    }

    // 解析搜狗搜索结果 HTML
    // 结构：<h3><a href="URL">TITLE</a></h3> 后面跟 <p>SNIPPET</p>
    let mut results = Vec::new();
    let title_re = regex::Regex::new(r#"<h3[^>]*>.*?<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>.*?</h3>"#)
        .map_err(|e| e.to_string())?;
    let snippet_re = regex::Regex::new(r#"<p[^>]*>(.*?)</p>"#).map_err(|e| e.to_string())?;

    // 按 <h3> 标签分块匹配
    for cap in title_re.captures_iter(&html) {
        if results.len() >= 10 {
            break;
        }
        let raw_url = cap[1].to_string();
        let title = strip_html_tags(&cap[2]).trim().to_string();
        if title.is_empty() {
            continue;
        }

        // 拼完整 URL
        let full_url = if raw_url.starts_with("/link?") {
            format!("https://www.sogou.com{}", raw_url)
        } else {
            raw_url
        };

        // 在匹配位置之后搜索 snippet
        let match_end = cap.get(0).unwrap().end();
        let rest = &html[match_end..std::cmp::min(match_end + 2000, html.len())];
        let snippet = if let Some(snip_cap) = snippet_re.captures(rest) {
            strip_html_tags(&snip_cap[1])
                .trim()
                .chars()
                .take(200)
                .collect::<String>()
        } else {
            String::new()
        };

        let content_score = calculate_online_relevance(query, &title, &snippet, "");
        results.push(FreeSearchResult {
            title,
            snippet,
            url: full_url,
            score: Some(content_score),
        });
    }

    let total = results.len();
    println!("[SOGOU] Parsed {} results", total);
    Ok((results, total))
}

// ── CNIPR (国知局) 专利搜索 ──────────────────────────────────────

/// CNIPR OAuth2 login to get access_token
#[allow(dead_code)] // 兼容保留：当前在线链路已切到国外数据源优先
async fn cnipr_login(
    config: &std::sync::Arc<std::sync::RwLock<super::AppConfig>>,
) -> Option<(String, String, String)> {
    let (client_id, client_secret, user, password) = {
        let c = config.read().unwrap_or_else(|e| e.into_inner());
        // Check if token is still valid (with 60s buffer)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if !c.cnipr_access_token.is_empty() && c.cnipr_token_expires > now + 60 {
            return Some((
                c.cnipr_access_token.clone(),
                c.cnipr_open_id.clone(),
                c.cnipr_client_id.clone(),
            ));
        }
        (
            c.cnipr_client_id.clone(),
            c.cnipr_client_secret.clone(),
            c.cnipr_user.clone(),
            c.cnipr_password.clone(),
        )
    };

    if client_id.is_empty() || user.is_empty() || password.is_empty() {
        return None;
    }

    println!("[CNIPR] Logging in as {}...", user);
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let resp = client
        .post("https://open.cnipr.com/oauth/json/user/login")
        .form(&[
            ("user_account", user.as_str()),
            ("user_password", password.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "password"),
            ("return_refresh_token", "1"),
        ])
        .send()
        .await
        .ok()?;

    let body = resp.text().await.ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;

    if json["status"].as_i64() != Some(0) {
        println!(
            "[CNIPR] Login failed: {}",
            json["message"].as_str().unwrap_or("unknown")
        );
        return None;
    }

    let access_token = json["access_token"].as_str()?.to_string();
    let open_id = json["open_id"].as_str()?.to_string();
    let expires_in = json["expires_in"].as_u64().unwrap_or(2592000);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Cache token in config
    {
        let mut c = config.write().unwrap_or_else(|e| e.into_inner());
        c.cnipr_access_token = access_token.clone();
        c.cnipr_open_id = open_id.clone();
        c.cnipr_token_expires = now + expires_in;
    }

    println!("[CNIPR] Login success, token expires in {}s", expires_in);
    Some((access_token, open_id, client_id))
}

/// Search CNIPR for Chinese patents
#[allow(dead_code)] // 兼容保留：当前在线链路已切到国外数据源优先
pub async fn search_cnipr(
    query: &str,
    config: &std::sync::Arc<std::sync::RwLock<super::AppConfig>>,
    page: usize,
) -> anyhow::Result<(Vec<PatentSummary>, usize)> {
    let (access_token, open_id, client_id) = cnipr_login(config)
        .await
        .ok_or_else(|| anyhow::anyhow!("CNIPR login failed"))?;

    let from = if page > 1 { (page - 1) * 10 } else { 0 };

    // Build search expression
    let query_clean = query.trim().replace('.', "");
    let is_number = query.starts_with("CN")
        || query.starts_with("ZL")
        || query_clean.chars().all(|c| c.is_ascii_digit());
    let exp = if is_number {
        // 同时搜 公开号 和 申请号（用户可能输入任一种）
        // 申请号格式：纯数字（如 2023101234567）
        // 公开号格式：CN + 数字 + 种类码（如 CN116401354A）
        let digits: String = query.chars().filter(|c| c.is_ascii_digit()).collect();
        // 去掉前缀/种类码，得到纯数字用于申请号搜索
        let app_number = digits.clone();
        // 构造公开号：保留原始输入（可能已带 CN 前缀）
        let pub_number = query.trim().to_string();
        // 还要试 CN + 纯数字（用户可能输入 ZL 号或纯数字）
        let cn_number = if !pub_number.starts_with("CN") {
            format!("CN{}", digits)
        } else {
            pub_number.clone()
        };
        // 申请号同时搜有点和无点格式（CNIPR可能用任一格式存储）
        let app_with_dot = query.trim().to_string();
        let app_no_dot = app_number.clone();
        // 只有当原始输入和纯数字不同（即原始有点）时才加两种格式
        let app_conditions = if app_with_dot != app_no_dot {
            format!("申请号='{}' OR 申请号='{}'", app_with_dot, app_no_dot)
        } else {
            format!("申请号='{}'", app_no_dot)
        };
        format!(
            "公开（公告）号='{}' OR 公开（公告）号='{}' OR {}",
            pub_number, cn_number, app_conditions
        )
    } else {
        format!("名称+摘要=({})", query)
    };

    let url = format!(
        "https://open.cnipr.com/cnipr-api/v1/api/search/sf1/{}",
        client_id
    );

    println!("[CNIPR] Search exp='{}' from={}", exp, from);

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let resp = client
        .post(&url)
        .form(&[
            ("openid", open_id.as_str()),
            ("access_token", access_token.as_str()),
            ("exp", exp.as_str()),
            ("dbs", "FMZL"),  // 发明专利
            ("dbs", "FMSQ"),  // 发明申请
            ("dbs", "SYXX"),  // 实用新型
            ("option", "1"),  // 按词检索
            ("order", "-pubDate"), // 最新优先
            ("from", &from.to_string()),
            ("size", "10"),
            ("displayCols", "名称,摘要,申请号,公开（公告）号,公开（公告）日,申请日,申请（专利权）人,发明（设计）人,主分类号,法律状态"),
        ])
        .send()
        .await?;

    let body = resp.text().await?;
    println!("[CNIPR] Response len={}", body.len());

    let json: serde_json::Value = serde_json::from_str(&body)?;

    if json["status"].as_i64() != Some(0) {
        let msg = json["message"].as_str().unwrap_or("unknown error");
        println!("[CNIPR] Search error: {}", msg);
        return Err(anyhow::anyhow!("CNIPR: {}", msg));
    }

    let total = json["total"].as_u64().unwrap_or(0) as usize;
    let mut patents = Vec::new();

    if let Some(results) = json["results"].as_array() {
        println!("[CNIPR] Got {} results, total={}", results.len(), total);
        for (idx, r) in results.iter().enumerate() {
            let title = r["title"]
                .as_str()
                .or_else(|| r["名称"].as_str())
                .unwrap_or("")
                .to_string();
            let abstract_text = r["abs"]
                .as_str()
                .or_else(|| r["摘要"].as_str())
                .unwrap_or("")
                .to_string();
            let pub_number = r["pubNumber"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .or_else(|| r["公开（公告）号"].as_str())
                .unwrap_or("")
                .to_string();
            let app_date = r["appDate"]
                .as_str()
                .or_else(|| r["申请日"].as_str())
                .unwrap_or("")
                .to_string();
            let applicant = r["applicantName"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .or_else(|| r["申请（专利权）人"].as_str())
                .unwrap_or("")
                .to_string();
            let inventor = r["inventorName"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .or_else(|| r["发明（设计）人"].as_str())
                .unwrap_or("")
                .to_string();
            let legal_status = r["lprs"]
                .as_str()
                .or_else(|| r["法律状态"].as_str())
                .unwrap_or("")
                .to_string();

            // Compute relevance score
            let position_score = (98.0 - idx as f64 * 3.0).max(30.0);
            let content_score =
                calculate_online_relevance(query, &title, &abstract_text, &applicant);
            let score = (position_score * 0.3 + content_score * 0.7).min(100.0);

            // Append legal status to abstract for display
            let display_abstract = if !legal_status.is_empty() {
                format!("[{}] {}", legal_status, abstract_text)
            } else {
                abstract_text
            };

            patents.push(PatentSummary {
                id: uuid::Uuid::new_v4().to_string(),
                patent_number: pub_number,
                title,
                abstract_text: display_abstract,
                applicant,
                inventor,
                filing_date: app_date,
                country: "CN".to_string(),
                relevance_score: Some(score),
                score_source: Some("cnipr".to_string()),
            });
        }
    }

    Ok((patents, total))
}

/// 从标题/摘要/链接中提取中国专利号（CN开头或纯数字申请号）
#[allow(dead_code)] // 兼容保留：当前在线链路已切到国外数据源优先
fn extract_cn_patent_number(title: &str, snippet: &str, link: &str) -> String {
    let all_text = format!("{} {} {}", title, snippet, link);
    // 匹配 CN + 数字 + 字母 格式（如 CN116401354A）
    if let Some(m) = regex::Regex::new(r"CN\d{6,}[A-Z]?")
        .ok()
        .and_then(|re| re.find(&all_text))
    {
        return m.as_str().to_string();
    }
    // 匹配纯数字申请号（12-13位）
    if let Some(m) = regex::Regex::new(r"\d{12,13}\.\d")
        .ok()
        .and_then(|re| re.find(&all_text))
    {
        return m.as_str().to_string();
    }
    String::new()
}

/// 清理 HTML 标签（百度搜索结果含 <em> 等标签）
#[allow(dead_code)] // 兼容保留：当前在线链路已切到国外数据源优先
fn clean_html_tags(s: &str) -> String {
    regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(s, "")
        .to_string()
}

/// 精确专利号查询：通过 SerpAPI 按专利号精确抓取
/// 对于中国申请号（如 202210835143.9），先通过关键词搜索找到公开号，再用 details API 抓取
async fn try_exact_patent_lookup(
    query: &str,
    api_key: &str,
    state: &super::AppState,
) -> Option<serde_json::Value> {
    let q = query.trim();
    let digits: String = q.chars().filter(|c| c.is_ascii_digit()).collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(ONLINE_UPSTREAM_TIMEOUT_SECS))
        .build()
        .ok()?;

    // Step 1: Determine patent_id to look up
    let patent_id: String;

    let is_bare_cn_app = digits.len() >= 10
        && digits.len() <= 15
        && q.chars().all(|c| c.is_ascii_digit() || c == '.');

    if q.starts_with("CN")
        || q.starts_with("US")
        || q.starts_with("EP")
        || q.starts_with("WO")
        || q.starts_with("JP")
        || q.starts_with("KR")
    {
        // Already has country prefix — likely a publication number, try directly
        let no_dot = q.replace('.', "");
        let lang = if q.starts_with("CN") { "zh" } else { "en" };
        patent_id = format!("patent/{}/{}", no_dot, lang);
    } else if is_bare_cn_app {
        // Bare Chinese APPLICATION number (e.g. 202210835143.9)
        // Google Patents indexes by PUBLICATION number, not application number.
        // We must first search to discover the publication number.
        let core = if digits.len() >= 13 {
            &digits[..digits.len() - 1] // strip check digit
        } else {
            &digits
        };
        println!(
            "[EXACT] Bare CN app number detected, searching for publication number via '{}'",
            core
        );

        let search_url = format!(
            "https://serpapi.com/search.json?engine=google_patents&q={}&page=1&api_key={}",
            urlencoding::encode(core),
            api_key
        );
        let resp = client.get(&search_url).send().await.ok()?;
        let body = resp.text().await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;

        // Find the first result's patent_id
        let found_id = json["organic_results"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|r| r["patent_id"].as_str())
            .map(|s| s.to_string());

        match found_id {
            Some(id) => {
                // For CN patents, use /zh to get Chinese results
                let id = if id.contains("/CN") {
                    id.replace("/en", "/zh")
                } else {
                    id
                };
                println!("[EXACT] Found publication via keyword search: {}", id);
                patent_id = id;
            }
            None => {
                println!("[EXACT] Keyword search returned no results for '{}'", core);
                return None;
            }
        }
    } else {
        // Default: use /en for non-CN patents
        patent_id = format!("patent/{}/en", q);
    }

    // Step 2: Fetch full details via google_patents_details
    let url = format!(
        "https://serpapi.com/search.json?engine=google_patents_details&patent_id={}&api_key={}",
        urlencoding::encode(&patent_id),
        api_key
    );
    println!("[EXACT] Fetching details for: {}", patent_id);

    let resp = client.get(&url).send().await.ok()?;
    let body = resp.text().await.ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;

    if json.get("error").is_some() {
        println!("[EXACT] Details API error: {}", json["error"]);
        return None;
    }

    let title = json["title"].as_str().unwrap_or("");
    if title.is_empty() {
        println!("[EXACT] Details returned empty title");
        return None;
    }

    // Extract inventors/assignees from arrays
    let inventor = json["inventors"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["name"].as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    let assignee = json["assignees"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();

    let pub_number = json["publication_number"].as_str().unwrap_or(q).to_string();
    let country = pub_number.chars().take(2).collect::<String>();
    let patent = Patent {
        id: uuid::Uuid::new_v4().to_string(),
        patent_number: pub_number.clone(),
        title: title.to_string(),
        abstract_text: json["abstract"].as_str().unwrap_or("").to_string(),
        description: json["description"].as_str().unwrap_or("").to_string(),
        claims: json["claims"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default(),
        applicant: assignee.clone(),
        inventor: inventor.clone(),
        filing_date: json["filing_date"].as_str().unwrap_or("").to_string(),
        publication_date: json["publication_date"].as_str().unwrap_or("").to_string(),
        grant_date: json["grant_date"].as_str().map(|s| s.to_string()),
        ipc_codes: String::new(),
        cpc_codes: String::new(),
        priority_date: json["priority_date"].as_str().unwrap_or("").to_string(),
        country: country.clone(),
        kind_code: String::new(),
        family_id: None,
        legal_status: String::new(),
        citations: "[]".into(),
        cited_by: "[]".into(),
        source: "serpapi_exact".into(),
        raw_json: body,
        created_at: chrono::Utc::now().to_rfc3339(),
        images: "[]".into(),
        pdf_url: json["pdf"].as_str().unwrap_or("").to_string(),
    };

    // Cache to local DB
    let _ = state.db.insert_patent(&patent);

    let summary = PatentSummary {
        id: patent.id.clone(),
        patent_number: patent.patent_number.clone(),
        title: patent.title.clone(),
        abstract_text: patent.abstract_text.clone(),
        applicant: patent.applicant.clone(),
        inventor: patent.inventor.clone(),
        filing_date: patent.filing_date.clone(),
        country,
        relevance_score: Some(100.0),
        score_source: Some("exact_lookup".to_string()),
    };

    println!(
        "[EXACT] Found patent: {} — {}",
        summary.patent_number, summary.title
    );
    Some(serde_json::json!({
        "patents": [summary],
        "total": 1,
        "page": 1,
        "page_size": 10,
        "source": "serpapi_exact"
    }))
}
