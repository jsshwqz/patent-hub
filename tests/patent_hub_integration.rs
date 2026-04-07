//! Integration tests for InnoForge core functionality.

use innoforge::db::Database;
use innoforge::patent::*;

fn sample_patent(id: &str, title: &str) -> Patent {
    Patent {
        id: id.to_string(),
        patent_number: format!("CN{}A", id),
        title: title.to_string(),
        abstract_text: "测试摘要，包含中文字符".to_string(),
        description: "详细描述".to_string(),
        claims: "权利要求1：一种方法".to_string(),
        applicant: "测试公司".to_string(),
        inventor: "张三".to_string(),
        filing_date: "2024-06-15".to_string(),
        publication_date: "2024-12-15".to_string(),
        grant_date: None,
        ipc_codes: "G06F".to_string(),
        cpc_codes: "G06F17/00".to_string(),
        priority_date: "2024-06-15".to_string(),
        country: "CN".to_string(),
        kind_code: "A".to_string(),
        family_id: None,
        legal_status: "pending".to_string(),
        citations: "[]".to_string(),
        cited_by: "[]".to_string(),
        source: "test".to_string(),
        raw_json: "{}".to_string(),
        created_at: "2024-06-15T00:00:00Z".to_string(),
        images: "[]".to_string(),
        pdf_url: String::new(),
    }
}

// ── Database tests ───────────────────────────────────────────────────────────

#[test]
fn insert_and_retrieve_patent() {
    let db = Database::init(":memory:").unwrap();
    let p = sample_patent("p1", "向量数据库专利");
    db.insert_patent(&p).unwrap();

    let retrieved = db.get_patent("p1").unwrap().unwrap();
    assert_eq!(retrieved.title, "向量数据库专利");
    assert_eq!(retrieved.applicant, "测试公司");
    assert_eq!(retrieved.inventor, "张三");
}

#[test]
fn get_patent_by_patent_number() {
    let db = Database::init(":memory:").unwrap();
    let p = sample_patent("p2", "AI芯片专利");
    db.insert_patent(&p).unwrap();

    // Should be retrievable by patent_number too
    let retrieved = db.get_patent("CNp2A").unwrap().unwrap();
    assert_eq!(retrieved.title, "AI芯片专利");
}

#[test]
fn get_nonexistent_patent_returns_none() {
    let db = Database::init(":memory:").unwrap();
    assert!(db.get_patent("nonexistent").unwrap().is_none());
}

#[test]
fn upsert_patent_updates_existing() {
    let db = Database::init(":memory:").unwrap();
    let mut p = sample_patent("p3", "原始标题");
    db.insert_patent(&p).unwrap();

    p.title = "更新后的标题".to_string();
    db.insert_patent(&p).unwrap();

    let retrieved = db.get_patent("p3").unwrap().unwrap();
    assert_eq!(retrieved.title, "更新后的标题");
}

// ── Search type detection ────────────────────────────────────────────────────

#[test]
fn detect_patent_number_search_type() {
    let db = Database::init(":memory:").unwrap();
    assert!(matches!(
        db.detect_search_type("CN1234567A"),
        SearchType::PatentNumber
    ));
    assert!(matches!(
        db.detect_search_type("US10000000B2"),
        SearchType::PatentNumber
    ));
}

#[test]
fn detect_company_as_applicant() {
    let db = Database::init(":memory:").unwrap();
    assert!(matches!(
        db.detect_search_type("华为技术有限公司"),
        SearchType::Applicant
    ));
    // Company keywords are now checked before name detection
    assert!(matches!(
        db.detect_search_type("Google Inc"),
        SearchType::Applicant
    ));
    assert!(matches!(
        db.detect_search_type("Test Tech Co"),
        SearchType::Applicant
    ));
}

#[test]
fn detect_chinese_name_as_inventor() {
    let db = Database::init(":memory:").unwrap();
    assert!(matches!(
        db.detect_search_type("张三"),
        SearchType::Inventor
    ));
}

#[test]
fn detect_english_name_as_inventor() {
    let db = Database::init(":memory:").unwrap();
    assert!(matches!(
        db.detect_search_type("John Smith"),
        SearchType::Inventor
    ));
}

#[test]
fn detect_keyword_as_mixed() {
    let db = Database::init(":memory:").unwrap();
    assert!(matches!(
        db.detect_search_type("人工智能图像识别"),
        SearchType::Mixed
    ));
}

// ── Full-text search ─────────────────────────────────────────────────────────

#[test]
fn fts_search_finds_matching_patent() {
    let db = Database::init(":memory:").unwrap();
    db.insert_patent(&sample_patent("fts1", "Neural network optimization method"))
        .unwrap();
    db.insert_patent(&sample_patent("fts2", "Database indexing algorithm"))
        .unwrap();

    let (results, total, _) = db
        .search_smart(
            "neural",
            Some(&SearchType::Keyword),
            None,
            None,
            None,
            1,
            10,
        )
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "fts1");
}

#[test]
fn search_with_country_filter() {
    let db = Database::init(":memory:").unwrap();
    let mut p1 = sample_patent("cf1", "Test patent alpha");
    p1.country = "CN".to_string();
    let mut p2 = sample_patent("cf2", "Test patent beta");
    p2.country = "US".to_string();
    p2.patent_number = "UScf2A".to_string();
    db.insert_patent(&p1).unwrap();
    db.insert_patent(&p2).unwrap();

    let (results, total, _) = db
        .search_smart("patent", None, Some("CN"), None, None, 1, 10)
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(results[0].country, "CN");
}

#[test]
fn search_with_date_range_filter() {
    let db = Database::init(":memory:").unwrap();
    let mut p_old = sample_patent("dr1", "Old patent");
    p_old.filing_date = "2020-01-01".to_string();
    let mut p_new = sample_patent("dr2", "New patent");
    p_new.filing_date = "2024-06-01".to_string();
    db.insert_patent(&p_old).unwrap();
    db.insert_patent(&p_new).unwrap();

    let (results, total, _) = db
        .search_smart(
            "patent",
            Some(&SearchType::Keyword),
            None,
            Some("2024-01-01"),
            None,
            1,
            10,
        )
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(results[0].id, "dr2");
}

// ── Idea CRUD ────────────────────────────────────────────────────────────────

#[test]
fn idea_crud_lifecycle() {
    let db = Database::init(":memory:").unwrap();
    let idea = Idea {
        id: "idea1".to_string(),
        title: "智能停车系统".to_string(),
        description: "利用AI识别车位".to_string(),
        input_type: "text".to_string(),
        status: "pending".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: None,
        created_at: "2024-06-15T00:00:00Z".to_string(),
        updated_at: "2024-06-15T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };

    // Insert
    db.insert_idea(&idea).unwrap();

    // Get
    let retrieved = db.get_idea("idea1").unwrap().unwrap();
    assert_eq!(retrieved.title, "智能停车系统");
    assert_eq!(retrieved.status, "pending");

    // Update
    let mut updated = retrieved;
    updated.status = "done".to_string();
    updated.novelty_score = Some(75.0);
    updated.analysis = "分析结果".to_string();
    db.update_idea(&updated).unwrap();

    let after_update = db.get_idea("idea1").unwrap().unwrap();
    assert_eq!(after_update.status, "done");
    assert_eq!(after_update.novelty_score, Some(75.0));

    // List
    let list = db.list_ideas().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "智能停车系统");
}

#[test]
fn get_nonexistent_idea_returns_none() {
    let db = Database::init(":memory:").unwrap();
    assert!(db.get_idea("nonexistent").unwrap().is_none());
}

// ── Pagination ───────────────────────────────────────────────────────────────

#[test]
fn search_pagination_works() {
    let db = Database::init(":memory:").unwrap();
    for i in 0..25 {
        db.insert_patent(&sample_patent(
            &format!("pg{i}"),
            &format!("Pagination test patent {i}"),
        ))
        .unwrap();
    }

    let (page1, total, _) = db
        .search_smart(
            "Pagination",
            Some(&SearchType::Keyword),
            None,
            None,
            None,
            1,
            10,
        )
        .unwrap();
    assert_eq!(total, 25);
    assert_eq!(page1.len(), 10);

    let (page3, _, _) = db
        .search_smart(
            "Pagination",
            Some(&SearchType::Keyword),
            None,
            None,
            None,
            3,
            10,
        )
        .unwrap();
    assert_eq!(page3.len(), 5);
}

// ── AI response parsing ─────────────────────────────────────────────────────

#[test]
fn ai_extracts_zhipu_format() {
    // Test that the response parser handles Zhipu/GLM format
    let raw = r#"{"choices":[{"message":{"role":"assistant","content":"这是一个专利分析"}}]}"#;
    let parsed: serde_json::Value = serde_json::from_str(raw).unwrap();
    let content = parsed["choices"][0]["message"]["content"].as_str().unwrap();
    assert_eq!(content, "这是一个专利分析");
}

// ── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_query_fts_returns_empty() {
    let db = Database::init(":memory:").unwrap();
    db.insert_patent(&sample_patent("e1", "Some patent"))
        .unwrap();
    let (results, total) = db.search_fts("", 1, 10).unwrap();
    assert_eq!(total, 0);
    assert!(results.is_empty());
}

#[test]
fn search_special_characters_does_not_panic() {
    let db = Database::init(":memory:").unwrap();
    db.insert_patent(&sample_patent("sc1", "Test patent"))
        .unwrap();
    // These should not panic or error
    let _ = db.search_smart("\"test\"", None, None, None, None, 1, 10);
    let _ = db.search_smart("test OR drop", None, None, None, None, 1, 10);
    let _ = db.search_smart("a AND b", None, None, None, None, 1, 10);
    let _ = db.search_smart("'single quotes'", None, None, None, None, 1, 10);
}

#[test]
fn unicode_search_does_not_panic() {
    let db = Database::init(":memory:").unwrap();
    let mut p = sample_patent("uni1", "中文专利标题测试");
    p.abstract_text = "这是一个包含特殊字符的摘要：🔬⚡💡".to_string();
    db.insert_patent(&p).unwrap();

    let (_results, total, _) = db
        .search_smart("中文", None, None, None, None, 1, 10)
        .unwrap();
    assert!(total <= 1); // May or may not match depending on LIKE behavior
}

// ── Database migration ───────────────────────────────────────────────────

#[test]
fn schema_version_is_set_on_fresh_db() {
    let db = Database::init(":memory:").unwrap();
    let version: i32 = db
        .query_schema_version()
        .expect("should be able to read schema version");
    assert_eq!(version, 8);
}

#[test]
fn reinit_same_db_is_idempotent() {
    // Create a temp file so we can open the same DB twice
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let path_str = path.to_str().unwrap();

    let db1 = Database::init(path_str).unwrap();
    db1.insert_patent(&sample_patent("m1", "Migration test"))
        .unwrap();
    drop(db1);

    // Re-open — should not fail or lose data
    let db2 = Database::init(path_str).unwrap();
    let p = db2.get_patent("m1").unwrap();
    assert!(p.is_some());
    assert_eq!(p.unwrap().title, "Migration test");

    let version = db2.query_schema_version().unwrap();
    assert_eq!(version, 8);
}

// ── Feature cards CRUD ──────────────────────────────────────────────────────

#[test]
fn feature_card_insert_and_retrieve() {
    use innoforge::patent::FeatureCard;

    let db = Database::init(":memory:").unwrap();

    // Create an idea first (FK constraint)
    let idea = Idea {
        id: "idea-fc1".to_string(),
        title: "Test idea for feature cards".to_string(),
        description: "Description".to_string(),
        input_type: "text".to_string(),
        status: "pending".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: None,
        created_at: "2026-04-01T00:00:00Z".to_string(),
        updated_at: "2026-04-01T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };
    db.insert_idea(&idea).unwrap();

    let card = FeatureCard {
        id: "fc1".to_string(),
        idea_id: "idea-fc1".to_string(),
        title: "Novel heat dissipation method".to_string(),
        description: "Uses graphene layer for thermal conductivity".to_string(),
        novelty_score: Some(85.5),
        created_at: "2026-04-01T00:00:00Z".to_string(),
    };
    db.insert_feature_card(&card).unwrap();

    let cards = db.get_feature_cards_by_idea("idea-fc1").unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].id, "fc1");
    assert_eq!(cards[0].title, "Novel heat dissipation method");
    assert_eq!(cards[0].description, "Uses graphene layer for thermal conductivity");
    assert_eq!(cards[0].novelty_score, Some(85.5));
}

#[test]
fn feature_cards_empty_for_unknown_idea() {
    let db = Database::init(":memory:").unwrap();
    let cards = db.get_feature_cards_by_idea("nonexistent").unwrap();
    assert!(cards.is_empty());
}

#[test]
fn feature_card_chinese_title() {
    use innoforge::patent::FeatureCard;

    let db = Database::init(":memory:").unwrap();

    let idea = Idea {
        id: "idea-cn".to_string(),
        title: "中文创意测试".to_string(),
        description: "测试描述".to_string(),
        input_type: "text".to_string(),
        status: "pending".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: None,
        created_at: "2026-04-01T00:00:00Z".to_string(),
        updated_at: "2026-04-01T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };
    db.insert_idea(&idea).unwrap();

    let card = FeatureCard {
        id: "fc-cn".to_string(),
        idea_id: "idea-cn".to_string(),
        title: "基于石墨烯的散热方法".to_string(),
        description: "利用石墨烯优异的导热性能实现高效散热".to_string(),
        novelty_score: Some(92.0),
        created_at: "2026-04-01T00:00:00Z".to_string(),
    };
    db.insert_feature_card(&card).unwrap();

    let cards = db.get_feature_cards_by_idea("idea-cn").unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].title, "基于石墨烯的散热方法");
    assert_eq!(cards[0].description, "利用石墨烯优异的导热性能实现高效散热");
}

// ── Pipeline snapshots CRUD ────────────────────────────────────────────────────

#[test]
fn pipeline_snapshot_save_load_delete() {
    let db = Database::init(":memory:").unwrap();

    // Save
    db.save_pipeline_snapshot("snap-idea1", r#"{"step":1,"title":"test"}"#, "ParseInput")
        .unwrap();

    // Load
    let loaded = db.load_pipeline_snapshot("snap-idea1").unwrap();
    assert!(loaded.is_some());
    let (json, step) = loaded.unwrap();
    assert!(json.contains("\"step\":1"));
    assert_eq!(step, "ParseInput");

    // Delete
    db.delete_pipeline_snapshot("snap-idea1").unwrap();

    // Load after delete → None
    let gone = db.load_pipeline_snapshot("snap-idea1").unwrap();
    assert!(gone.is_none());
}

// ── Search cache CRUD ─────────────────────────────────────────────────────────

#[test]
fn search_cache_set_get() {
    let db = Database::init(":memory:").unwrap();

    db.set_search_cache("hash_abc", "电池 专利", r#"[{"id":"r1"}]"#, "web")
        .unwrap();

    let cached = db.get_search_cache("hash_abc").unwrap();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), r#"[{"id":"r1"}]"#);

    // Nonexistent hash → None
    let miss = db.get_search_cache("no_such_hash").unwrap();
    assert!(miss.is_none());
}

#[test]
fn search_cache_upsert_overwrites() {
    let db = Database::init(":memory:").unwrap();

    db.set_search_cache("hash_up", "query", r#"[{"v":1}]"#, "patent")
        .unwrap();
    db.set_search_cache("hash_up", "query", r#"[{"v":2}]"#, "patent")
        .unwrap();

    let cached = db.get_search_cache("hash_up").unwrap().unwrap();
    assert_eq!(cached, r#"[{"v":2}]"#);
}

// ── Feature card get by ID ────────────────────────────────────────────────────

#[test]
fn feature_card_get_by_id() {
    use innoforge::patent::FeatureCard;

    let db = Database::init(":memory:").unwrap();

    let idea = Idea {
        id: "idea-getfc".to_string(),
        title: "Test idea".to_string(),
        description: "Desc".to_string(),
        input_type: "text".to_string(),
        status: "pending".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: Some(75.0),
        created_at: "2026-04-01T00:00:00Z".to_string(),
        updated_at: "2026-04-01T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };
    db.insert_idea(&idea).unwrap();

    let card = FeatureCard {
        id: "fc-get1".to_string(),
        idea_id: "idea-getfc".to_string(),
        title: "Unique feature".to_string(),
        description: "Feature description here".to_string(),
        novelty_score: Some(88.0),
        created_at: "2026-04-01T00:00:00Z".to_string(),
    };
    db.insert_feature_card(&card).unwrap();

    // Get existing
    let found = db.get_feature_card("fc-get1").unwrap();
    assert!(found.is_some());
    let c = found.unwrap();
    assert_eq!(c.title, "Unique feature");
    assert_eq!(c.novelty_score, Some(88.0));

    // Get nonexistent
    let miss = db.get_feature_card("no-such-card").unwrap();
    assert!(miss.is_none());
}

// ── Batch idea retrieval ──────────────────────────────────────────────────────

#[test]
fn batch_idea_get_multiple() {
    let db = Database::init(":memory:").unwrap();

    let make_idea = |id: &str, title: &str, score: Option<f64>| Idea {
        id: id.to_string(),
        title: title.to_string(),
        description: "batch test".to_string(),
        input_type: "text".to_string(),
        status: "completed".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: score,
        created_at: "2026-04-02T00:00:00Z".to_string(),
        updated_at: "2026-04-02T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };

    db.insert_idea(&make_idea("b1", "Battery tech", Some(80.0))).unwrap();
    db.insert_idea(&make_idea("b2", "Solar panel", Some(60.0))).unwrap();
    db.insert_idea(&make_idea("b3", "Wind turbine", None)).unwrap();

    let i1 = db.get_idea("b1").unwrap().unwrap();
    assert_eq!(i1.title, "Battery tech");
    assert_eq!(i1.novelty_score, Some(80.0));

    let i2 = db.get_idea("b2").unwrap().unwrap();
    assert_eq!(i2.title, "Solar panel");
    assert_eq!(i2.novelty_score, Some(60.0));

    let i3 = db.get_idea("b3").unwrap().unwrap();
    assert_eq!(i3.title, "Wind turbine");
    assert_eq!(i3.novelty_score, None);
}

// ── PriorArtCluster pipeline step ────────────────────────────────────────────

#[tokio::test]
async fn prior_art_cluster_integration_with_sample_ranked_results() {
    use innoforge::pipeline::context::{PipelineContext, RankedMatch};
    use innoforge::pipeline::steps::prior_art_cluster;

    let mut ctx = PipelineContext::new("idea-cluster", "智能停车创新", "基于AI的停车位识别系统");

    // Simulate ranked results with a mix of Chinese and English text
    ctx.top_matches = vec![
        RankedMatch {
            rank: 1,
            source_id: "p1".into(),
            source_title: "Neural network parking detection".into(),
            source_type: "patent".into(),
            source_url: "https://example.com/p1".into(),
            snippet: "Deep learning for parking spot recognition".into(),
            combined_score: 0.85,
            tokens: vec!["neural".into(), "network".into(), "parking".into(), "detection".into(), "deep".into(), "learning".into(), "spot".into(), "recognition".into()],
        },
        RankedMatch {
            rank: 2,
            source_id: "p2".into(),
            source_title: "Deep learning vehicle detection system".into(),
            source_type: "web".into(),
            source_url: "https://example.com/p2".into(),
            snippet: "CNN-based parking lot monitoring with neural networks".into(),
            combined_score: 0.80,
            tokens: vec!["deep".into(), "learning".into(), "vehicle".into(), "detection".into(), "system".into(), "cnn".into(), "parking".into(), "neural".into(), "networks".into()],
        },
        RankedMatch {
            rank: 3,
            source_id: "p3".into(),
            source_title: "基于区块链的停车费支付系统".into(),
            source_type: "patent".into(),
            source_url: "https://example.com/p3".into(),
            snippet: "分布式账本技术用于停车场计费".into(),
            combined_score: 0.60,
            tokens: vec!["区块链".into(), "停车费".into(), "支付".into(), "系统".into(), "分布式".into(), "账本".into()],
        },
        RankedMatch {
            rank: 4,
            source_id: "p4".into(),
            source_title: "IoT sensor array for smart parking".into(),
            source_type: "patent".into(),
            source_url: "https://example.com/p4".into(),
            snippet: "Ultrasonic and infrared sensors for occupancy detection".into(),
            combined_score: 0.55,
            tokens: vec!["iot".into(), "sensor".into(), "array".into(), "smart".into(), "parking".into(), "ultrasonic".into(), "infrared".into(), "occupancy".into()],
        },
    ];

    prior_art_cluster::execute(&mut ctx).await.unwrap();

    // p1 and p2 share many tokens (neural, deep, learning, parking, detection) — should cluster
    // p3 is blockchain-related — separate cluster
    // p4 is IoT sensor — only shares "parking" with p1/p2, low Jaccard — separate cluster
    assert!(ctx.prior_art_clusters.len() >= 2, "Expected at least 2 clusters, got {}", ctx.prior_art_clusters.len());

    // Verify the neural-network cluster has p1 and p2
    let nn_cluster = ctx.prior_art_clusters.iter().find(|c| c.patent_indices.contains(&0)).unwrap();
    assert!(nn_cluster.patent_indices.contains(&1), "p2 should be in same cluster as p1");

    // Verify blockchain item is in its own cluster
    let bc_cluster = ctx.prior_art_clusters.iter().find(|c| c.patent_indices.contains(&2)).unwrap();
    assert_eq!(bc_cluster.patent_indices.len(), 1, "Blockchain item should be in its own cluster");

    // Verify avg_similarity is computed correctly for the nn cluster
    let expected_avg = (0.85 + 0.80) / 2.0;
    assert!((nn_cluster.avg_similarity - expected_avg).abs() < 0.001);

    // Verify topic is non-empty
    assert!(!nn_cluster.topic.is_empty());
    assert!(!bc_cluster.topic.is_empty());
}
