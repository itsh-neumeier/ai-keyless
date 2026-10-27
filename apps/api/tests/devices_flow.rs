//! Geräteverwaltung über die HTTP-API gegen eine echte PostgreSQL.

mod common;

use akl_api::AppState;
use akl_device_lifecycle::{identity::IdentityStore, service::Lifecycle};
use akl_persistence::testing::{fresh_schema, test_database_url};
use common::{ADMIN_PW, Client, setup_admin};
use serde_json::json;

#[tokio::test]
async fn device_profiles_kill_switch_and_audit() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_devices").await;
    let dir = std::env::temp_dir().join(format!("akl-devices-{}", uuid::Uuid::new_v4()));

    // Ohne Geräteverwaltung: ehrlich 503 statt leerer Liste.
    let mut bare = Client::new(db.clone());
    setup_admin(&db, &mut bare).await;
    let r = bare.send("GET", "/api/v1/devices", None).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (503, Some("lifecycle_unavailable"))
    );

    let mut state = AppState::new(Some(db.clone()), false);
    state.lifecycle = Some(Lifecycle::new(
        db.clone(),
        IdentityStore::open(&dir).unwrap(),
        false,
    ));
    let mut anon = Client::with_state(state.clone());
    let mut admin = Client::with_state(state);
    // Konto existiert bereits (setup_admin oben); neu anmelden.
    let r = admin
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "Admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 200, "{}", r.body);
    admin.adopt(&r);

    assert_eq!(anon.send("GET", "/api/v1/devices", None).await.status, 401);

    let r = admin
        .send(
            "POST",
            "/api/v1/devices",
            Some(json!({"kind": "ai_key", "display_name": "  "})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "invalid_name");

    let r = admin
        .send(
            "POST",
            "/api/v1/devices",
            Some(json!({"kind": "ai_key", "display_name": "Labor-Key"})),
        )
        .await;
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(r.body["state"], "unconfigured");
    let fingerprint = r.body["fingerprint"].as_str().unwrap().to_owned();
    assert_eq!(fingerprint.len(), 95, "SHA-256 als AB:CD:…");
    let id = r.body["id"].as_str().unwrap().to_owned();
    // Kein privater Schlüssel in der Antwort.
    assert!(!r.body.to_string().contains("PRIVATE"));

    let r = admin
        .send(
            "POST",
            "/api/v1/devices",
            Some(json!({"kind": "ai_key", "display_name": "Zweiter"})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (409, Some("ai_key_exists"))
    );

    // Reaktivieren eines aktiven Profils ist unzulässig (erwartet disabled).
    let r = admin
        .send("POST", &format!("/api/v1/devices/{id}/enable"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "state_changed");

    let r = admin
        .send(
            "POST",
            &format!("/api/v1/devices/{id}/disable"),
            Some(json!({"reason": "Wartung"})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["state"].as_str()),
        (200, Some("disabled"))
    );
    let r = admin
        .send("POST", &format!("/api/v1/devices/{id}/disable"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "invalid_transition");
    let r = admin
        .send("POST", &format!("/api/v1/devices/{id}/enable"), None)
        .await;
    assert_eq!(r.body["state"], "unconfigured");

    // Kill-Switch: deaktiviert alles, blockiert Anlage und Reaktivierung.
    let r = admin
        .send(
            "PUT",
            "/api/v1/devices/kill-switch",
            Some(json!({"active": true})),
        )
        .await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(r.body["disabled_profiles"], 1);
    assert_eq!(r.body["kill_switch"]["active"], true);
    let r = admin
        .send(
            "POST",
            "/api/v1/devices",
            Some(json!({"kind": "ai_port", "display_name": "Port"})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "kill_switch_active");
    let r = admin
        .send("POST", &format!("/api/v1/devices/{id}/enable"), None)
        .await;
    assert_eq!(r.body["error"]["code"], "kill_switch_active");

    let r = admin.send("GET", "/api/v1/devices", None).await;
    assert_eq!(r.body["kill_switch"]["active"], true);
    assert_eq!(r.body["profiles"][0]["fingerprint"], fingerprint.as_str());

    let r = admin
        .send("GET", &format!("/api/v1/devices/{id}/transitions"), None)
        .await;
    let reasons: Vec<_> = r.body["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["reason"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        reasons,
        ["kill_switch", "manuell reaktiviert", "Wartung"],
        "neueste zuerst"
    );

    let actions: Vec<String> = db
        .audit_actions()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, a)| a)
        .filter(|a| a.starts_with("device."))
        .collect();
    assert_eq!(
        actions,
        [
            "device.created",
            "device.disabled",
            "device.enabled",
            "device.kill_switch_on"
        ]
    );
    std::fs::remove_dir_all(dir).unwrap();
}
