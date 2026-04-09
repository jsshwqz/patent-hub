use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use crate::patent::{Patent, PatentSummary, SearchType};
use super::relevance::{calculate_field_relevance, calculate_mixed_relevance, is_likely_name};

impl super::Database {
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

        // Patent number format (e.g., CN1234567A, US10000000B2, ZL202310123456.7)
        if q.len() >= 6 && q.len() <= 25 {
            let upper = q.to_uppercase();
            let country_codes = ["CN", "US", "EP", "JP", "KR", "TW", "HK", "WO", "PCT", "ZL"];
            for code in country_codes {
                if upper.starts_with(code) {
                    return SearchType::PatentNumber;
                }
            }
            // Pure digits (7+) or digit.digit application number format (e.g. 202310123456.7)
            let digits_only: String = q.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits_only.len() >= 7
                && q.chars().all(|c| c.is_ascii_digit() || c == '.')
            {
                return SearchType::PatentNumber;
            }
        }

        // Company keywords (check BEFORE name detection to avoid misclassifying company names)
        let company_keywords = [
            "公司",
            "集团",
            "股份",
            "有限",
            "责任",
            "corporation",
            "corp",
            "inc",
            "ltd",
            "gmbh",
            "co.",
            "co,",
            "company",
            "tech",
            "technologies",
            "systems",
            "global",
            "group",
            "energy",
            "electronics",
            "motors",
            "pharma",
            "lab",
            "labs",
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
                .search_by_field(
                    query,
                    "applicant",
                    country,
                    date_from,
                    date_to,
                    page,
                    page_size,
                )
                .map(|(p, t)| (p, t, SearchType::Applicant)),
            SearchType::Inventor => self
                .search_by_field(
                    query, "inventor", country, date_from, date_to, page, page_size,
                )
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
        // Strip spaces and dots for flexible matching (e.g. 202310123456.7 → 2023101234567)
        let q_clean = query.replace(' ', "").replace('.', "");
        // Also strip CN/ZL prefix to match against bare numbers in patent_number field
        let q_digits: String = q_clean.chars().filter(|c| c.is_ascii_digit()).collect();
        let q_with_prefix = format!("%{}%", query.replace(' ', ""));
        let q_digits_like = format!("%{}%", q_digits);
        let date_from = date_from.unwrap_or("");
        let date_to = date_to.unwrap_or("");

        // Match either the original query or just the digit portion
        let total: usize = c
            .prepare(
                "SELECT COUNT(*) FROM patents
             WHERE (REPLACE(REPLACE(patent_number, ' ', ''), '.', '') LIKE ?1
                    OR REPLACE(REPLACE(patent_number, ' ', ''), '.', '') LIKE ?4)
             AND (?2 = '' OR filing_date >= ?2)
             AND (?3 = '' OR filing_date <= ?3)",
            )?
            .query_row(params![q_with_prefix, date_from, date_to, q_digits_like], |r| r.get(0))?;

        let mut stmt = c.prepare(
            "SELECT id,patent_number,title,abstract_text,applicant,inventor,filing_date,country
             FROM patents
             WHERE (REPLACE(REPLACE(patent_number, ' ', ''), '.', '') LIKE ?1
                    OR REPLACE(REPLACE(patent_number, ' ', ''), '.', '') LIKE ?4)
             AND (?2 = '' OR filing_date >= ?2)
             AND (?3 = '' OR filing_date <= ?3)
             ORDER BY filing_date DESC LIMIT ?5 OFFSET ?6",
        )?;
        let rows = stmt
            .query_map(
                params![q_with_prefix, date_from, date_to, q_digits_like, page_size as i64, offset as i64],
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
            .query_map(params![safe_query, page_size as i64, offset as i64], |r| {
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
            })?
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
                .query_row(params![q, country.unwrap(), date_from, date_to], |r| {
                    r.get(0)
                })?
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
                        let score = calculate_mixed_relevance(query, &applicant, &inventor, &title);
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
                        let score = calculate_mixed_relevance(query, &applicant, &inventor, &title);
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

    // ── IPC Classification ────────────────────────────────────────────────────

    pub fn get_all_ipc_codes(&self) -> Result<Vec<String>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT ipc_codes FROM patents WHERE ipc_codes != '' AND ipc_codes IS NOT NULL",
        )?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn search_by_ipc(&self, code: &str) -> Result<Vec<serde_json::Value>> {
        let c = self.conn();
        let pattern = format!("%{}%", code);
        let mut stmt = c.prepare(
            "SELECT id, patent_number, title, abstract_text, applicant, filing_date, country, ipc_codes
             FROM patents WHERE ipc_codes LIKE ?1 ORDER BY filing_date DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "patent_number": row.get::<_, String>(1)?,
                    "title": row.get::<_, String>(2)?,
                    "abstract_text": row.get::<_, String>(3).unwrap_or_default(),
                    "applicant": row.get::<_, String>(4).unwrap_or_default(),
                    "filing_date": row.get::<_, String>(5).unwrap_or_default(),
                    "country": row.get::<_, String>(6).unwrap_or_default(),
                    "ipc_codes": row.get::<_, String>(7).unwrap_or_default(),
                }))
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

    /// 更新专利的法律状态 / Update patent legal status
    pub fn update_patent_legal_status(&self, patent_number: &str, legal_status: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "UPDATE patents SET legal_status = ?1 WHERE patent_number = ?2",
            params![legal_status, patent_number],
        )?;
        Ok(())
    }
}
