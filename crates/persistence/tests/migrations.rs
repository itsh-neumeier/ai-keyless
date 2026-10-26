//! Migrationstest gegen eine echte PostgreSQL-Instanz.
//!
//! Schemanamen sind Testkonstanten, daher ist `AssertSqlSafe` hier zulässig.
//!
//! Benötigt `AKL_TEST_DATABASE_URL` (z. B. via `scripts/dev-db.sh`).
//! Ohne Variable wird der Test übersprungen; in der CI ist sie gesetzt und
//! `AKL_REQUIRE_DB_TESTS=1` macht ein Überspringen zum Fehler.

use akl_core::{DeviceProfileKind, DeviceState};
use akl_persistence::Database;

fn test_url() -> Option<String> {
    match std::env::var("AKL_TEST_DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) => {
            assert!(
                std::env::var("AKL_REQUIRE_DB_TESTS").is_err(),
                "AKL_REQUIRE_DB_TESTS gesetzt, aber AKL_TEST_DATABASE_URL fehlt"
            );
            eprintln!("übersprungen: AKL_TEST_DATABASE_URL nicht gesetzt");
            None
        }
    }
}

/// Frisches Schema pro Test, damit Tests unabhängig und wiederholbar sind.
async fn fresh_database(url: &str, schema: &str) -> Database {
    let admin = sqlx::PgPool::connect(url).await.expect("Verbindung");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP SCHEMA IF EXISTS {schema} CASCADE"
    )))
    .execute(&admin)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .after_connect({
            let schema = schema.to_owned();
            move |conn, _| {
                let schema = schema.clone();
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            }
        })
        .connect(url)
        .await
        .unwrap();
    Database::from_pool(pool)
}

#[tokio::test]
async fn migrations_apply_and_are_idempotent() {
    let Some(url) = test_url() else { return };
    let db = fresh_database(&url, "akl_test_migrate").await;
    db.migrate().await.expect("erste Migration");
    db.migrate().await.expect("zweite Migration ist No-Op");
    db.ping().await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), Some(20261026000001));
    assert!(db.device_states().await.unwrap().is_empty());
}

#[tokio::test]
async fn schema_enforces_profile_rules() {
    let Some(url) = test_url() else { return };
    let db = fresh_database(&url, "akl_test_rules").await;
    db.migrate().await.unwrap();

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
    assert!(
        sqlx::query(insert)
            .bind("ai_port")
            .bind("ready")
            .execute(db.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query(insert)
            .bind("ai_cam")
            .bind("unconfigured")
            .execute(db.pool())
            .await
            .is_err()
    );

    assert_eq!(
        db.device_states().await.unwrap(),
        vec![(DeviceProfileKind::AiKey, DeviceState::Discovered)]
    );
}
