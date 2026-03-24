use crate::patent::{Idea, IdeaSummary, Patent, PatentSummary, SearchType};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Acquire database connection, recovering from mutex poison if needed.
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("Database mutex was poisoned, recovering");
            poisoned.into_inner()
        })
    }

    /// Current schema version. Increment when adding migrations.
    const SCHEMA_VERSION: i32 = 4;

    pub fn init(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Schema version tracking
        conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);")?;
        let current_version: i32 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
            .unwrap_or(0);

        // Migration 0 → 1: Initial schema
        if current_version < 1 {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS patents (
                    id TEXT PRIMARY KEY, patent_number TEXT NOT NULL, title TEXT NOT NULL,
                    abstract_text TEXT, description TEXT, claims TEXT, applicant TEXT,
                    inventor TEXT, filing_date TEXT, publication_date TEXT, grant_date TEXT,
                    ipc_codes TEXT, cpc_codes TEXT, priority_date TEXT, country TEXT,
                    kind_code TEXT, family_id TEXT, legal_status TEXT, citations TEXT,
                    cited_by TEXT, source TEXT, raw_json TEXT,
                    created_at TEXT DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_pn ON patents(patent_number);
                CREATE INDEX IF NOT EXISTS idx_applicant ON patents(applicant);
                CREATE INDEX IF NOT EXISTS idx_inventor ON patents(inventor);
                CREATE INDEX IF NOT EXISTS idx_country ON patents(country);
                CREATE INDEX IF NOT EXISTS idx_filing_date ON patents(filing_date);
                CREATE VIRTUAL TABLE IF NOT EXISTS patents_fts USING fts5(
                    patent_number, title, abstract_text, claims, applicant, inventor, ipc_codes,
                    content='patents', content_rowid='rowid'
                );

                CREATE TABLE IF NOT EXISTS ideas (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    input_type TEXT DEFAULT 'text',
                    status TEXT DEFAULT 'pending',
                    analysis TEXT DEFAULT '',
                    web_results TEXT DEFAULT '[]',
                    patent_results TEXT DEFAULT '[]',
                    novelty_score REAL,
                    created_at TEXT DEFAULT (datetime('now')),
                    updated_at TEXT DEFAULT (datetime('now'))
                );

                DELETE FROM schema_version;
                INSERT INTO schema_version (version) VALUES (1);
            ",
            )?;
            tracing::info!("Database migrated to version 1");
        }

        // Migration 1 → 2: Collections and tags
        if current_version < 2 {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS collections (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT DEFAULT '',
                    created_at TEXT DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS patent_collections (
                    patent_id TEXT NOT NULL,
                    collection_id TEXT NOT NULL,
                    added_at TEXT DEFAULT (datetime('now')),
                    PRIMARY KEY (patent_id, collection_id)
                );

                CREATE TABLE IF NOT EXISTS patent_tags (
                    patent_id TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    PRIMARY KEY (patent_id, tag)
                );

                CREATE INDEX IF NOT EXISTS idx_pc_collection ON patent_collections(collection_id);
                CREATE INDEX IF NOT EXISTS idx_pt_tag ON patent_tags(tag);

                DELETE FROM schema_version;
                INSERT INTO schema_version (version) VALUES (2);
            ",
            )?;
            tracing::info!("Database migrated to version 2");
        }

        // Migration 2 → 3: Idea multi-round chat messages
        if current_version < 3 {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS idea_messages (
                    id TEXT PRIMARY KEY,
                    idea_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at TEXT DEFAULT (datetime('now')),
                    FOREIGN KEY (idea_id) REFERENCES ideas(id)
                );

                CREATE INDEX IF NOT EXISTS idx_im_idea ON idea_messages(idea_id);

                -- Add summary field to ideas table
                ALTER TABLE ideas ADD COLUMN discussion_summary TEXT DEFAULT '';

                DELETE FROM schema_version;
                INSERT INTO schema_version (version) VALUES (3);
            ",
            )?;
            tracing::info!("Database migrated to version 3");
        }

        if current_version < 4 {
            conn.execute_batch(
                "
                ALTER TABLE patents ADD COLUMN images TEXT DEFAULT '[]';
                ALTER TABLE patents ADD COLUMN pdf_url TEXT DEFAULT '';

                DELETE FROM schema_version;
                INSERT INTO schema_version (version) VALUES (4);
            ",
            )?;
            tracing::info!("Database migrated to version 4 (patent images)");
        }

        if current_version > 0 && current_version < Self::SCHEMA_VERSION {
            tracing::info!("Database migrated from version {} to {}", current_version, Self::SCHEMA_VERSION);
        }

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Read current schema version (for testing/diagnostics).
    #[allow(dead_code)]
    pub fn query_schema_version(&self) -> Result<i32> {
        let c = self.conn();
        let v: i32 = c.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))?;
        Ok(v)
    }

    pub fn insert_patent(&self, p: &Patent) -> Result<()> {
        let c = self.conn();
        // Delete old FTS entry if replacing
        if let Err(e) = c.execute(
            "DELETE FROM patents_fts WHERE rowid = (SELECT rowid FROM patents WHERE id = ?1)",
            params![p.id],
        ) {
            tracing::warn!("FTS delete for patent {} failed: {}", p.id, e);
        }
        c.execute("INSERT OR REPLACE INTO patents (id,patent_number,title,abstract_text,description,claims,applicant,inventor,filing_date,publication_date,grant_date,ipc_codes,cpc_codes,priority_date,country,kind_code,family_id,legal_status,citations,cited_by,source,raw_json,images,pdf_url) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
            params![p.id,p.patent_number,p.title,p.abstract_text,p.description,p.claims,p.applicant,p.inventor,p.filing_date,p.publication_date,p.grant_date,p.ipc_codes,p.cpc_codes,p.priority_date,p.country,p.kind_code,p.family_id,p.legal_status,p.citations,p.cited_by,p.source,p.raw_json,p.images,p.pdf_url])?;
        // Insert single row into FTS index (incremental, not full rebuild)
        if let Err(e) = c.execute(
            "INSERT INTO patents_fts(rowid, patent_number, title, abstract_text, claims, applicant, inventor, ipc_codes) SELECT rowid, patent_number, title, abstract_text, claims, applicant, inventor, ipc_codes FROM patents WHERE id = ?1",
            params![p.id],
        ) {
            tracing::warn!("FTS insert for patent {} failed: {}", p.id, e);
        }
        Ok(())
    }

    pub fn get_patent(&self, id: &str) -> Result<Option<Patent>> {
        let c = self.conn();
        let mut stmt = c.prepare("SELECT id,patent_number,title,abstract_text,description,claims,applicant,inventor,filing_date,publication_date,grant_date,ipc_codes,cpc_codes,priority_date,country,kind_code,family_id,legal_status,citations,cited_by,source,raw_json,created_at,images,pdf_url FROM patents WHERE id=?1 OR patent_number=?1")?;
        let result = stmt
            .query_row(params![id], |r| Ok(Self::row_to_patent(r)))
            .optional()?;
        Ok(result)
    }

    /// Detect search type from query string.
    pub fn detect_search_type(&self, query: &str) -> SearchType {
        let q = query.trim();

        // Patent number format (e.g., CN1234567A, US10000000B2)
        if q.len() >= 6 && q.len() <= 20 {
            let upper = q.to_uppercase();
            let country_codes = ["CN", "US", "EP", "JP", "KR", "TW", "HK", "WO", "PCT"];
            for code in country_codes {
                if upper.starts_with(code) {
                    return SearchType::PatentNumber;
                }
            }
            if q.chars().all(|c| c.is_ascii_digit()) && q.len() >= 7 {
                return SearchType::PatentNumber;
            }
        }

        // Company keywords (check BEFORE name detection to avoid misclassifying company names)
        let company_keywords = [
            "公司", "集团", "股份", "有限", "责任",
            "corporation", "corp", "inc", "ltd", "gmbh", "co.", "co,", "company",
            "tech", "technologies", "systems", "global", "group", "energy",
            "electronics", "motors", "pharma", "lab", "labs",
        ];
        let q_lower = q.to_lowercase();
        if company_keywords.iter().any(|k| q_lower.contains(k)) {
            return SearchType::Applicant;
        }

        if is_likely_name(q) {
            return SearchType::Inventor;
        }

        SearchType::Mixed
    }

    /// Smart search: choose strategy by detected or requested search type.
    #[allow(clippy::too_many_arguments)]
    pub fn search_smart(
        &self,
        query: &str,
        search_type: Option<&SearchType>,
        country: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<PatentSummary>, usize, SearchType)> {
        let detected_type = if let Some(st) = search_type {
            st.clone()
        } else {
            self.detect_search_type(query)
        };

        match detected_type {
            SearchType::PatentNumber => self
                .search_by_patent_number(query, date_from, date_to, page, page_size)
                .map(|(p, t)| (p, t, SearchType::PatentNumber)),
            SearchType::Applicant => self
                .search_by_field(query, "applicant", country, date_from, date_to, page, page_size)
                .map(|(p, t)| (p, t, SearchType::Applicant)),
            SearchType::Inventor => self
                .search_by_field(query, "inventor", country, date_from, date_to, page, page_size)
                .map(|(p, t)| (p, t, SearchType::Inventor)),
            SearchType::Keyword => {
                let has_filters = country.filter(|s| !s.is_empty()).is_some()
                    || date_from.filter(|s| !s.is_empty()).is_some()
                    || date_to.filter(|s| !s.is_empty()).is_some();

                if has_filters {
                    self.search_like(query, country, date_from, date_to, page, page_size)
                        .map(|(p, t)| (p, t, SearchType::Keyword))
                } else {
                    self.search_fts(query, page, page_size)
                        .map(|(p, t)| (p, t, SearchType::Keyword))
                }
            }
            SearchType::Mixed => self
                .search_like(query, country, date_from, date_to, page, page_size)
                .map(|(p, t)| (p, t, SearchType::Mixed)),
        }
    }

    /// Search by patent number.
    fn search_by_patent_number(
        &self,
        query: &str,
        date_from: Option<&str>,
        date_to: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<PatentSummary>, usize)> {
        let c = self.conn();
        let offset = page.saturating_sub(1) * page_size;
        let q = format!("%{}%", query.replace(' ', ""));
        let date_from = date_from.unwrap_or("");
        let date_to = date_to.unwrap_or("");

        let total: usize = c
            .prepare(
                "SELECT COUNT(*) FROM patents
             WHERE REPLACE(patent_number, ' ', '') LIKE ?1
             AND (?2 = '' OR filing_date >= ?2)
             AND (?3 = '' OR filing_date <= ?3)",
            )?
            .query_row(params![q, date_from, date_to], |r| r.get(0))?;

        let mut stmt = c.prepare(
            "SELECT id,patent_number,title,abstract_text,applicant,inventor,filing_date,country
             FROM patents
             WHERE REPLACE(patent_number, ' ', '') LIKE ?1
             AND (?2 = '' OR filing_date >= ?2)
             AND (?3 = '' OR filing_date <= ?3)
             ORDER BY filing_date DESC LIMIT ?4 OFFSET ?5",
        )?;
        let rows = stmt
            .query_map(
                params![q, date_from, date_to, page_size as i64, offset as i64],
                |r| {
                    Ok(PatentSummary {
                        id: r.get(0)?,
                        patent_number: r.get(1)?,
                        title: r.get(2)?,
                        abstract_text: r.get::<_, String>(3).unwrap_or_default(),
                        applicant: r.get::<_, String>(4).unwrap_or_default(),
                        inventor: r.get::<_, String>(5).unwrap_or_default(),
                        filing_date: r.get::<_, String>(6).unwrap_or_default(),
                        country: r.get::<_, String>(7).unwrap_or_default(),
                        relevance_score: Some(100.0),
                        score_source: Some("patent number exact match".to_string()),
                    })
                },
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok((rows, total))
    }

    /// Generic search by a single field (applicant or inventor) with optional country filter.
    #[allow(clippy::too_many_arguments)]
    fn search_by_field(
        &self,
        query: &str,
        field: &str, // "applicant" or "inventor"
        country: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<PatentSummary>, usize)> {
        // Whitelist field names to prevent SQL injection
        let field = match field {
            "applicant" => "applicant",
            "inventor" => "inventor",
            _ => return Err(anyhow::anyhow!("invalid search field: {}", field)),
        };
        let c = self.conn();
        let offset = page.saturating_sub(1) * page_size;
        let q = format!("%{}%", query);
        let date_from = date_from.unwrap_or("");
        let date_to = date_to.unwrap_or("");

        // Build WHERE clause dynamically based on whether country filter is present
        let (count_sql, data_sql) = if let Some(country_val) = country.filter(|s| !s.is_empty()) {
            let count = format!(
                "SELECT COUNT(*) FROM patents WHERE {} LIKE ?1 AND country = ?2
                 AND (?3 = '' OR filing_date >= ?3)
                 AND (?4 = '' OR filing_date <= ?4)",
                field
            );
            let data = format!(
                "SELECT id,patent_number,title,abstract_text,applicant,inventor,filing_date,country
                 FROM patents WHERE {} LIKE ?1 AND country = ?2
                 AND (?3 = '' OR filing_date >= ?3)
                 AND (?4 = '' OR filing_date <= ?4)
                 ORDER BY filing_date DESC LIMIT ?5 OFFSET ?6",
                field
            );

            let total: usize = c
                .prepare(&count)?
                .query_row(params![q, country_val, date_from, date_to], |r| r.get(0))?;

            let mut stmt = c.prepare(&data)?;
            let rows: Vec<PatentSummary> = stmt
                .query_map(
                    params![
                        q,
                        country_val,
                        date_from,
                        date_to,
                        page_size as i64,
                        offset as i64
                    ],
                    |row| self.row_to_summary_with_relevance(row, query, field),
                )?
                .filter_map(|r| r.ok())
                .collect();

            return Ok((rows, total));
        } else {
            let count = format!(
                "SELECT COUNT(*) FROM patents WHERE {} LIKE ?1
                 AND (?2 = '' OR filing_date >= ?2)
                 AND (?3 = '' OR filing_date <= ?3)",
                field
            );
            let data = format!(
                "SELECT id,patent_number,title,abstract_text,applicant,inventor,filing_date,country
                 FROM patents WHERE {} LIKE ?1
                 AND (?2 = '' OR filing_date >= ?2)
                 AND (?3 = '' OR filing_date <= ?3)
                 ORDER BY filing_date DESC LIMIT ?4 OFFSET ?5",
                field
            );
            (count, data)
        };

        let total: usize = c
            .prepare(&count_sql)?
            .query_row(params![q, date_from, date_to], |r| r.get(0))?;

        let mut stmt = c.prepare(&data_sql)?;
        let rows: Vec<PatentSummary> = stmt
            .query_map(
                params![q, date_from, date_to, page_size as i64, offset as i64],
                |row| self.row_to_summary_with_relevance(row, query, field),
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok((rows, total))
    }

    /// Map a row to PatentSummary with relevance scoring based on field type.
    fn row_to_summary_with_relevance(
        &self,
        row: &rusqlite::Row,
        query: &str,
        field: &str,
    ) -> rusqlite::Result<PatentSummary> {
        let applicant = row.get::<_, String>(4).unwrap_or_default();
        let inventor = row.get::<_, String>(5).unwrap_or_default();

        let (score, source) = match field {
            "applicant" => calculate_field_relevance(query, &applicant, "applicant"),
            "inventor" => calculate_field_relevance(query, &inventor, "inventor"),
            _ => (50.0, "unknown field".to_string()),
        };

        Ok(PatentSummary {
            id: row.get(0)?,
            patent_number: row.get(1)?,
            title: row.get(2)?,
            abstract_text: row.get::<_, String>(3).unwrap_or_default(),
            applicant,
            inventor,
            filing_date: row.get::<_, String>(6).unwrap_or_default(),
            country: row.get::<_, String>(7).unwrap_or_default(),
            relevance_score: Some(score),
            score_source: Some(source),
        })
    }

    /// Sanitize user input for FTS5 MATCH queries.
    /// Wraps each token in double quotes to prevent FTS5 syntax injection.
    fn sanitize_fts_query(query: &str) -> String {
        query
            .split_whitespace()
            .map(|word| {
                let clean: String = word.chars().filter(|c| *c != '"').collect();
                if clean.is_empty() {
                    return String::new();
                }
                format!("\"{}\"", clean)
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn search_fts(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<PatentSummary>, usize)> {
        let c = self.conn();
        let offset = page.saturating_sub(1) * page_size;
        let safe_query = Self::sanitize_fts_query(query);
        if safe_query.is_empty() {
            return Ok((vec![], 0));
        }
        let total: usize = c
            .prepare("SELECT COUNT(*) FROM patents_fts WHERE patents_fts MATCH ?1")?
            .query_row(params![safe_query], |r| r.get(0))
            .unwrap_or(0);
        let mut stmt = c.prepare("SELECT p.id,p.patent_number,p.title,p.abstract_text,p.applicant,p.inventor,p.filing_date,p.country,f.rank FROM patents p INNER JOIN patents_fts f ON p.rowid=f.rowid WHERE patents_fts MATCH ?1 ORDER BY rank LIMIT ?2 OFFSET ?3")?;
        let rows = stmt
            .query_map(
                params![safe_query, page_size as i64, offset as i64],
                |r| {
                    let rank: f64 = r.get::<_, f64>(8).unwrap_or(0.0);
                    // FTS5 rank is negative (closer to 0 = better). Normalize to 0-100.
                    // Typical range: -20 (very relevant) to 0 (less relevant)
                    let score = ((-rank).min(20.0) / 20.0 * 70.0 + 30.0).min(100.0);
                    Ok(PatentSummary {
                        id: r.get(0)?,
                        patent_number: r.get(1)?,
                        title: r.get(2)?,
                        abstract_text: r.get::<_, String>(3).unwrap_or_default(),
                        applicant: r.get::<_, String>(4).unwrap_or_default(),
                        inventor: r.get::<_, String>(5).unwrap_or_default(),
                        filing_date: r.get::<_, String>(6).unwrap_or_default(),
                        country: r.get::<_, String>(7).unwrap_or_default(),
                        relevance_score: Some(score),
                        score_source: Some("FTS5-BM25".to_string()),
                    })
                },
            )?
            .filter_map(|r| r.ok())
            .collect();
        Ok((rows, total))
    }

    pub fn search_like(
        &self,
        query: &str,
        country: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<PatentSummary>, usize)> {
        let c = self.conn();
        let offset = page.saturating_sub(1) * page_size;
        let q = format!("%{}%", query);
        let date_from = date_from.unwrap_or("");
        let date_to = date_to.unwrap_or("");
        let has_country = country.filter(|s| !s.is_empty()).is_some();

        let where_clause = if has_country {
            "WHERE (title LIKE ?1 OR abstract_text LIKE ?1 OR applicant LIKE ?1 OR inventor LIKE ?1 OR patent_number LIKE ?1)
             AND country=?2
             AND (?3 = '' OR filing_date >= ?3)
             AND (?4 = '' OR filing_date <= ?4)"
        } else {
            "WHERE (title LIKE ?1 OR abstract_text LIKE ?1 OR applicant LIKE ?1 OR inventor LIKE ?1 OR patent_number LIKE ?1)
             AND (?2 = '' OR filing_date >= ?2)
             AND (?3 = '' OR filing_date <= ?3)"
        };

        let total: usize = if has_country {
            c.prepare(&format!("SELECT COUNT(*) FROM patents {where_clause}"))?
                .query_row(
                    params![q, country.unwrap(), date_from, date_to],
                    |r| r.get(0),
                )?
        } else {
            c.prepare(&format!("SELECT COUNT(*) FROM patents {where_clause}"))?
                .query_row(params![q, date_from, date_to], |r| r.get(0))?
        };

        let select = format!("SELECT id,patent_number,title,abstract_text,applicant,inventor,filing_date,country FROM patents {where_clause} ORDER BY filing_date DESC");

        let rows: Vec<PatentSummary> = if has_country {
            let sql = format!("{select} LIMIT ?5 OFFSET ?6");
            let mut stmt = c.prepare(&sql)?;
            let r = stmt
                .query_map(
                    params![
                        q,
                        country.unwrap(),
                        date_from,
                        date_to,
                        page_size as i64,
                        offset as i64
                    ],
                    |row| {
                        let applicant = row.get::<_, String>(4).unwrap_or_default();
                        let inventor = row.get::<_, String>(5).unwrap_or_default();
                        let title = row.get::<_, String>(2).unwrap_or_default();
                        let score =
                            calculate_mixed_relevance(query, &applicant, &inventor, &title);
                        Ok(PatentSummary {
                            id: row.get(0)?,
                            patent_number: row.get(1)?,
                            title,
                            abstract_text: row.get::<_, String>(3).unwrap_or_default(),
                            applicant,
                            inventor,
                            filing_date: row.get::<_, String>(6).unwrap_or_default(),
                            country: row.get::<_, String>(7).unwrap_or_default(),
                            relevance_score: Some(score),
                            score_source: Some("mixed search match".to_string()),
                        })
                    },
                )?
                .filter_map(|r| r.ok())
                .collect();
            r
        } else {
            let sql = format!("{select} LIMIT ?4 OFFSET ?5");
            let mut stmt = c.prepare(&sql)?;
            let r = stmt
                .query_map(
                    params![q, date_from, date_to, page_size as i64, offset as i64],
                    |row| {
                        let applicant = row.get::<_, String>(4).unwrap_or_default();
                        let inventor = row.get::<_, String>(5).unwrap_or_default();
                        let title = row.get::<_, String>(2).unwrap_or_default();
                        let score =
                            calculate_mixed_relevance(query, &applicant, &inventor, &title);
                        Ok(PatentSummary {
                            id: row.get(0)?,
                            patent_number: row.get(1)?,
                            title,
                            abstract_text: row.get::<_, String>(3).unwrap_or_default(),
                            applicant,
                            inventor,
                            filing_date: row.get::<_, String>(6).unwrap_or_default(),
                            country: row.get::<_, String>(7).unwrap_or_default(),
                            relevance_score: Some(score),
                            score_source: Some("mixed search match".to_string()),
                        })
                    },
                )?
                .filter_map(|r| r.ok())
                .collect();
            r
        };
        Ok((rows, total))
    }

    fn row_to_summary(r: &rusqlite::Row) -> rusqlite::Result<PatentSummary> {
        Ok(PatentSummary {
            id: r.get(0)?,
            patent_number: r.get(1)?,
            title: r.get(2)?,
            abstract_text: r.get::<_, String>(3).unwrap_or_default(),
            applicant: r.get::<_, String>(4).unwrap_or_default(),
            inventor: r.get::<_, String>(5).unwrap_or_default(),
            filing_date: r.get::<_, String>(6).unwrap_or_default(),
            country: r.get::<_, String>(7).unwrap_or_default(),
            relevance_score: None,
            score_source: None,
        })
    }

    // ── Idea CRUD ─────────────────────────────────────────────────────────────

    pub fn insert_idea(&self, idea: &Idea) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT OR REPLACE INTO ideas (id,title,description,input_type,status,analysis,web_results,patent_results,novelty_score,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![idea.id, idea.title, idea.description, idea.input_type, idea.status, idea.analysis, idea.web_results, idea.patent_results, idea.novelty_score, idea.created_at, idea.updated_at],
        )?;
        Ok(())
    }

    pub fn get_idea(&self, id: &str) -> Result<Option<Idea>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id,title,description,input_type,status,analysis,web_results,patent_results,novelty_score,created_at,updated_at,COALESCE(discussion_summary,'') FROM ideas WHERE id=?1",
        )?;
        let result = stmt
            .query_row(params![id], |r| {
                Ok(Idea {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    description: r.get(2)?,
                    input_type: r.get(3)?,
                    status: r.get(4)?,
                    analysis: r.get(5)?,
                    web_results: r.get(6)?,
                    patent_results: r.get(7)?,
                    novelty_score: r.get(8)?,
                    created_at: r.get(9)?,
                    updated_at: r.get(10)?,
                    discussion_summary: r.get(11)?,
                })
            })
            .optional()?;
        Ok(result)
    }

    pub fn update_idea(&self, idea: &Idea) -> Result<()> {
        let c = self.conn();
        c.execute(
            "UPDATE ideas SET title=?2,description=?3,status=?4,analysis=?5,web_results=?6,patent_results=?7,novelty_score=?8,updated_at=datetime('now') WHERE id=?1",
            params![idea.id, idea.title, idea.description, idea.status, idea.analysis, idea.web_results, idea.patent_results, idea.novelty_score],
        )?;
        Ok(())
    }

    pub fn list_ideas(&self) -> Result<Vec<IdeaSummary>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id,title,status,novelty_score,created_at FROM ideas ORDER BY created_at DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(IdeaSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    status: r.get(2)?,
                    novelty_score: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    fn row_to_patent(r: &rusqlite::Row) -> Patent {
        Patent {
            id: r.get(0).unwrap_or_default(),
            patent_number: r.get(1).unwrap_or_default(),
            title: r.get(2).unwrap_or_default(),
            abstract_text: r.get(3).unwrap_or_default(),
            description: r.get(4).unwrap_or_default(),
            claims: r.get(5).unwrap_or_default(),
            applicant: r.get(6).unwrap_or_default(),
            inventor: r.get(7).unwrap_or_default(),
            filing_date: r.get(8).unwrap_or_default(),
            publication_date: r.get(9).unwrap_or_default(),
            grant_date: r.get(10).ok(),
            ipc_codes: r.get(11).unwrap_or_default(),
            cpc_codes: r.get(12).unwrap_or_default(),
            priority_date: r.get(13).unwrap_or_default(),
            country: r.get(14).unwrap_or_default(),
            kind_code: r.get(15).unwrap_or_default(),
            family_id: r.get(16).ok(),
            legal_status: r.get(17).unwrap_or_default(),
            citations: r.get(18).unwrap_or_default(),
            cited_by: r.get(19).unwrap_or_default(),
            source: r.get(20).unwrap_or_default(),
            raw_json: r.get(21).unwrap_or_default(),
            created_at: r.get(22).unwrap_or_default(),
            images: r.get(23).unwrap_or_default(),
            pdf_url: r.get(24).unwrap_or_default(),
        }
    }

    // ── Collections CRUD ──────────────────────────────────────────────

    pub fn create_collection(&self, id: &str, name: &str, description: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO collections (id, name, description) VALUES (?1, ?2, ?3)",
            params![id, name, description],
        )?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn list_collections(&self) -> Result<Vec<(String, String, String, i64, String)>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT c.id, c.name, c.description,
                    (SELECT COUNT(*) FROM patent_collections pc WHERE pc.collection_id = c.id) as count,
                    c.created_at
             FROM collections c ORDER BY c.created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn delete_collection(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.execute("DELETE FROM patent_collections WHERE collection_id = ?1", params![id])?;
        c.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_to_collection(&self, patent_id: &str, collection_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT OR IGNORE INTO patent_collections (patent_id, collection_id) VALUES (?1, ?2)",
            params![patent_id, collection_id],
        )?;
        Ok(())
    }

    pub fn remove_from_collection(&self, patent_id: &str, collection_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM patent_collections WHERE patent_id = ?1 AND collection_id = ?2",
            params![patent_id, collection_id],
        )?;
        Ok(())
    }

    pub fn get_collection_patents(&self, collection_id: &str) -> Result<Vec<PatentSummary>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT p.id, p.patent_number, p.title, p.abstract_text, p.applicant,
                    p.inventor, p.filing_date, p.country
             FROM patents p
             INNER JOIN patent_collections pc ON p.id = pc.patent_id
             WHERE pc.collection_id = ?1
             ORDER BY pc.added_at DESC",
        )?;
        let rows = stmt
            .query_map(params![collection_id], |r| {
                Ok(PatentSummary {
                    id: r.get(0)?,
                    patent_number: r.get(1)?,
                    title: r.get(2)?,
                    abstract_text: r.get(3)?,
                    applicant: r.get(4)?,
                    inventor: r.get(5)?,
                    filing_date: r.get(6)?,
                    country: r.get(7)?,
                    relevance_score: None,
                    score_source: None,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_patent_collections(&self, patent_id: &str) -> Result<Vec<String>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT collection_id FROM patent_collections WHERE patent_id = ?1",
        )?;
        let rows = stmt
            .query_map(params![patent_id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── Tags CRUD ────────────────────────────────────────────────────

    pub fn add_tag(&self, patent_id: &str, tag: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT OR IGNORE INTO patent_tags (patent_id, tag) VALUES (?1, ?2)",
            params![patent_id, tag],
        )?;
        Ok(())
    }

    pub fn remove_tag(&self, patent_id: &str, tag: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM patent_tags WHERE patent_id = ?1 AND tag = ?2",
            params![patent_id, tag],
        )?;
        Ok(())
    }

    pub fn get_patent_tags(&self, patent_id: &str) -> Result<Vec<String>> {
        let c = self.conn();
        let mut stmt = c.prepare("SELECT tag FROM patent_tags WHERE patent_id = ?1 ORDER BY tag")?;
        let rows = stmt
            .query_map(params![patent_id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn list_all_tags(&self) -> Result<Vec<(String, i64)>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT tag, COUNT(*) as cnt FROM patent_tags GROUP BY tag ORDER BY cnt DESC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── Idea Messages CRUD ───────────────────────────────────────────

    pub fn add_idea_message(&self, id: &str, idea_id: &str, role: &str, content: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO idea_messages (id, idea_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params![id, idea_id, role, content],
        )?;
        Ok(())
    }

    pub fn get_idea_messages(&self, idea_id: &str) -> Result<Vec<(String, String, String, String)>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, role, content, created_at FROM idea_messages WHERE idea_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn update_idea_summary(&self, idea_id: &str, summary: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "UPDATE ideas SET discussion_summary = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![summary, idea_id],
        )?;
        Ok(())
    }

    pub fn get_idea_summary(&self, idea_id: &str) -> Result<String> {
        let c = self.conn();
        let summary: String = c.query_row(
            "SELECT COALESCE(discussion_summary, '') FROM ideas WHERE id = ?1",
            params![idea_id],
            |r| r.get(0),
        ).unwrap_or_default();
        Ok(summary)
    }
}

// ── Relevance scoring functions ──────────────────────────────────────────────

/// Unified relevance scoring for a single field (applicant or inventor).
fn calculate_field_relevance(query: &str, field_value: &str, field_name: &str) -> (f64, String) {
    let q = query.trim().to_lowercase();
    let f = field_value.trim().to_lowercase();

    // Exact match
    if q == f || q.replace(' ', "") == f.replace(' ', "") {
        return (100.0, format!("{field_name} exact match"));
    }

    // Prefix match (for applicant)
    if field_name == "applicant" && f.starts_with(&q) {
        return (95.0, format!("{field_name} prefix match"));
    }

    // Contains match
    if f.contains(&q) {
        return (90.0, format!("{field_name} contains match"));
    }

    // Chinese character matching (for inventor)
    if field_name == "inventor" {
        let q_chars: Vec<char> = q.chars().filter(|c| *c > '\u{7F}').collect();
        let f_chars: Vec<char> = f.chars().filter(|c| *c > '\u{7F}').collect();
        if !q_chars.is_empty() && !f_chars.is_empty() {
            // Surname match
            if q_chars.first() == f_chars.first()
                && (q_chars.len() <= 2 || f_chars.len() <= 2)
            {
                return (85.0, "surname match".to_string());
            }
            if q_chars.iter().all(|qc| f_chars.contains(qc)) {
                return (80.0, format!("{field_name} name contains"));
            }
        }
    }

    // Word-level matching
    let q_words: Vec<&str> = q
        .split(|c: char| c.is_whitespace() || c == ',' || c == '.')
        .filter(|s| !s.is_empty())
        .collect();
    let f_words: Vec<&str> = f
        .split(|c: char| c.is_whitespace() || c == ',' || c == '.')
        .filter(|s| !s.is_empty())
        .collect();

    let mut matched_words = 0;
    for qw in &q_words {
        for fw in &f_words {
            if fw.contains(qw) || qw.contains(fw) {
                matched_words += 1;
                break;
            }
        }
    }

    if !q_words.is_empty() {
        let match_ratio = matched_words as f64 / q_words.len() as f64;
        if match_ratio > 0.0 {
            let score = 50.0 + (match_ratio * 40.0);
            return (
                score,
                format!("{field_name} word match ({:.0}%)", match_ratio * 100.0),
            );
        }
    }

    (30.0, format!("{field_name} fuzzy match"))
}

/// Calculate mixed search relevance score.
fn calculate_mixed_relevance(query: &str, applicant: &str, inventor: &str, title: &str) -> f64 {
    let q = query.trim().to_lowercase();

    let (applicant_score, _) = calculate_field_relevance(query, applicant, "applicant");
    if applicant_score >= 90.0 {
        return applicant_score;
    }

    let (inventor_score, _) = calculate_field_relevance(query, inventor, "inventor");
    if inventor_score >= 90.0 {
        return inventor_score;
    }

    // Title match
    let t = title.trim().to_lowercase();
    if t == q {
        return 95.0;
    }
    if t.starts_with(&q) {
        return 85.0;
    }
    if t.contains(&q) {
        return 75.0;
    }

    applicant_score.max(inventor_score).max(40.0)
}

/// Detect if input is likely a person's name.
fn is_likely_name(query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() || q.len() < 2 || q.len() > 50 {
        return false;
    }
    is_chinese_name(q) || is_english_name(q)
}

fn is_chinese_name(query: &str) -> bool {
    let q = query.trim();
    if q.len() < 2 || q.len() > 6 {
        return false;
    }
    let chinese_chars = q.chars().filter(|c| *c > '\u{7F}').count();
    let total_chars = q.chars().count();
    total_chars > 0 && (chinese_chars as f64 / total_chars as f64) >= 0.8 && !q.contains(' ')
}

fn is_english_name(query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() < 2 || words.len() > 5 {
        return false;
    }
    let capitalized_count = words
        .iter()
        .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
        .count();
    capitalized_count >= words.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::patent::{Patent, SearchType};

    fn sample_patent(id: &str, title: &str, filing_date: &str) -> Patent {
        Patent {
            id: id.to_string(),
            patent_number: format!("CN{}A", id.to_uppercase()),
            title: title.to_string(),
            abstract_text: format!("{title} abstract"),
            description: "description".to_string(),
            claims: "claim".to_string(),
            applicant: "Acme Corp".to_string(),
            inventor: "Alice Zhang".to_string(),
            filing_date: filing_date.to_string(),
            publication_date: filing_date.to_string(),
            grant_date: None,
            ipc_codes: "G06N".to_string(),
            cpc_codes: "G06N".to_string(),
            priority_date: filing_date.to_string(),
            country: "CN".to_string(),
            kind_code: "A".to_string(),
            family_id: None,
            legal_status: "pending".to_string(),
            citations: "[]".to_string(),
            cited_by: "[]".to_string(),
            source: "test".to_string(),
            raw_json: "{}".to_string(),
            created_at: "2026-03-07T00:00:00Z".to_string(),
            images: "[]".to_string(),
            pdf_url: String::new(),
        }
    }

    #[test]
    fn keyword_search_without_filters_uses_fts_path() {
        let db = Database::init(":memory:").expect("init db");
        db.insert_patent(&sample_patent(
            "fts1",
            "Vector database patent",
            "2024-01-10",
        ))
        .expect("insert patent");

        let (rows, total, detected) = db
            .search_smart(
                "Vector",
                Some(&SearchType::Keyword),
                None,
                None,
                None,
                1,
                10,
            )
            .expect("search succeeds");

        assert_eq!(detected, SearchType::Keyword);
        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Vector database patent");
        // FTS results now have BM25-based relevance scores (30-100 range)
        assert!(rows[0].relevance_score.is_some());
        let score = rows[0].relevance_score.unwrap();
        assert!(score >= 30.0 && score <= 100.0, "FTS score {} out of range", score);
    }

    #[test]
    fn keyword_search_with_date_filter_uses_filtered_search() {
        let db = Database::init(":memory:").expect("init db");
        db.insert_patent(&sample_patent(
            "old1",
            "Vector database patent old",
            "2023-01-10",
        ))
        .expect("insert old patent");
        db.insert_patent(&sample_patent(
            "new1",
            "Vector database patent new",
            "2024-01-10",
        ))
        .expect("insert new patent");

        let (rows, total, detected) = db
            .search_smart(
                "Vector",
                Some(&SearchType::Keyword),
                None,
                Some("2024-01-01"),
                Some("2024-12-31"),
                1,
                10,
            )
            .expect("search succeeds");

        assert_eq!(detected, SearchType::Keyword);
        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Vector database patent new");
        assert!(rows[0].relevance_score.is_some());
    }
}
