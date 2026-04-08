//! 版本管理 + 发现记忆 CRUD / Version management + Findings memory

use crate::db::Database;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaVersion {
    pub id: String,
    pub idea_id: String,
    pub version_number: i32,
    pub context_json: String,
    pub current_step: String,
    pub branch_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaBranch {
    pub id: String,
    pub idea_id: String,
    pub name: String,
    pub parent_branch_id: String,
    pub parent_version_id: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub idea_id: String,
    /// "discovery" | "dead_end" | "insight" | "experiment_result"
    pub finding_type: String,
    pub title: String,
    pub content: String,
    pub source_step: String,
    pub branch_id: String,
    pub created_at: String,
}

impl Database {
    // ── Idea Versions ────────────────────────────────────────────

    pub fn insert_idea_version(&self, v: &IdeaVersion) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO idea_versions (id, idea_id, version_number, context_json, current_step, branch_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![v.id, v.idea_id, v.version_number, v.context_json, v.current_step, v.branch_id, v.created_at],
        )?;
        Ok(())
    }

    pub fn get_idea_versions(&self, idea_id: &str) -> Result<Vec<IdeaVersion>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, version_number, context_json, current_step, branch_id, created_at \
             FROM idea_versions WHERE idea_id = ?1 ORDER BY version_number ASC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok(IdeaVersion {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    version_number: r.get(2)?,
                    context_json: r.get(3)?,
                    current_step: r.get(4)?,
                    branch_id: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn get_latest_version(&self, idea_id: &str, branch_id: &str) -> Result<Option<IdeaVersion>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, version_number, context_json, current_step, branch_id, created_at \
             FROM idea_versions WHERE idea_id = ?1 AND branch_id = ?2 \
             ORDER BY version_number DESC LIMIT 1",
        )?;
        let v = stmt
            .query_row(params![idea_id, branch_id], |r| {
                Ok(IdeaVersion {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    version_number: r.get(2)?,
                    context_json: r.get(3)?,
                    current_step: r.get(4)?,
                    branch_id: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })
            .ok();
        Ok(v)
    }

    pub fn get_next_version_number(&self, idea_id: &str) -> Result<i32> {
        let c = self.conn();
        let max: Option<i32> = c
            .query_row(
                "SELECT MAX(version_number) FROM idea_versions WHERE idea_id = ?1",
                params![idea_id],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        Ok(max.unwrap_or(0) + 1)
    }

    // ── Idea Branches ────────────────────────────────────────────

    pub fn insert_idea_branch(&self, b: &IdeaBranch) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO idea_branches (id, idea_id, name, parent_branch_id, parent_version_id, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![b.id, b.idea_id, b.name, b.parent_branch_id, b.parent_version_id, b.status, b.created_at],
        )?;
        Ok(())
    }

    pub fn get_idea_branches(&self, idea_id: &str) -> Result<Vec<IdeaBranch>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, name, parent_branch_id, parent_version_id, status, created_at \
             FROM idea_branches WHERE idea_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok(IdeaBranch {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    name: r.get(2)?,
                    parent_branch_id: r.get(3)?,
                    parent_version_id: r.get(4)?,
                    status: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── Findings ────────────────────────────────────────────

    pub fn insert_finding(&self, f: &Finding) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO findings (id, idea_id, finding_type, title, content, source_step, branch_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![f.id, f.idea_id, f.finding_type, f.title, f.content, f.source_step, f.branch_id, f.created_at],
        )?;
        Ok(())
    }

    pub fn get_findings_by_idea(&self, idea_id: &str) -> Result<Vec<Finding>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, finding_type, title, content, source_step, branch_id, created_at \
             FROM findings WHERE idea_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok(Finding {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    finding_type: r.get(2)?,
                    title: r.get(3)?,
                    content: r.get(4)?,
                    source_step: r.get(5)?,
                    branch_id: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn search_findings(&self, query: &str) -> Result<Vec<Finding>> {
        let c = self.conn();
        let pattern = format!("%{}%", query);
        let mut stmt = c.prepare(
            "SELECT id, idea_id, finding_type, title, content, source_step, branch_id, created_at \
             FROM findings WHERE title LIKE ?1 OR content LIKE ?1 \
             ORDER BY created_at DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map(params![pattern], |r| {
                Ok(Finding {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    finding_type: r.get(2)?,
                    title: r.get(3)?,
                    content: r.get(4)?,
                    source_step: r.get(5)?,
                    branch_id: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}
