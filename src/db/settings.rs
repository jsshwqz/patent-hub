use anyhow::Result;
use rusqlite::{params, OptionalExtension};

impl super::Database {
    // ── App Settings (SQLite-based persistence, works on Android) ──

    /// 获取单个设置项
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let c = self.conn();
        let result = c
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(result)
    }

    /// 保存设置项（插入或更新）
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value],
        )?;
        Ok(())
    }

    /// 获取所有设置项
    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>> {
        let c = self.conn();
        let mut stmt = c.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    // ── 搜索结果缓存 / Search Cache ──

    /// 查询缓存（未过期才返回）/ Get cached search results if not expired
    pub fn get_search_cache(&self, query_hash: &str) -> Result<Option<String>> {
        let c = self.conn();
        let mut stmt = c.prepare(
            "SELECT results_json FROM search_cache \
             WHERE query_hash = ?1 AND expires_at > datetime('now')",
        )?;
        let result = stmt
            .query_row(params![query_hash], |r| r.get::<_, String>(0))
            .ok();
        Ok(result)
    }

    /// 写入搜索缓存（TTL 24h）/ Write search cache with 24h TTL
    pub fn set_search_cache(
        &self,
        query_hash: &str,
        query_text: &str,
        results_json: &str,
        source: &str,
    ) -> Result<()> {
        let c = self.conn();
        c.execute(
            "INSERT OR REPLACE INTO search_cache (query_hash, query_text, results_json, source, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now', '+24 hours'))",
            params![query_hash, query_text, results_json, source],
        )?;
        Ok(())
    }

    /// 清理过期缓存 / Purge expired cache entries
    pub fn purge_expired_cache(&self) -> Result<usize> {
        let c = self.conn();
        let deleted = c.execute(
            "DELETE FROM search_cache WHERE expires_at <= datetime('now')",
            [],
        )?;
        Ok(deleted)
    }
}
