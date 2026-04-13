use crate::patent::{FeatureCard, Idea, IdeaSummary};
use anyhow::Result;
use rusqlite::{params, OptionalExtension};

impl super::Database {
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

    /// 启动时将所有卡在 "analyzing" 的创意重置为 "error"（上次 pipeline 中断）
    pub fn reset_stale_analyzing(&self) -> Result<usize> {
        let c = self.conn();
        let count = c.execute(
            "UPDATE ideas SET status='error' WHERE status='analyzing'",
            [],
        )?;
        Ok(count)
    }

    /// 运行时重置超过指定分钟数仍在 "analyzing" 的创意为 "error"
    /// Runtime: reset ideas stuck in "analyzing" for more than N minutes
    pub fn reset_stuck_analyzing(&self, stale_minutes: i64) -> Result<usize> {
        let c = self.conn();
        let count = c.execute(
            "UPDATE ideas SET status='error' \
             WHERE status='analyzing' \
             AND updated_at < datetime('now', ?1)",
            params![format!("-{} minutes", stale_minutes)],
        )?;
        Ok(count)
    }

    pub fn delete_idea(&self, id: &str) -> Result<()> {
        let c = self.conn();
        c.execute("DELETE FROM ideas WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn delete_idea_messages(&self, idea_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM idea_messages WHERE idea_id=?1",
            params![idea_id],
        )?;
        Ok(())
    }

    /// 删除创意关联的所有特征卡片 / Delete all feature cards for an idea
    pub fn delete_feature_cards_by_idea(&self, idea_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM feature_cards WHERE idea_id=?1",
            params![idea_id],
        )?;
        Ok(())
    }

    pub fn list_ideas(&self) -> Result<Vec<IdeaSummary>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT i.id, i.title, i.status, i.novelty_score, i.created_at, COALESCE(i.description,''), \
             (SELECT COUNT(*) FROM idea_messages m WHERE m.idea_id = i.id) \
             FROM ideas i ORDER BY i.created_at DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(IdeaSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    status: r.get(2)?,
                    novelty_score: r.get(3)?,
                    created_at: r.get(4)?,
                    description: r.get(5)?,
                    message_count: r.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── Idea Messages CRUD ───────────────────────────────────────────

    pub fn add_idea_message(
        &self,
        id: &str,
        idea_id: &str,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO idea_messages (id, idea_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params![id, idea_id, role, content],
        )?;
        Ok(())
    }

    pub fn get_idea_messages(
        &self,
        idea_id: &str,
    ) -> Result<Vec<(String, String, String, String)>> {
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
        let summary: String = c
            .query_row(
                "SELECT COALESCE(discussion_summary, '') FROM ideas WHERE id = ?1",
                params![idea_id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        Ok(summary)
    }

    // ── Feature Cards CRUD ──────────────────────────────────────────

    pub fn insert_feature_card(&self, card: &FeatureCard) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO feature_cards (id, idea_id, title, description, novelty_score, created_at, \
             technical_problem, core_structure, key_relations, process_steps, application_scenarios) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                card.id,
                card.idea_id,
                card.title,
                card.description,
                card.novelty_score,
                card.created_at,
                card.technical_problem,
                card.core_structure,
                card.key_relations,
                card.process_steps,
                card.application_scenarios
            ],
        )?;
        Ok(())
    }

    /// 按 ID 获取单张特征卡片 / Get a single feature card by ID
    pub fn get_feature_card(&self, id: &str) -> Result<Option<FeatureCard>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, title, COALESCE(description,''), novelty_score, created_at, \
             COALESCE(technical_problem,''), COALESCE(core_structure,''), \
             COALESCE(key_relations,''), COALESCE(process_steps,''), \
             COALESCE(application_scenarios,'') \
             FROM feature_cards WHERE id = ?1",
        )?;
        let card = stmt
            .query_row(params![id], |r| {
                Ok(FeatureCard {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    novelty_score: r.get(4)?,
                    created_at: r.get(5)?,
                    technical_problem: r.get(6)?,
                    core_structure: r.get(7)?,
                    key_relations: r.get(8)?,
                    process_steps: r.get(9)?,
                    application_scenarios: r.get(10)?,
                })
            })
            .ok();
        Ok(card)
    }

    pub fn get_feature_cards_by_idea(&self, idea_id: &str) -> Result<Vec<FeatureCard>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT id, idea_id, title, COALESCE(description,''), novelty_score, created_at, \
             COALESCE(technical_problem,''), COALESCE(core_structure,''), \
             COALESCE(key_relations,''), COALESCE(process_steps,''), \
             COALESCE(application_scenarios,'') \
             FROM feature_cards WHERE idea_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![idea_id], |r| {
                Ok(FeatureCard {
                    id: r.get(0)?,
                    idea_id: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    novelty_score: r.get(4)?,
                    created_at: r.get(5)?,
                    technical_problem: r.get(6)?,
                    core_structure: r.get(7)?,
                    key_relations: r.get(8)?,
                    process_steps: r.get(9)?,
                    application_scenarios: r.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── 管道断点快照 / Pipeline Snapshots ──────────────────────────────────

    /// 保存管道执行快照（每步完成后调用）/ Save pipeline snapshot after each step
    pub fn save_pipeline_snapshot(
        &self,
        idea_id: &str,
        context_json: &str,
        current_step: &str,
    ) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT OR REPLACE INTO pipeline_snapshots (idea_id, context_json, current_step, updated_at) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![idea_id, context_json, current_step],
        )?;
        Ok(())
    }

    /// 加载管道快照（用于断点续跑）/ Load pipeline snapshot for resume
    pub fn load_pipeline_snapshot(&self, idea_id: &str) -> Result<Option<(String, String)>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT context_json, current_step FROM pipeline_snapshots WHERE idea_id = ?1",
        )?;
        let result = stmt
            .query_row(params![idea_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .ok();
        Ok(result)
    }

    /// 删除管道快照（完成后清理）/ Delete snapshot after pipeline completes
    pub fn delete_pipeline_snapshot(&self, idea_id: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "DELETE FROM pipeline_snapshots WHERE idea_id = ?1",
            params![idea_id],
        )?;
        Ok(())
    }
}
