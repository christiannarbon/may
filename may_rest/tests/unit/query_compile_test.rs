use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serial_test::serial;
use std::sync::Arc;
use tower::ServiceExt;

// The include path is relative to THIS test file (may_rest/tests/unit/).
// `../../../demos/...` resolves to the repo-root `demos/` directory.
const DEMO_MODEL_YAML: &str = include_str!("../../../demos/valid_demo/ecommerce_model.yml");

fn build_app_with_model() -> Router {
    let state_mgr = Arc::new(may_core::StateMgr::new());
    state_mgr
        .load_from_yaml(DEMO_MODEL_YAML)
        .expect("demo model loads");

    let mut state = crate::support::test_state();
    state.state_mgr = state_mgr;

    may_rest::build_router(state)
}

async fn post_query(app: Router, body: &'static str) -> (StatusCode, serde_json::Value) {
    let token = crate::support::mint_token("admin");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
#[serial]
async fn valid_query_compiles_to_sql_200() {
    let app = build_app_with_model();
    let (status, json) = post_query(
        app,
        r#"{"metrics":["revenue_by_status"],"dimensions":["status"]}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["metric"], "revenue_by_status");
    let sql = json["sql"].as_str().expect("sql is a string");
    assert!(!sql.is_empty(), "compiled SQL should be non-empty");
}

#[tokio::test]
#[serial]
async fn unknown_metric_returns_400() {
    let app = build_app_with_model();
    let (status, json) = post_query(app, r#"{"metrics":["does_not_exist"],"dimensions":[]}"#).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].is_string());
}
