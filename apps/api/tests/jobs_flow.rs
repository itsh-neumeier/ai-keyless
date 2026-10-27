//! Queue + Worker Ende-zu-Ende: API → persistente Queue → Worker → Provider (Mock).

mod common;

use std::time::Duration;

use akl_api::{
    AppState,
    worker::{Tick, Worker},
};
use akl_persistence::testing::{fresh_schema, test_database_url};
use axum::{
    Json, Router,
    routing::{get, post},
};
use common::{Client, setup_admin};
use serde_json::{Value, json};

async fn mock_ollama() -> String {
    let router = Router::new()
        .route(
            "/api/tags",
            get(|| async { Json(json!({ "models": [{ "name": "m" }] })) }),
        )
        .route(
            "/api/chat",
            post(|Json(_): Json<Value>| async { Json(json!({ "message": { "content": "red" } })) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn probe_jobs_run_through_queue_with_limit_and_retry() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_jobs").await;

    let mut state = AppState::new(Some(db.clone()), false);
    state.provider_policy.allow_loopback = true;
    state.provider_timeout = Duration::from_secs(3);
    state.jobs_per_hour = 2;
    let worker = Worker::new(state.clone(), db.clone());
    let mut admin = Client::with_state(state);
    setup_admin(&db, &mut admin).await;

    let base = mock_ollama().await;
    let ok_id = admin
        .send(
            "POST",
            "/api/v1/providers",
            Some(json!({"name": "ok", "kind": "ollama", "base_url": base, "default_model": "m"})),
        )
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let dead_id = admin
        .send("POST", "/api/v1/providers", Some(json!({"name": "tot", "kind": "ollama", "base_url": "http://127.0.0.1:9", "default_model": "m"})))
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Leere Queue.
    assert_eq!(worker.tick().await.unwrap(), Tick::Empty);

    // Erfolgreicher Job.
    let r = admin
        .send(
            "POST",
            &format!("/api/v1/providers/{ok_id}/probe-job"),
            Some(json!({})),
        )
        .await;
    assert_eq!(r.status, 202, "{}", r.body);
    assert_eq!(r.body["state"], "queued");
    let job_id = r.body["id"].as_str().unwrap().to_owned();
    assert_eq!(worker.tick().await.unwrap(), Tick::Processed);
    let jobs = admin.send("GET", "/api/v1/jobs", None).await.body;
    let job = jobs
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == job_id.as_str())
        .unwrap()
        .clone();
    assert_eq!(job["state"], "completed");
    assert_eq!(job["result"]["matched"], true);

    // Fehlschlagender Job: Verbindungsfehler → retryable mit Backoff.
    let r = admin
        .send(
            "POST",
            &format!("/api/v1/providers/{dead_id}/probe-job"),
            Some(json!({})),
        )
        .await;
    let dead_job = r.body["id"].as_str().unwrap().to_owned();
    assert_eq!(worker.tick().await.unwrap(), Tick::Processed);
    let r = admin
        .send("GET", "/api/v1/jobs?state=retryable", None)
        .await;
    assert_eq!(r.body[0]["id"], dead_job.as_str());
    assert_eq!(r.body[0]["last_error_code"], "connect_failed");

    // Limit (2/h) ist erreicht: weiterer Job bleibt eingereiht.
    admin
        .send(
            "POST",
            &format!("/api/v1/providers/{ok_id}/probe-job"),
            Some(json!({})),
        )
        .await;
    assert_eq!(worker.tick().await.unwrap(), Tick::RateLimited);
    let stats = admin.send("GET", "/api/v1/jobs/stats", None).await.body;
    assert_eq!(stats["starts_last_hour"], 2);
    assert_eq!(stats["limit_per_hour"], 2);
    assert_eq!(stats["counts"]["queued"], 1);
    assert_eq!(stats["counts"]["completed"], 1);
    assert_eq!(stats["counts"]["uncertain"], 0);

    // Operator-Aktionen: Abbrechen erlaubt, Neu-Einreihen nur aus failed/uncertain.
    assert_eq!(
        admin
            .send("POST", &format!("/api/v1/jobs/{dead_job}/requeue"), None)
            .await
            .status,
        409
    );
    assert_eq!(
        admin
            .send("POST", &format!("/api/v1/jobs/{dead_job}/cancel"), None)
            .await
            .status,
        204
    );
    assert_eq!(
        admin
            .send("POST", &format!("/api/v1/jobs/{job_id}/cancel"), None)
            .await
            .status,
        409
    );
    let unknown = uuid_like();
    assert_eq!(
        admin
            .send("POST", &format!("/api/v1/jobs/{unknown}/cancel"), None)
            .await
            .status,
        404
    );
    assert_eq!(
        admin
            .send("GET", "/api/v1/jobs?state=quatsch", None)
            .await
            .status,
        422
    );

    // Viewer darf lesen, aber nicht eingreifen.
    admin
        .send(
            "POST",
            "/api/v1/users",
            Some(json!({"username": "leser", "password": "ein-langes-passwort", "role": "viewer"})),
        )
        .await;
    let mut viewer = Client::new(db.clone());
    let r = viewer
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "leser", "password": "ein-langes-passwort"})),
        )
        .await;
    viewer.adopt(&r);
    assert_eq!(
        viewer.send("GET", "/api/v1/jobs/stats", None).await.status,
        200
    );
    assert_eq!(
        viewer
            .send("POST", &format!("/api/v1/jobs/{job_id}/cancel"), None)
            .await
            .status,
        403
    );
    assert_eq!(
        viewer
            .send(
                "POST",
                &format!("/api/v1/providers/{ok_id}/probe-job"),
                Some(json!({}))
            )
            .await
            .status,
        403
    );
}

fn uuid_like() -> String {
    "00000000-0000-4000-8000-000000000000".to_owned()
}
