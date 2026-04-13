//! 特征卡片 API 路由 / Feature cards API routes
//!
//! CRUD 操作 + 卡片间最小差异比较

use super::AppState;
use crate::patent::{CreateFeatureCardRequest, FeatureCard};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// 获取指定创意的所有特征卡片 / Get all feature cards for an idea
pub async fn api_get_feature_cards(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
) -> impl IntoResponse {
    // 校验创意存在 / Verify the idea exists
    match s.db.get_idea(&idea_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "error", "message": "idea not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": e.to_string()})),
            )
                .into_response();
        }
    }

    match s.db.get_feature_cards_by_idea(&idea_id) {
        Ok(cards) => Json(json!({"status": "ok", "cards": cards})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string()})),
        )
            .into_response(),
    }
}

/// 创建特征卡片 / Create a feature card for an idea
pub async fn api_create_feature_card(
    State(s): State<AppState>,
    Path(idea_id): Path<String>,
    Json(req): Json<CreateFeatureCardRequest>,
) -> impl IntoResponse {
    // 输入校验 / Validate input
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": "title is required"})),
        )
            .into_response();
    }
    if title.chars().count() > 500 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "error", "message": "title too long (max 500 chars)"})),
        )
            .into_response();
    }

    // 校验创意存在 / Verify the idea exists
    match s.db.get_idea(&idea_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "error", "message": "idea not found"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": e.to_string()})),
            )
                .into_response();
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let card = FeatureCard {
        id: id.clone(),
        idea_id: idea_id.clone(),
        title,
        description: req.description.trim().to_string(),
        novelty_score: req.novelty_score,
        created_at: now,
        technical_problem: req.technical_problem.trim().to_string(),
        core_structure: req.core_structure.trim().to_string(),
        key_relations: req.key_relations.trim().to_string(),
        process_steps: req.process_steps.trim().to_string(),
        application_scenarios: req.application_scenarios.trim().to_string(),
    };

    match s.db.insert_feature_card(&card) {
        Ok(()) => Json(json!({"status": "ok", "card": card})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/feature-cards/diff?a=ID_A&b=ID_B
/// 最小差异定位：对比两张特征卡片的标题和描述
/// Minimal diff: compare two feature cards' title and description
pub async fn api_feature_card_diff(
    State(s): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let id_a = match params.get("a") {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": "缺少参数 a"})),
            )
                .into_response();
        }
    };
    let id_b = match params.get("b") {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": "缺少参数 b"})),
            )
                .into_response();
        }
    };

    let card_a = match s.db.get_feature_card(id_a) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "error", "message": format!("卡片 {} 不存在", id_a)})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": e.to_string()})),
            )
                .into_response();
        }
    };

    let card_b = match s.db.get_feature_card(id_b) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"status": "error", "message": format!("卡片 {} 不存在", id_b)})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": e.to_string()})),
            )
                .into_response();
        }
    };

    let diff = compute_minimal_diff(&card_a, &card_b);
    let (diff_type, novelty_significance, novelty_reason) = classify_diff(&card_a, &card_b);
    Json(json!({
        "status": "ok",
        "card_a": { "id": card_a.id, "title": card_a.title },
        "card_b": { "id": card_b.id, "title": card_b.title },
        "diff": diff,
        "diff_type": diff_type,
        "novelty_significance": novelty_significance,
        "novelty_reason": novelty_reason,
    }))
    .into_response()
}

/// 计算两张卡片的最小差异 / Compute minimal diff between two cards
///
/// 使用字符级 LCS 定位差异段落，返回结构化差异列表
fn compute_minimal_diff(a: &FeatureCard, b: &FeatureCard) -> serde_json::Value {
    let title_diff = diff_strings(&a.title, &b.title);
    let desc_diff = diff_strings(&a.description, &b.description);
    let score_diff = match (a.novelty_score, b.novelty_score) {
        (Some(sa), Some(sb)) => json!({
            "a": format!("{:.1}", sa),
            "b": format!("{:.1}", sb),
            "delta": format!("{:+.1}", sb - sa),
        }),
        _ => json!(null),
    };
    let tp_diff = diff_strings(&a.technical_problem, &b.technical_problem);
    let cs_diff = diff_strings(&a.core_structure, &b.core_structure);
    let kr_diff = diff_strings(&a.key_relations, &b.key_relations);
    let ps_diff = diff_strings(&a.process_steps, &b.process_steps);
    let as_diff = diff_strings(&a.application_scenarios, &b.application_scenarios);

    json!({
        "title": title_diff,
        "description": desc_diff,
        "novelty_score": score_diff,
        "technical_problem": tp_diff,
        "core_structure": cs_diff,
        "key_relations": kr_diff,
        "process_steps": ps_diff,
        "application_scenarios": as_diff,
    })
}

/// 差异类型分类 + 新颖性判断
fn classify_diff(a: &FeatureCard, b: &FeatureCard) -> (String, bool, String) {
    let structure_changed = a.core_structure != b.core_structure;
    let method_changed = a.process_steps != b.process_steps;
    let problem_changed = a.technical_problem != b.technical_problem;
    let relations_changed = a.key_relations != b.key_relations;
    let scenarios_changed = a.application_scenarios != b.application_scenarios;

    let diff_type = if structure_changed {
        "structure"
    } else if method_changed {
        "method"
    } else if problem_changed || relations_changed || scenarios_changed {
        "parameter"
    } else {
        "none"
    };

    let novelty_significance = structure_changed || method_changed;

    let novelty_reason = if structure_changed {
        "核心结构采用了不同的技术方案".to_string()
    } else if method_changed {
        "工艺/实施步骤存在差异".to_string()
    } else if problem_changed || relations_changed || scenarios_changed {
        "参数级差异，新颖性意义较低".to_string()
    } else {
        "无显著差异".to_string()
    };

    (diff_type.to_string(), novelty_significance, novelty_reason)
}

/// 字符级差异提取 / Character-level diff extraction
///
/// 基于最长公共子序列（LCS），输出增删操作列表
fn diff_strings(a: &str, b: &str) -> Vec<serde_json::Value> {
    if a == b {
        return vec![json!({"type": "equal", "text": a})];
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // LCS 表（空间优化：只保留两行）
    // 对超长文本截断避免 O(m*n) 爆内存
    if m > 5000 || n > 5000 {
        return vec![
            json!({"type": "remove", "text": a}),
            json!({"type": "add", "text": b}),
        ];
    }

    // 计算 LCS 表
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a_chars[i - 1] == b_chars[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }

    // 回溯生成差异
    let mut ops: Vec<serde_json::Value> = Vec::new();
    let mut i = m;
    let mut j = n;
    let mut buf_eq = String::new();
    let mut buf_rm = String::new();
    let mut buf_add = String::new();

    // 先收集反向操作，最后翻转
    let mut raw_ops: Vec<(char, char)> = Vec::new(); // ('=', ch), ('-', ch), ('+', ch)
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a_chars[i - 1] == b_chars[j - 1] {
            raw_ops.push(('=', a_chars[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            raw_ops.push(('+', b_chars[j - 1]));
            j -= 1;
        } else {
            raw_ops.push(('-', a_chars[i - 1]));
            i -= 1;
        }
    }
    raw_ops.reverse();

    // 合并连续的同类操作
    let flush = |ops: &mut Vec<serde_json::Value>,
                 buf_eq: &mut String,
                 buf_rm: &mut String,
                 buf_add: &mut String| {
        if !buf_rm.is_empty() {
            ops.push(json!({"type": "remove", "text": buf_rm.clone()}));
            buf_rm.clear();
        }
        if !buf_add.is_empty() {
            ops.push(json!({"type": "add", "text": buf_add.clone()}));
            buf_add.clear();
        }
        if !buf_eq.is_empty() {
            ops.push(json!({"type": "equal", "text": buf_eq.clone()}));
            buf_eq.clear();
        }
    };

    for (op, ch) in raw_ops {
        match op {
            '=' => {
                // 先刷新增删
                if !buf_rm.is_empty() {
                    ops.push(json!({"type": "remove", "text": buf_rm.clone()}));
                    buf_rm.clear();
                }
                if !buf_add.is_empty() {
                    ops.push(json!({"type": "add", "text": buf_add.clone()}));
                    buf_add.clear();
                }
                buf_eq.push(ch);
            }
            '-' => {
                if !buf_eq.is_empty() {
                    ops.push(json!({"type": "equal", "text": buf_eq.clone()}));
                    buf_eq.clear();
                }
                buf_rm.push(ch);
            }
            '+' => {
                if !buf_eq.is_empty() {
                    ops.push(json!({"type": "equal", "text": buf_eq.clone()}));
                    buf_eq.clear();
                }
                buf_add.push(ch);
            }
            _ => {}
        }
    }
    // 最后刷新
    flush(&mut ops, &mut buf_eq, &mut buf_rm, &mut buf_add);

    ops
}
