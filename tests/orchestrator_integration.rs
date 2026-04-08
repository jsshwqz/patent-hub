//! Orchestrator 集成测试
//! 验证状态机编排的线性流、跳转、版本追踪功能

use innoforge::db::Database;
use innoforge::patent::Idea;

fn create_test_idea(db: &Database, idea_id: &str) {
    let idea = Idea {
        id: idea_id.to_string(),
        title: "test idea".to_string(),
        description: "test description".to_string(),
        input_type: "text".to_string(),
        status: "pending".to_string(),
        analysis: String::new(),
        web_results: "[]".to_string(),
        patent_results: "[]".to_string(),
        novelty_score: None,
        created_at: "2026-04-08T00:00:00Z".to_string(),
        updated_at: "2026-04-08T00:00:00Z".to_string(),
        discussion_summary: String::new(),
    };
    db.insert_idea(&idea).unwrap();
}

#[test]
fn orchestrator_version_tracking_on_fresh_db() {
    // 验证新数据库有 idea_versions 表且可插入
    let db = Database::init(":memory:").unwrap();
    create_test_idea(&db, "idea-001");

    let version = innoforge::db::version::IdeaVersion {
        id: "v-test-001".to_string(),
        idea_id: "idea-001".to_string(),
        version_number: 1,
        context_json: r#"{"test": true}"#.to_string(),
        current_step: "ParseInput".to_string(),
        branch_id: "main".to_string(),
        created_at: "2026-04-08T00:00:00Z".to_string(),
    };
    db.insert_idea_version(&version).unwrap();

    let versions = db.get_idea_versions("idea-001").unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version_number, 1);
    assert_eq!(versions[0].branch_id, "main");
}

#[test]
fn orchestrator_branch_crud() {
    let db = Database::init(":memory:").unwrap();
    create_test_idea(&db, "idea-001");

    let branch = innoforge::db::version::IdeaBranch {
        id: "br-001".to_string(),
        idea_id: "idea-001".to_string(),
        name: "design-around-patent-A".to_string(),
        parent_branch_id: "main".to_string(),
        parent_version_id: Some("v-001".to_string()),
        status: "active".to_string(),
        created_at: "2026-04-08T00:00:00Z".to_string(),
    };
    db.insert_idea_branch(&branch).unwrap();

    let branches = db.get_idea_branches("idea-001").unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "design-around-patent-A");
    assert_eq!(branches[0].parent_branch_id, "main");
}

#[test]
fn orchestrator_findings_crud() {
    let db = Database::init(":memory:").unwrap();
    create_test_idea(&db, "idea-001");

    let finding = innoforge::db::version::Finding {
        id: "f-001".to_string(),
        idea_id: "idea-001".to_string(),
        finding_type: "dead_end".to_string(),
        title: "SearchWeb 失败".to_string(),
        content: "网络不可达".to_string(),
        source_step: "SearchWeb".to_string(),
        branch_id: "main".to_string(),
        created_at: "2026-04-08T00:00:00Z".to_string(),
    };
    db.insert_finding(&finding).unwrap();

    let findings = db.get_findings_by_idea("idea-001").unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].finding_type, "dead_end");

    // 搜索
    let results = db.search_findings("SearchWeb").unwrap();
    assert_eq!(results.len(), 1);

    let empty = db.search_findings("不存在的关键词").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn orchestrator_latest_version() {
    let db = Database::init(":memory:").unwrap();
    create_test_idea(&db, "idea-001");

    for i in 1..=3 {
        let v = innoforge::db::version::IdeaVersion {
            id: format!("v-{}", i),
            idea_id: "idea-001".to_string(),
            version_number: i,
            context_json: format!(r#"{{"step": {}}}"#, i),
            current_step: format!("Step{}", i),
            branch_id: "main".to_string(),
            created_at: format!("2026-04-08T00:00:0{}Z", i),
        };
        db.insert_idea_version(&v).unwrap();
    }

    let latest = db.get_latest_version("idea-001", "main").unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().version_number, 3);

    // 不存在的分支
    let none = db.get_latest_version("idea-001", "nonexistent").unwrap();
    assert!(none.is_none());
}

#[test]
fn orchestrator_next_version_number() {
    let db = Database::init(":memory:").unwrap();
    create_test_idea(&db, "idea-new");

    // 空时返回 1
    assert_eq!(db.get_next_version_number("idea-new").unwrap(), 1);

    let v = innoforge::db::version::IdeaVersion {
        id: "v-1".to_string(),
        idea_id: "idea-new".to_string(),
        version_number: 5,
        context_json: "{}".to_string(),
        current_step: "Finalize".to_string(),
        branch_id: "main".to_string(),
        created_at: "2026-04-08T00:00:00Z".to_string(),
    };
    db.insert_idea_version(&v).unwrap();

    assert_eq!(db.get_next_version_number("idea-new").unwrap(), 6);
}

#[test]
fn migration_v11_claim_tables_exist() {
    let db = Database::init(":memory:").unwrap();
    let version = db.query_schema_version().unwrap();
    assert_eq!(version, 11);

    // 验证 idea_versions/idea_branches/findings 表都可用（通过 CRUD 方法）
    assert!(db.get_idea_versions("nonexistent").unwrap().is_empty());
    assert!(db.get_idea_branches("nonexistent").unwrap().is_empty());
    assert!(db.get_findings_by_idea("nonexistent").unwrap().is_empty());
}
