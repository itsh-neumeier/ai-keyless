//! Provider-Verwaltung gegen echte PostgreSQL und einen Mock-Ollama auf Loopback.

mod common;

use std::time::Duration;

use akl_api::AppState;
use akl_persistence::testing::{fresh_schema, test_database_url};
use axum::{
    Json, Router,
    routing::{get, post},
};
use common::{Client, setup_admin};
use serde_json::{Value, json};

async fn mock_ollama(answer: &'static str) -> String {
    let router = Router::new()
        .route(
            "/api/tags",
            get(|| async { Json(json!({ "models": [{ "name": "vision:tiny" }] })) }),
        )
        .route(
            "/api/chat",
            post(move |Json(_): Json<Value>| async move {
                Json(json!({ "message": { "content": answer } }))
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn provider_lifecycle_with_measured_tests() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_providers").await;

    let secrets = std::env::temp_dir().join(format!("akl-secrets-{}", std::process::id()));
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("lm_key"), "geheimer-schluessel\n").unwrap();

    let mut state = AppState::new(Some(db.clone()), false);
    state.provider_policy.allow_loopback = true;
    state.provider_timeout = Duration::from_secs(3);
    state.secrets_dir = secrets.clone();
    let mut admin = Client::with_state(state);
    setup_admin(&db, &mut admin).await;

    let base = mock_ollama("Red").await;

    // Validierung: gesperrte Adresse, ungültige URL, fehlendes Secret, Pfad im Secret-Namen.
    for (body, code) in [
        (
            json!({"name": "x", "kind": "ollama", "base_url": "http://169.254.169.254"}),
            "blocked_address",
        ),
        (
            json!({"name": "x", "kind": "ollama", "base_url": "file:///etc/passwd"}),
            "invalid_url",
        ),
        (
            json!({"name": "x", "kind": "ollama", "base_url": base, "api_key_secret": "fehlt"}),
            "secret_missing",
        ),
        (
            json!({"name": "x", "kind": "ollama", "base_url": base, "api_key_secret": "../x"}),
            "invalid_secret_name",
        ),
        (
            json!({"name": "", "kind": "ollama", "base_url": base}),
            "invalid_name",
        ),
    ] {
        let r = admin.send("POST", "/api/v1/providers", Some(body)).await;
        assert_eq!(
            (r.status, r.body["error"]["code"].as_str()),
            (422, Some(code)),
            "{}",
            r.body
        );
    }
    let r = admin
        .send(
            "POST",
            "/api/v1/providers",
            Some(json!({"name": "x", "kind": "unbekannt", "base_url": base})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "invalid_request");

    // Anlegen (Secret-Name wird gespeichert, nie der Inhalt).
    let r = admin
        .send(
            "POST",
            "/api/v1/providers",
            Some(json!({"name": "Ollama GPU", "kind": "ollama", "base_url": format!("{base}/"), "api_key_secret": "lm_key", "default_model": "vision:tiny"})),
        )
        .await;
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(
        r.body["base_url"], base,
        "abschließender Schrägstrich normalisiert"
    );
    let id = r.body["id"].as_str().unwrap().to_owned();
    let r = admin
        .send(
            "POST",
            "/api/v1/providers",
            Some(json!({"name": "Ollama GPU", "kind": "ollama", "base_url": base})),
        )
        .await;
    assert_eq!(r.status, 409);

    // Verbindungstest und Bildtest werden gemessen und gespeichert.
    let r = admin
        .send("POST", &format!("/api/v1/providers/{id}/test"), None)
        .await;
    assert_eq!(r.body["ok"], true, "{}", r.body);
    assert_eq!(r.body["models"], json!(["vision:tiny"]));
    let r = admin
        .send(
            "POST",
            &format!("/api/v1/providers/{id}/test-vision"),
            Some(json!({})),
        )
        .await;
    assert_eq!(r.body["ok"], true, "{}", r.body);
    assert_eq!(r.body["matched"], true);
    assert_eq!(r.body["model"], "vision:tiny");

    let r = admin.send("GET", "/api/v1/providers", None).await;
    let listed = &r.body[0];
    assert_eq!(listed["tests"].as_array().unwrap().len(), 2);
    let text = r.body.to_string();
    assert!(
        text.contains("lm_key") && !text.contains("geheimer-schluessel"),
        "Secret-Inhalt darf nie erscheinen"
    );

    // Nicht erreichbarer Provider: Testergebnis ok=false statt API-Fehler.
    let r = admin
        .send(
            "POST",
            "/api/v1/providers",
            Some(json!({"name": "tot", "kind": "ollama", "base_url": "http://127.0.0.1:9"})),
        )
        .await;
    let dead = r.body["id"].as_str().unwrap().to_owned();
    let r = admin
        .send("POST", &format!("/api/v1/providers/{dead}/test"), None)
        .await;
    assert_eq!(
        (
            r.status,
            r.body["ok"].as_bool(),
            r.body["error_code"].as_str()
        ),
        (200, Some(false), Some("connect_failed"))
    );
    let r = admin
        .send(
            "POST",
            &format!("/api/v1/providers/{dead}/test-vision"),
            Some(json!({})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "model_required");

    // Viewer: lesen ja, ändern/testen nein.
    let r = admin
        .send(
            "POST",
            "/api/v1/users",
            Some(json!({"username": "leser", "password": "ein-langes-passwort", "role": "viewer"})),
        )
        .await;
    assert_eq!(r.status, 201);
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
        viewer.send("GET", "/api/v1/providers", None).await.status,
        200
    );
    assert_eq!(
        viewer
            .send("POST", &format!("/api/v1/providers/{id}/test"), None)
            .await
            .status,
        403
    );
    assert_eq!(
        viewer
            .send("DELETE", &format!("/api/v1/providers/{id}"), None)
            .await
            .status,
        403
    );

    // Löschen entfernt auch die Testergebnisse.
    assert_eq!(
        admin
            .send("DELETE", &format!("/api/v1/providers/{id}"), None)
            .await
            .status,
        204
    );
    assert_eq!(
        admin
            .send("DELETE", &format!("/api/v1/providers/{id}"), None)
            .await
            .status,
        404
    );
    let actions: Vec<String> = db
        .audit_actions()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, a)| a)
        .collect();
    for expected in [
        "provider.created",
        "provider.vision_tested",
        "provider.deleted",
    ] {
        assert!(actions.iter().any(|a| a == expected), "{expected}");
    }
    let _ = std::fs::remove_dir_all(secrets);
}
