//! 证据链 CRUD / Evidence Chain CRUD

use crate::pipeline::context::Evidence;
use anyhow::Result;
use rusqlite::params;

impl super::Database {
    /// 批量插入证据 / Batch insert evidence entries
    pub fn insert_evidence_batch(&self, evidences: &[Evidence]) -> Result<()> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "INSERT INTO evidence_chain (id, idea_id, claim, source_type, source_id, \
             source_title, source_url, claim_number, excerpt, relation, confidence, \
             produced_by, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )?;
        for ev in evidences {
            stmt.execute(params![
                ev.id,
                ev.idea_id,
                ev.claim,
                ev.source_type,
                ev.source_id,
                ev.source_title,
                ev.source_url,
                ev.claim_number,
                ev.excerpt,
                ev.relation,
                ev.confidence,
                ev.produced_by,
                ev.created_at,
            ])?;
        }
        Ok(())
    }

    /// 查询创意的全部证据（按置信度降序）/ Get evidence by idea, ordered by confidence desc
    pub fn get_evidence_by_idea(&self, idea_id: &str) -> Result<Vec<Evidence>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, claim, source_type, source_id, source_title, \
             source_url, claim_number, excerpt, relation, confidence, produced_by, created_at \
             FROM evidence_chain WHERE idea_id = ?1 ORDER BY confidence DESC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok(Evidence {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    claim: r.get(2)?,
                    source_type: r.get(3)?,
                    source_id: r.get(4)?,
                    source_title: r.get(5)?,
                    source_url: r.get(6)?,
                    claim_number: r.get(7)?,
                    excerpt: r.get(8)?,
                    relation: r.get(9)?,
                    confidence: r.get(10)?,
                    produced_by: r.get(11)?,
                    created_at: r.get(12)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 删除创意关联的所有证据 / Delete all evidence for an idea
    pub fn delete_evidence_by_idea(&self, idea_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM evidence_chain WHERE idea_id = ?1",
            params![idea_id],
        )?;
        Ok(())
    }
}
