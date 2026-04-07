use anyhow::Result;
use rusqlite::Connection;

/// Run all pending database migrations.
pub(crate) fn run(conn: &Connection, current_version: i32, target_version: i32) -> Result<()> {
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

    // Migration 4 → 5: App settings table (for Android persistence)
    if current_version < 5 {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );

            DELETE FROM schema_version;
            INSERT INTO schema_version (version) VALUES (5);
        ",
        )?;
        tracing::info!("Database migrated to version 5 (app_settings 表)");
    }

    // Migration 5 → 6: Feature cards
    if current_version < 6 {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS feature_cards (
                id TEXT PRIMARY KEY,
                idea_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT DEFAULT '',
                novelty_score REAL,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (idea_id) REFERENCES ideas(id)
            );
            CREATE INDEX IF NOT EXISTS idx_fc_idea ON feature_cards(idea_id);
            CREATE INDEX IF NOT EXISTS idx_fc_score ON feature_cards(novelty_score);

            DELETE FROM schema_version;
            INSERT INTO schema_version (version) VALUES (6);
        ")?;
        tracing::info!("Database migrated to version 6 (feature_cards)");
    }

    // Migration 6 → 7: 管道快照 + 搜索缓存 / Pipeline snapshots + search cache
    if current_version < 7 {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS pipeline_snapshots (
                idea_id TEXT PRIMARY KEY,
                context_json TEXT NOT NULL,
                current_step TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS search_cache (
                query_hash TEXT PRIMARY KEY,
                query_text TEXT NOT NULL,
                results_json TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                expires_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cache_expires ON search_cache(expires_at);

            DELETE FROM schema_version;
            INSERT INTO schema_version (version) VALUES (7);
        ")?;
        tracing::info!("Database migrated to version 7 (pipeline_snapshots + search_cache)");
    }

    // Migration 7 → 8: 证据链 / Evidence chain
    if current_version < 8 {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS evidence_chain (
                id TEXT PRIMARY KEY,
                idea_id TEXT NOT NULL,
                claim TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_title TEXT NOT NULL,
                source_url TEXT DEFAULT '',
                claim_number TEXT,
                excerpt TEXT NOT NULL,
                relation TEXT NOT NULL DEFAULT 'supports',
                confidence REAL NOT NULL DEFAULT 0.0,
                produced_by TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (idea_id) REFERENCES ideas(id)
            );
            CREATE INDEX IF NOT EXISTS idx_ev_idea ON evidence_chain(idea_id);
            CREATE INDEX IF NOT EXISTS idx_ev_confidence ON evidence_chain(confidence);

            DELETE FROM schema_version;
            INSERT INTO schema_version (version) VALUES (8);
        ")?;
        tracing::info!("Database migrated to version 8 (evidence_chain)");
    }

    if current_version > 0 && current_version < target_version {
        tracing::info!(
            "Database migrated from version {} to {}",
            current_version,
            target_version
        );
    }

    Ok(())
}
