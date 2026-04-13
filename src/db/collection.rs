use crate::patent::PatentSummary;
use anyhow::Result;
use rusqlite::params;

impl super::Database {
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
        c.execute(
            "DELETE FROM patent_collections WHERE collection_id = ?1",
            params![id],
        )?;
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
        let mut stmt =
            c.prepare("SELECT collection_id FROM patent_collections WHERE patent_id = ?1")?;
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
        let mut stmt =
            c.prepare("SELECT tag FROM patent_tags WHERE patent_id = ?1 ORDER BY tag")?;
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
}
