use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;
use crate::patent::{Patent, PatentSummary};

pub struct Database { conn: Mutex<Connection> }

impl Database {
    pub fn init(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("
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
            CREATE VIRTUAL TABLE IF NOT EXISTS patents_fts USING fts5(
                patent_number, title, abstract_text, claims, applicant, ipc_codes,
                content='patents', content_rowid='rowid'
            );
        ")?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn insert_patent(&self, p: &Patent) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("INSERT OR REPLACE INTO patents (id,patent_number,title,abstract_text,description,claims,applicant,inventor,filing_date,publication_date,grant_date,ipc_codes,cpc_codes,priority_date,country,kind_code,family_id,legal_status,citations,cited_by,source,raw_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
            params![p.id,p.patent_number,p.title,p.abstract_text,p.description,p.claims,p.applicant,p.inventor,p.filing_date,p.publication_date,p.grant_date,p.ipc_codes,p.cpc_codes,p.priority_date,p.country,p.kind_code,p.family_id,p.legal_status,p.citations,p.cited_by,p.source,p.raw_json])?;
        let _ = c.execute("INSERT INTO patents_fts(patents_fts) VALUES('rebuild')", []);
        Ok(())
    }

    pub fn get_patent(&self, id: &str) -> Result<Option<Patent>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT id,patent_number,title,abstract_text,description,claims,applicant,inventor,filing_date,publication_date,grant_date,ipc_codes,cpc_codes,priority_date,country,kind_code,family_id,legal_status,citations,cited_by,source,raw_json,created_at FROM patents WHERE id=?1 OR patent_number=?1")?;
        let result = stmt.query_row(params![id], |r| Ok(Self::row_to_patent(r))).optional()?;
        Ok(result)
    }

    pub fn search_fts(&self, query: &str, page: usize, page_size: usize) -> Result<(Vec<PatentSummary>, usize)> {
        let c = self.conn.lock().unwrap();
        let offset = page.saturating_sub(1) * page_size;
        let total: usize = c.prepare("SELECT COUNT(*) FROM patents_fts WHERE patents_fts MATCH ?1")?
            .query_row(params![query], |r| r.get(0)).unwrap_or(0);
        let mut stmt = c.prepare("SELECT p.id,p.patent_number,p.title,p.abstract_text,p.applicant,p.filing_date,p.country FROM patents p INNER JOIN patents_fts f ON p.rowid=f.rowid WHERE patents_fts MATCH ?1 ORDER BY rank LIMIT ?2 OFFSET ?3")?;
        let rows = stmt.query_map(params![query, page_size as i64, offset as i64], |r| {
            Ok(PatentSummary {
                id: r.get(0)?, patent_number: r.get(1)?, title: r.get(2)?,
                abstract_text: r.get::<_,String>(3).unwrap_or_default(),
                applicant: r.get::<_,String>(4).unwrap_or_default(),
                filing_date: r.get::<_,String>(5).unwrap_or_default(),
                country: r.get::<_,String>(6).unwrap_or_default(),
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok((rows, total))
    }

    pub fn search_like(&self, query: &str, country: Option<&str>, page: usize, page_size: usize) -> Result<(Vec<PatentSummary>, usize)> {
        let c = self.conn.lock().unwrap();
        let offset = page.saturating_sub(1) * page_size;
        let q = format!("%{query}%");
        let has_country = country.is_some();
        let where_clause = if has_country {
            "WHERE (title LIKE ?1 OR abstract_text LIKE ?1 OR applicant LIKE ?1 OR patent_number LIKE ?1) AND country=?2"
        } else {
            "WHERE title LIKE ?1 OR abstract_text LIKE ?1 OR applicant LIKE ?1 OR patent_number LIKE ?1"
        };
        let total: usize = if has_country {
            c.prepare(&format!("SELECT COUNT(*) FROM patents {where_clause}"))?.query_row(params![q, country.unwrap()], |r| r.get(0))?
        } else {
            c.prepare(&format!("SELECT COUNT(*) FROM patents {where_clause}"))?.query_row(params![q], |r| r.get(0))?
        };
        let rows: Vec<PatentSummary> = if has_country {
            let sql = format!("SELECT id,patent_number,title,abstract_text,applicant,filing_date,country FROM patents {where_clause} ORDER BY filing_date DESC LIMIT ?3 OFFSET ?4");
            let mut stmt = c.prepare(&sql)?;
            let r = stmt.query_map(params![q, country.unwrap(), page_size as i64, offset as i64], Self::row_to_summary)?.filter_map(|r| r.ok()).collect();
            r
        } else {
            let sql = format!("SELECT id,patent_number,title,abstract_text,applicant,filing_date,country FROM patents {where_clause} ORDER BY filing_date DESC LIMIT ?2 OFFSET ?3");
            let mut stmt = c.prepare(&sql)?;
            let r = stmt.query_map(params![q, page_size as i64, offset as i64], Self::row_to_summary)?.filter_map(|r| r.ok()).collect();
            r
        };
        Ok((rows, total))
    }

    fn row_to_summary(r: &rusqlite::Row) -> rusqlite::Result<PatentSummary> {
        Ok(PatentSummary {
            id: r.get(0)?, patent_number: r.get(1)?, title: r.get(2)?,
            abstract_text: r.get::<_,String>(3).unwrap_or_default(),
            applicant: r.get::<_,String>(4).unwrap_or_default(),
            filing_date: r.get::<_,String>(5).unwrap_or_default(),
            country: r.get::<_,String>(6).unwrap_or_default(),
        })
    }

    fn row_to_patent(r: &rusqlite::Row) -> Patent {
        Patent {
            id: r.get(0).unwrap_or_default(), patent_number: r.get(1).unwrap_or_default(),
            title: r.get(2).unwrap_or_default(), abstract_text: r.get(3).unwrap_or_default(),
            description: r.get(4).unwrap_or_default(), claims: r.get(5).unwrap_or_default(),
            applicant: r.get(6).unwrap_or_default(), inventor: r.get(7).unwrap_or_default(),
            filing_date: r.get(8).unwrap_or_default(), publication_date: r.get(9).unwrap_or_default(),
            grant_date: r.get(10).ok(), ipc_codes: r.get(11).unwrap_or_default(),
            cpc_codes: r.get(12).unwrap_or_default(), priority_date: r.get(13).unwrap_or_default(),
            country: r.get(14).unwrap_or_default(), kind_code: r.get(15).unwrap_or_default(),
            family_id: r.get(16).ok(), legal_status: r.get(17).unwrap_or_default(),
            citations: r.get(18).unwrap_or_default(), cited_by: r.get(19).unwrap_or_default(),
            source: r.get(20).unwrap_or_default(), raw_json: r.get(21).unwrap_or_default(),
            created_at: r.get(22).unwrap_or_default(),
        }
    }
}
