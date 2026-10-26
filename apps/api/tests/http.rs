//! HTTP-Vertragstests ohne Datenbank und gegen eine nicht erreichbare DB.

use akl_api::{AppState, app};
use akl_persistence::Database;
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn get_json(state: AppState, uri: &str) -> (u16, Value, axum::http::HeaderMap) {
    let res = app(state, None)
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status().as_u16();
    let headers = res.headers().clone();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap(), headers)
}

#[tokio::test]
async fn healthz_is_ok_and_sets_headers() {
    let (code, body, headers) = get_json(AppState::default(), "/healthz").await;
    assert_eq!(code, 200);
    assert_eq!(body["status"], "ok");
    assert!(headers.contains_key("x-request-id"));
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-frame-options"], "DENY");
}

#[tokio::test]
async fn status_without_database_is_honest() {
    let (code, body, _) = get_json(AppState::default(), "/api/v1/system/status").await;
    assert_eq!(code, 200);
    assert_eq!(body["service"], "ai-keyless");
    assert_eq!(body["unofficial"], true);
    assert_eq!(
        body["source_url"],
        "https://github.com/itsh-neumeier/ai-keyless"
    );
    assert_eq!(body["database"]["status"], "not_configured");
    let devices = body["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 2);
    for device in devices {
        // Kein erfundener Zustand, keine vorgetäuschte Evidenz.
        assert!(device["state"].is_null(), "{device}");
        assert_eq!(device["lifecycle_evidence"], "planned");
    }
    assert_eq!(devices[0]["profile"], "ai_key");
    assert_eq!(devices[1]["profile"], "ai_port");
}

#[tokio::test]
async fn unreachable_database_reports_unavailable_and_not_ready() {
    // Port 9 (discard) auf Loopback: Verbindung schlägt schnell fehl.
    let db = Database::connect_lazy("postgres://akl:x@127.0.0.1:9/akl").unwrap();
    let state = AppState { db: Some(db) };
    let (_, body, _) = get_json(state.clone(), "/api/v1/system/status").await;
    assert_eq!(body["database"]["status"], "unavailable");
    assert!(body["devices"][0]["state"].is_null());

    let (code, body, _) = get_json(state, "/readyz").await;
    assert_eq!(code, 503);
    assert_eq!(body["ready"], false);
}

#[tokio::test]
async fn unknown_api_route_is_404() {
    let res = app(AppState::default(), None)
        .oneshot(Request::get("/api/v1/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 404);
}

#[tokio::test]
async fn spa_fallback_serves_index_but_not_for_api() {
    let dir = std::env::temp_dir().join(format!("akl-web-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>AKL</title>").unwrap();

    let router = app(AppState::default(), Some(&dir));
    let res = router
        .clone()
        .oneshot(Request::get("/cameras").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 200);
    assert!(res.headers().contains_key("content-security-policy"));

    let res = router
        .oneshot(Request::get("/api/v1/unknown").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 404);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "not_found");
    let _ = std::fs::remove_dir_all(dir);
}
