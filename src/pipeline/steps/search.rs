//! Step 3-4: SearchWeb + SearchPatents
//!
//! 类型：CODE（HTTP 请求，不调用 LLM）
//!
//! 搜索优先级：
//!   网页搜索：SerpAPI → Bing Web Search API（国内可用）
//!   专利搜索：本地 DB → SerpAPI Google Patents → Lens.org（国内可用）

use crate::db::Database;
use crate::pipeline::context::{PipelineContext, SearchResult};
use anyhow::Result;
use reqwest::Client;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// 计算查询哈希（用于缓存键）/ Compute query hash for cache key
fn query_hash(query: &str, source: &str) -> String {
    let mut h = DefaultHasher::new();
    query.hash(&mut h);
    source.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// 尝试从缓存加载搜索结果 / Try loading search results from cache
fn try_cache(db: &Database, queries: &[String], source: &str) -> Option<Vec<SearchResult>> {
    let combined = queries.join("|");
    let hash = query_hash(&combined, source);
    if let Ok(Some(json)) = db.get_search_cache(&hash) {
        if let Ok(results) = serde_json::from_str::<Vec<SearchResult>>(&json) {
            tracing::info!("搜索缓存命中: {} ({} 条结果)", source, results.len());
            return Some(results);
        }
    }
    None
}

/// 写入搜索缓存 / Save search results to cache
fn save_cache(db: &Database, queries: &[String], source: &str, results: &[SearchResult]) {
    let combined = queries.join("|");
    let hash = query_hash(&combined, source);
    if let Ok(json) = serde_json::to_string(results) {
        let _ = db.set_search_cache(&hash, &combined, &json, source);
    }
}
use std::sync::Arc;

/// 执行 Step 3: 网络搜索
/// 优先使用 SerpAPI，无 SerpAPI 时降级到 Bing Web Search API（国内可用）
pub async fn search_web(
    ctx: &mut PipelineContext,
    serpapi_key: &str,
    bing_api_key: &str,
    db: &Database,
) -> Result<()> {
    let has_serp = !serpapi_key.is_empty() && serpapi_key != "your-serpapi-key-here";
    let has_bing = !bing_api_key.is_empty();

    if !has_serp && !has_bing {
        // 两者均未配置，跳过网络搜索
        return Ok(());
    }

    // 缓存命中则直接返回 / Return cached results if available
    let queries: Vec<String> = ctx.expanded_queries.iter().take(3).cloned().collect();
    if let Some(cached) = try_cache(db, &queries, "web") {
        ctx.web_results = cached;
        return Ok(());
    }

    let client = Client::new();
    let mut all_results = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    if has_serp {
        // SerpAPI 路径（原有逻辑）
        for query in ctx.expanded_queries.iter().take(3) {
            let resp = client
                .get("https://serpapi.com/search.json")
                .query(&[
                    (
                        "q",
                        format!("{} site:patents.google.com OR technology OR patent", query),
                    ),
                    ("api_key", serpapi_key.to_string()),
                    ("num", "10".to_string()),
                ])
                .send()
                .await;

            if let Ok(resp) = resp {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(results) = json["organic_results"].as_array() {
                        for r in results {
                            let link = r["link"].as_str().unwrap_or("").to_string();
                            if link.is_empty() || seen_urls.contains(&link) {
                                continue;
                            }
                            seen_urls.insert(link.clone());
                            all_results.push(SearchResult {
                                id: format!("web_{}", all_results.len()),
                                title: r["title"].as_str().unwrap_or("").to_string(),
                                snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                                link,
                                source: "serpapi".into(),
                            });
                        }
                    }
                }
            }
        }
    } else {
        // Bing Web Search API 路径（国内可用，替代 SerpAPI）
        for query in ctx.expanded_queries.iter().take(3) {
            let q = format!("{} patent technology", query);
            let resp = client
                .get("https://api.bing.microsoft.com/v7.0/search")
                .header("Ocp-Apim-Subscription-Key", bing_api_key)
                .query(&[("q", q.as_str()), ("mkt", "zh-CN"), ("count", "10")])
                .send()
                .await;

            if let Ok(resp) = resp {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(results) = json["webPages"]["value"].as_array() {
                        for r in results {
                            let link = r["url"].as_str().unwrap_or("").to_string();
                            if link.is_empty() || seen_urls.contains(&link) {
                                continue;
                            }
                            seen_urls.insert(link.clone());
                            all_results.push(SearchResult {
                                id: format!("web_{}", all_results.len()),
                                title: r["name"].as_str().unwrap_or("").to_string(),
                                snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                                link,
                                source: "bing".into(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 搜狗免费兜底（无需任何 Key，开箱即用，国内可用）
    if all_results.is_empty() {
        for query in ctx.expanded_queries.iter().take(2) {
            if let Ok(results) = search_sogou_pipeline(&format!("{} 专利", query)).await {
                for r in results {
                    let link = r.url.clone();
                    if link.is_empty() || seen_urls.contains(&link) {
                        continue;
                    }
                    seen_urls.insert(link.clone());
                    all_results.push(SearchResult {
                        id: format!("web_{}", all_results.len()),
                        title: r.title,
                        snippet: r.snippet,
                        link,
                        source: "sogou_free".into(),
                    });
                }
            }
        }
    }

    // 写入缓存 / Persist to cache
    if !all_results.is_empty() {
        save_cache(db, &queries, "web", &all_results);
    }

    ctx.web_results = all_results;
    Ok(())
}

/// 执行 Step 4: 专利搜索
/// 优先本地 DB，其次 SerpAPI Google Patents，最后 Lens.org（国内可用）
pub async fn search_patents(
    ctx: &mut PipelineContext,
    serpapi_key: &str,
    lens_api_key: &str,
    db: &Arc<crate::db::Database>,
) -> Result<()> {
    // 缓存命中则直接返回 / Return cached results if available
    let queries: Vec<String> = ctx.expanded_queries.iter().take(3).cloned().collect();
    if let Some(cached) = try_cache(db, &queries, "patent") {
        ctx.patent_results = cached;
        return Ok(());
    }

    let mut all_results = Vec::new();
    let mut seen_titles: HashSet<String> = HashSet::new();

    // 本地数据库搜索
    for query in ctx.expanded_queries.iter().take(3) {
        if let Ok((local_results, _total)) = db.search_fts(query, 1, 20) {
            for p in local_results {
                let title_key = p.title.chars().take(20).collect::<String>().to_lowercase();
                if seen_titles.contains(&title_key) {
                    continue;
                }
                seen_titles.insert(title_key);
                all_results.push(SearchResult {
                    id: format!("patent_local_{}", p.patent_number),
                    title: p.title,
                    snippet: p.abstract_text,
                    link: format!("https://patents.google.com/patent/{}", p.patent_number),
                    source: "local_db".into(),
                });
            }
        }
    }

    // SerpAPI Google Patents 搜索
    if !serpapi_key.is_empty() && serpapi_key != "your-serpapi-key-here" {
        let client = Client::new();
        for query in ctx.expanded_queries.iter().take(2) {
            let resp = client
                .get("https://serpapi.com/search.json")
                .query(&[
                    ("engine", "google_patents".to_string()),
                    ("q", query.clone()),
                    ("api_key", serpapi_key.to_string()),
                ])
                .send()
                .await;

            if let Ok(resp) = resp {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(results) = json["organic_results"].as_array() {
                        for r in results {
                            let title = r["title"].as_str().unwrap_or("").to_string();
                            let title_key =
                                title.chars().take(20).collect::<String>().to_lowercase();
                            if seen_titles.contains(&title_key) {
                                continue;
                            }
                            seen_titles.insert(title_key);
                            all_results.push(SearchResult {
                                id: format!("patent_online_{}", all_results.len()),
                                title,
                                snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                                link: r["patent_id"]
                                    .as_str()
                                    .map(|id| format!("https://patents.google.com/patent/{}", id))
                                    .unwrap_or_default(),
                                source: "google_patents".into(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Lens.org 专利搜索（国内可用，SerpAPI 无结果时补充）
    if !lens_api_key.is_empty() && all_results.len() < 5 {
        for query in ctx.expanded_queries.iter().take(2) {
            if let Ok(json) = search_lens_raw(query, lens_api_key).await {
                if let Some(data) = json["data"].as_array() {
                    for item in data {
                        let pub_ref = &item["biblio"]["publication_reference"];
                        let jurisdiction = pub_ref["jurisdiction"]
                            .as_str()
                            .unwrap_or("")
                            .to_uppercase();
                        let doc_number = pub_ref["doc_number"].as_str().unwrap_or("");
                        let kind = pub_ref["kind"].as_str().unwrap_or("");
                        let _patent_number = format!("{}{}{}", jurisdiction, doc_number, kind);

                        let title = item["title"]
                            .as_array()
                            .and_then(|arr| {
                                arr.iter()
                                    .find(|t| t["lang"].as_str() == Some("zh"))
                                    .or_else(|| arr.first())
                            })
                            .and_then(|t| t["text"].as_str())
                            .unwrap_or("")
                            .to_string();

                        let snippet = item["abstract"]
                            .as_array()
                            .and_then(|arr| {
                                arr.iter()
                                    .find(|t| t["lang"].as_str() == Some("zh"))
                                    .or_else(|| arr.first())
                            })
                            .and_then(|t| t["text"].as_str())
                            .unwrap_or("")
                            .chars()
                            .take(300)
                            .collect::<String>();

                        let title_key = title.chars().take(20).collect::<String>().to_lowercase();
                        if seen_titles.contains(&title_key) || title.is_empty() {
                            continue;
                        }
                        seen_titles.insert(title_key);

                        all_results.push(SearchResult {
                            id: format!("patent_lens_{}", all_results.len()),
                            title,
                            snippet,
                            link: format!(
                                "https://www.lens.org/lens/patent/{}",
                                item["lens_id"].as_str().unwrap_or("")
                            ),
                            source: "lens_org".into(),
                        });
                    }
                }
            }
        }
    }

    // 写入缓存 / Persist to cache
    if !all_results.is_empty() {
        save_cache(db, &queries, "patent", &all_results);
    }

    ctx.patent_results = all_results;
    Ok(())
}

/// 内部辅助：向 Lens.org 发起原始 JSON 请求
async fn search_lens_raw(query: &str, api_key: &str) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "query": {
            "query_string": {
                "query": query,
                "fields": ["title", "abstract"]
            }
        },
        "size": 10,
        "from": 0,
        "include": [
            "lens_id", "title", "abstract", "date_published",
            "biblio.publication_reference",
            "biblio.parties.applicants"
        ]
    });
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post("https://api.lens.org/patent/search")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Lens HTTP {}", resp.status()));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}

/// 搜狗搜索结果（pipeline 用）
struct SogouResultItem {
    title: String,
    snippet: String,
    url: String,
}

/// 搜狗搜索（pipeline 用，零配置，国内可用）
async fn search_sogou_pipeline(query: &str) -> Result<Vec<SogouResultItem>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://www.sogou.com/web?query={}&num=10",
        urlencoding::encode(query)
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("搜狗 HTTP {}", resp.status()));
    }

    let html = resp.text().await.map_err(|e| e.to_string())?;
    if html.contains("安全验证") || html.len() < 5000 {
        return Err("搜狗触发验证".to_string());
    }

    let title_re = regex::Regex::new(r#"<h3[^>]*>.*?<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>.*?</h3>"#)
        .map_err(|e| e.to_string())?;
    let snippet_re = regex::Regex::new(r#"<p[^>]*>(.*?)</p>"#).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for cap in title_re.captures_iter(&html) {
        if results.len() >= 10 {
            break;
        }
        let raw_url = cap[1].to_string();
        let title = strip_html(&cap[2]).trim().to_string();
        if title.is_empty() {
            continue;
        }

        let full_url = if raw_url.starts_with("/link?") {
            format!("https://www.sogou.com{}", raw_url)
        } else {
            raw_url
        };

        let match_end = cap.get(0).unwrap().end();
        let rest = &html[match_end..std::cmp::min(match_end + 2000, html.len())];
        let snippet = if let Some(snip_cap) = snippet_re.captures(rest) {
            strip_html(&snip_cap[1])
                .trim()
                .chars()
                .take(200)
                .collect::<String>()
        } else {
            String::new()
        };

        results.push(SogouResultItem {
            title,
            snippet,
            url: full_url,
        });
    }

    Ok(results)
}

/// 简单 HTML 标签去除
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}
