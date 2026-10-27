//! HTTP-Vertragstests ohne Datenbank bzw. gegen eine nicht erreichbare DB.

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
async fn without_database_everything_but_health_is_unavailable() {
    for uri in [
        "/api/v1/system/status",
        "/api/v1/auth/state",
        "/api/v1/auth/session",
    ] {
        let (code, body, _) = get_json(AppState::default(), uri).await;
        assert_eq!(code, 503, "{uri}");
        assert_eq!(body["error"]["code"], "database_unavailable", "{uri}");
    }
    let (code, body, _) = get_json(AppState::default(), "/readyz").await;
    assert_eq!(code, 200);
    assert_eq!(body["database"]["status"], "not_configured");
}

#[tokio::test]
async fn unreachable_database_reports_unavailable_and_not_ready() {
    // Port 9 (discard) auf Loopback: Verbindung schlägt schnell fehl.
    let db = Database::connect_lazy("postgres://akl:x@127.0.0.1:9/akl").unwrap();
    let state = AppState::new(Some(db), false);
    let (code, body, _) = get_json(state.clone(), "/readyz").await;
    assert_eq!(code, 503);
    assert_eq!(body["ready"], false);
    assert_eq!(body["database"]["status"], "unavailable");

    let (code, body, _) = get_json(state, "/api/v1/auth/state").await;
    assert_eq!(code, 503);
    assert_eq!(body["error"]["code"], "database_unavailable");
}

#[tokio::test]
async fn cross_origin_writes_are_rejected_before_handlers() {
    let res = app(AppState::default(), None)
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("host", "nas.lan:8090")
                .header("origin", "https://evil.example")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"a","password":"b"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 403);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "origin_rejected");
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
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><title>AKL</title><script>boot()</script>",
    )
    .unwrap();

    let router = app(AppState::default(), Some(&dir));
    let res = router
        .clone()
        .oneshot(Request::get("/cameras").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status().as_u16(), 200);
    let csp = res.headers()["content-security-policy"].to_str().unwrap();
    // AKL-M020-07: Inline-Skripte nur per Hash, keine Inline-Styles.
    assert!(!csp.contains("unsafe-inline"), "{csp}");
    assert!(csp.contains("script-src 'self' 'sha256-"), "{csp}");
    assert!(csp.contains("style-src 'self';"), "{csp}");

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
