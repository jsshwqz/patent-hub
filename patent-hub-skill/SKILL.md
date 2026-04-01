---
name: patent-hub-dev
description: "Develop, fix, and extend Patent Hub — a Rust/Axum patent search & AI analysis platform. Trigger when: adding pipeline steps, fixing security issues (XSS/SSRF), writing database migrations, adding API routes, modifying AI prompts, or any patent-hub feature work. Also trigger on mentions of patent-hub, idea validation pipeline, patent search, or novelty scoring."
---

# Patent Hub Development Skill

Guides development of **patent-hub** v0.4.x — a Rust Axum platform for patent search, AI analysis, and innovation validation.

## Architecture Overview

```
src/
├── main.rs              # Axum server entry, route registration
├── lib.rs               # Library exports + Android JNI
├── ai.rs                # Multi-provider AI client (6 providers with failover)
├── db.rs                # SQLite + FTS5, schema migrations (v1-v5)
├── patent.rs            # Data models (Patent, Idea, SearchType)
├── error.rs             # Error types
├── routes/
│   ├── mod.rs           # Router + AppState (AppConfig, pipeline_channels)
│   ├── search.rs        # Search APIs (local + online multi-source fallback)
│   ├── ai.rs            # AI chat, summarize, compare endpoints
│   ├── idea.rs          # Idea validation + multi-round discussion
│   ├── patent.rs        # Patent detail, enrich, PDF export, image proxy
│   ├── collections.rs   # Favorites and tags
│   ├── settings.rs      # Runtime configuration
│   ├── upload.rs        # File upload (PDF, DOCX, images)
│   ├── ipc.rs           # IPC classification lookup
│   └── pages.rs         # HTML page rendering
├── pipeline/
│   ├── runner.rs         # State machine executor (loop over steps)
│   ├── state.rs          # PipelineStep enum (13 variants) + metadata
│   ├── context.rs        # PipelineContext (data passed between steps)
│   └── steps/            # Individual step implementations
│       ├── parse.rs      # Step 0: Extract keywords from idea
│       ├── expand.rs     # Step 1: AI query expansion
│       ├── search.rs     # Steps 2-3: Web + patent search
│       ├── diversity.rs  # Step 4: Coverage gate (may trigger retry)
│       ├── similarity.rs # Step 5: TF-IDF + Jaccard scoring
│       ├── rank.rs       # Step 6: Sort and filter matches
│       ├── contradiction.rs # Step 7: Conflict detection
│       ├── scoring.rs    # Step 8: Novelty score computation
│       ├── analysis.rs   # Steps 9-10: AI deep analysis + action plan
│       └── finalize.rs   # Step 11: Report generation
└── skill_router/         # CLI skill routing engine (separate binary)
```

## Common Evolution Tasks

### Adding a new pipeline step

This is the most complex change — requires modifying 4 files in sync:

1. **`src/pipeline/state.rs`**: Add variant to `PipelineStep` enum, update `next()` chain, add `index()`, `description()`, `is_critical()`, and `skipped_in_quick_mode()` match arms. Update `TOTAL_STEPS` constant.

2. **`src/pipeline/context.rs`**: Add fields to `PipelineContext` for the step's output data. Initialize them in `PipelineContext::new()`.

3. **`src/pipeline/steps/new_step.rs`**: Create the step implementation with signature:
```rust
use super::super::context::PipelineContext;
use anyhow::Result;

pub async fn execute(ctx: &mut PipelineContext) -> Result<()> {
    // Read from ctx fields set by previous steps
    // Write results to ctx fields for downstream steps
    Ok(())
}
```
If the step needs AI, add `ai_client: &AiClient` parameter.

4. **`src/pipeline/runner.rs`**: Add the match arm in `execute_step()`:
```rust
PipelineStep::NewStep => steps::new_step::execute(ctx).await,
```

5. **`src/pipeline/steps/mod.rs`**: Add `pub mod new_step;`

6. **Update tests**: Add integration test in `tests/patent_hub_integration.rs`

7. Build and test: `cargo build --release && cargo test`

**Critical**: When inserting a step in the middle of the chain, you must update ALL subsequent `next()` mappings AND `index()` values in `state.rs`. The `TOTAL_STEPS` constant must match the actual count.

### Adding a database migration

Migrations live in `src/db.rs` inside `Database::init()`. Follow this pattern:

1. Increment `SCHEMA_VERSION` constant (e.g., 5 → 6)
2. Add a new `if current_version < N` block AFTER the last migration:
```rust
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

        DELETE FROM schema_version;
        INSERT INTO schema_version (version) VALUES (6);
    ")?;
    tracing::info!("Database migrated to version 6");
}
```

**Rules**:
- Always use `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS` for idempotency
- Always end with `DELETE FROM schema_version; INSERT INTO schema_version (version) VALUES (N);`
- Use `TEXT` for IDs (UUID strings), `REAL` for scores, `TEXT DEFAULT (datetime('now'))` for timestamps
- Add FOREIGN KEY constraints where applicable
- Add indexes for columns used in WHERE/JOIN clauses

### Adding an API route

1. **Handler function** in the appropriate `src/routes/*.rs` file:
```rust
pub async fn api_new_endpoint(
    State(state): State<AppState>,
    Json(body): Json<NewEndpointRequest>,
) -> impl IntoResponse {
    match do_work(&state, &body).await {
        Ok(result) => Json(serde_json::json!({"status": "ok", "data": result})).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status": "error", "message": e.to_string()}))
        ).into_response(),
    }
}
```

2. **Register route** in `src/main.rs` router:
```rust
.route("/api/new-endpoint", post(routes::api_new_endpoint))
```

3. **Response format**: Always use `{"status": "ok/error", ...}` — this is the project's convention.

### Fixing security issues

**XSS in HTML templates**: Search for `innerHTML` in `templates/*.html`. Replace with:
- `textContent` for plain text
- `DOMPurify.sanitize(html)` for formatted content (add DOMPurify CDN to template head)

**SSRF in image proxy**: In `src/routes/patent.rs`, the image proxy endpoint must validate URL domains against an allowlist:
```rust
const ALLOWED_DOMAINS: &[&str] = &[
    "patentimages.storage.googleapis.com",
    "worldwide.espacenet.com",
    "image.patent.k.sogou.com",
];
```

### Modifying AI prompts

AI prompts are in two locations:
- `src/routes/ai.rs` — chat, summarize, compare prompts
- `src/pipeline/steps/analysis.rs` — deep_analysis and action_plan prompts

When modifying prompts:
- Keep the Chinese/English bilingual structure
- Test with at least 2 different AI providers (different models handle prompts differently)
- Long prompts should be `const` strings at module level, not inline

## Testing

Run: `cargo build --release && cargo test`

Integration tests in `tests/patent_hub_integration.rs` use in-memory SQLite. When adding tests:
- Use `Database::init(":memory:")` for isolation
- Create sample data with `insert_patent()` helper
- Test Chinese character support explicitly (this is a bilingual app)
- For pipeline tests, mock AI responses by testing individual steps with pre-filled `PipelineContext`

**Critical**: When changing `SCHEMA_VERSION`, update existing tests that assert the version number. Search for the old version value in `tests/patent_hub_integration.rs` — tests like `schema_version_is_set_on_fresh_db` and `reinit_same_db_is_idempotent` hard-code the expected version.

**Test completeness checklist**:
- New DB methods → add insert + retrieve + empty-result tests
- New pipeline steps → add unit test in step file + integration test with sample context
- Security fixes → verify no regression in UI functionality (build is sufficient since templates are `include_str!`)

## Known Constraints

- `std::sync::Mutex` wraps SQLite Connection (not `tokio::sync::Mutex`) — blocks tokio threads briefly on DB access
- No OpenSSL dependency — uses `rustls-tls` exclusively
- SQLite is bundled (rusqlite `bundled` feature) — no system SQLite needed
- Templates are `include_str!` embedded in binary — changes require rebuild
- pipeline_channels HashMap in routes/mod.rs never cleans up completed channels (memory leak on heavy use)
- AI timeout: 120s × 2 retries × 6 providers = worst-case 24 minutes for a single AI call chain
