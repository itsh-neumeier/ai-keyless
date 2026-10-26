//! Migrations- und Repository-Tests gegen eine echte PostgreSQL-Instanz.
//!
//! Benötigt `AKL_TEST_DATABASE_URL` (z. B. via `scripts/dev-db.sh`).
//! Ohne Variable wird übersprungen; in der CI macht `AKL_REQUIRE_DB_TESTS=1`
//! ein Überspringen zum Fehler.

use akl_core::{DeviceProfileKind, DeviceState, Role};
use akl_persistence::{
    auth::{CreateUserOutcome, SetupOutcome},
    testing::{fresh_schema, test_database_url},
};
use chrono::{Duration, Utc};

const LATEST_MIGRATION: i64 = 20261027000001;

#[tokio::test]
async fn migrations_apply_and_are_idempotent() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_test_migrate").await;
    db.migrate().await.expect("zweite Migration ist No-Op");
    db.ping().await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), Some(LATEST_MIGRATION));
    assert!(db.device_states().await.unwrap().is_empty());
    assert_eq!(db.user_count().await.unwrap(), 0);
}

#[tokio::test]
async fn schema_enforces_profile_rules() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_test_rules").await;

    let insert = "INSERT INTO device_profiles (id, kind, state, display_name) \
                  VALUES (gen_random_uuid(), $1, $2, 'test')";
    sqlx::query(insert)
        .bind("ai_key")
        .bind("discovered")
        .execute(db.pool())
        .await
        .unwrap();

    // Zweites aktives AI-Key-Profil ist verboten.
    assert!(
        sqlx::query(insert)
            .bind("ai_key")
            .bind("unconfigured")
            .execute(db.pool())
            .await
            .is_err()
    );
    // Unbekannte Zustände und Profile werden abgewiesen.
    for (kind, state) in [("ai_port", "ready"), ("ai_cam", "unconfigured")] {
        assert!(
            sqlx::query(insert)
                .bind(kind)
                .bind(state)
                .execute(db.pool())
                .await
                .is_err(),
            "{kind}/{state}"
        );
    }

    assert_eq!(
        db.device_states().await.unwrap(),
        vec![(DeviceProfileKind::AiKey, DeviceState::Discovered)]
    );
}

#[tokio::test]
async fn setup_token_is_single_use_and_only_before_first_account() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_test_setup").await;
    let future = Utc::now() + Duration::minutes(30);

    // Unbekanntes Token.
    assert_eq!(
        db.complete_setup("nope", "admin", "hash").await.unwrap(),
        SetupOutcome::InvalidToken
    );

    // Abgelaufenes Token.
    db.replace_setup_token("expired", Utc::now() - Duration::minutes(1))
        .await
        .unwrap();
    assert_eq!(
        db.complete_setup("expired", "admin", "hash").await.unwrap(),
        SetupOutcome::InvalidToken
    );

    // Ein neues Token ersetzt alle offenen.
    db.replace_setup_token("first", future).await.unwrap();
    db.replace_setup_token("second", future).await.unwrap();
    assert_eq!(
        db.complete_setup("first", "admin", "hash").await.unwrap(),
        SetupOutcome::InvalidToken
    );

    let SetupOutcome::Completed(admin) =
        db.complete_setup("second", "admin", "hash").await.unwrap()
    else {
        panic!("Einrichtung erwartet");
    };
    assert_eq!(admin.role, Role::Admin);
    assert_eq!(db.user_count().await.unwrap(), 1);

    // Danach ist keine erneute Einrichtung möglich – auch nicht mit neuem Token.
    db.replace_setup_token("third", future).await.unwrap();
    assert_eq!(
        db.complete_setup("third", "evil", "hash").await.unwrap(),
        SetupOutcome::AlreadyCompleted
    );
}

#[tokio::test]
async fn users_sessions_and_audit() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_test_users").await;

    let CreateUserOutcome::Created(viewer) = db
        .create_user("viewer1", "hash-v", Role::Viewer)
        .await
        .unwrap()
    else {
        panic!("Konto erwartet");
    };
    assert_eq!(
        db.create_user("viewer1", "x", Role::Admin).await.unwrap(),
        CreateUserOutcome::UsernameTaken
    );
    // CHECK-Constraint greift auch bei Umgehung der Validierung.
    assert!(db.create_user("Bad Name", "x", Role::Viewer).await.is_err());

    let (found, hash) = db.find_credentials("viewer1").await.unwrap().unwrap();
    assert_eq!(found.id, viewer.id);
    assert_eq!(hash, "hash-v");
    assert!(db.find_credentials("unknown").await.unwrap().is_none());

    let long_ago = Utc::now() - Duration::hours(12);
    db.create_session("tok", viewer.id, "csrf", Utc::now() + Duration::days(1))
        .await
        .unwrap();
    let session = db.find_session("tok", long_ago).await.unwrap().unwrap();
    assert_eq!(session.csrf_token, "csrf");
    assert_eq!(session.user.username, "viewer1");

    // Inaktivitätsgrenze in der Zukunft → Sitzung gilt als abgelaufen.
    assert!(
        db.find_session("tok", Utc::now() + Duration::seconds(5))
            .await
            .unwrap()
            .is_none()
    );

    // Abgelaufene Sitzungen werden nicht gefunden und aufgeräumt.
    db.create_session("old", viewer.id, "c", Utc::now() - Duration::seconds(1))
        .await
        .unwrap();
    assert!(db.find_session("old", long_ago).await.unwrap().is_none());
    assert_eq!(db.delete_expired_sessions().await.unwrap(), 1);

    // Deaktivierte Konten verlieren ihre Sitzungen sofort.
    sqlx::query("UPDATE users SET disabled = true WHERE id = $1")
        .bind(viewer.id)
        .execute(db.pool())
        .await
        .unwrap();
    assert!(db.find_session("tok", long_ago).await.unwrap().is_none());

    db.delete_session("tok").await.unwrap();
    db.audit(
        "viewer1",
        "auth.login",
        None,
        serde_json::json!({}),
        Some("req-1"),
    )
    .await
    .unwrap();
    assert_eq!(
        db.audit_actions().await.unwrap(),
        vec![("viewer1".to_owned(), "auth.login".to_owned())]
    );
}
