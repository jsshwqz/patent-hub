use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patent {
    #[serde(default = "gen_id")]
    pub id: String,
    pub patent_number: String,
    pub title: String,
    #[serde(default)]
    pub abstract_text: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub claims: String,
    #[serde(default)]
    pub applicant: String,
    #[serde(default)]
    pub inventor: String,
    #[serde(default)]
    pub filing_date: String,
    #[serde(default)]
    pub publication_date: String,
    pub grant_date: Option<String>,
    #[serde(default)]
    pub ipc_codes: String,
    #[serde(default)]
    pub cpc_codes: String,
    #[serde(default)]
    pub priority_date: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub kind_code: String,
    pub family_id: Option<String>,
    #[serde(default)]
    pub legal_status: String,
    #[serde(default)]
    pub citations: String,
    #[serde(default)]
    pub cited_by: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub raw_json: String,
    #[serde(default = "now_str")]
    pub created_at: String,
    /// JSON array of image URLs (patent drawings)
    #[serde(default)]
    pub images: String,
    /// PDF download URL
    #[serde(default)]
    pub pdf_url: String,
}

fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SearchType {
    Applicant,    // 按申请人搜索
    Inventor,     // 按发明人搜索
    PatentNumber, // 按专利号搜索
    Keyword,      // 关键词搜索（标题/摘要）
    Mixed,        // 混合搜索
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "d1")]
    pub page: usize,
    #[serde(default = "d20")]
    pub page_size: usize,
    pub country: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_type: Option<String>, // "applicant", "inventor", "keyword", "mixed"
    pub sort_by: Option<String>,     // "relevance", "new", "old"
    #[serde(default)]
    pub ipc: Option<String>,         // IPC classification filter (prefix match)
    #[serde(default)]
    pub cpc: Option<String>,         // CPC classification filter (prefix match)
}

fn d1() -> usize {
    1
}
fn d20() -> usize {
    20
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub patents: Vec<PatentSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub search_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<CategoryGroup>>,
    #[serde(default)]
    pub dedup_removed: usize,
}

/// Group of search results by category (applicant, country, etc.)
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryGroup {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatentSummary {
    pub id: String,
    pub patent_number: String,
    pub title: String,
    pub abstract_text: String,
    pub applicant: String,
    pub inventor: String,
    pub filing_date: String,
    pub country: String,
    #[serde(default)]
    pub relevance_score: Option<f64>,
    #[serde(default)]
    pub score_source: Option<String>, // 评分来源说明
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiChatRequest {
    pub message: String,
    pub patent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchPatentRequest {
    pub patent_number: String,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportRequest {
    pub patents: Vec<Patent>,
}

// ── Idea / Innovation validation ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idea {
    pub id: String,
    pub title: String,
    pub description: String,
    pub input_type: String,
    pub status: String,
    pub analysis: String,
    pub web_results: String,
    pub patent_results: String,
    pub novelty_score: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub discussion_summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeaSubmitRequest {
    pub title: String,
    pub description: String,
    #[serde(default = "default_text")]
    pub input_type: String,
}

fn default_text() -> String {
    "text".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeaSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub novelty_score: Option<f64>,
    pub created_at: String,
}
