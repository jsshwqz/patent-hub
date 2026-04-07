//! 数据库操作层 / Database Operations
//!
//! 基于 SQLite + FTS5 的本地数据持久化，支持全文搜索、创意管理、收藏夹、标签等。
//! Local data persistence with SQLite + FTS5, supporting full-text search, ideas, collections, tags.

mod migrations;
mod patent;
mod idea;
mod collection;
mod evidence;
mod settings;
pub(crate) mod relevance;
#[cfg(test)]
mod tests;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

/// 数据库实例 / Database instance
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 获取数据库连接，自动恢复被污染的互斥锁
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("Database mutex was poisoned, recovering");
            poisoned.into_inner()
        })
    }

    /// Current schema version. Increment when adding migrations.
    pub(crate) const SCHEMA_VERSION: i32 = 8;

    pub fn init(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);",
        )?;
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        migrations::run(&conn, current_version, Self::SCHEMA_VERSION)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Read current schema version (for testing/diagnostics).
    #[allow(dead_code)]
    pub fn query_schema_version(&self) -> Result<i32> {
        let c = self.conn();
        let v: i32 = c.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )?;
        Ok(v)
    }
}
