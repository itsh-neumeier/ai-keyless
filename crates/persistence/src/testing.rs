//! Testhilfen: frisches, isoliertes Schema pro Test gegen eine echte PostgreSQL.
//!
//! Schemanamen stammen ausschließlich aus Testkonstanten; daher ist
//! `AssertSqlSafe` hier zulässig.

use crate::Database;

/// `AKL_TEST_DATABASE_URL` oder `None` (Test überspringen). Mit
/// `AKL_REQUIRE_DB_TESTS=1` ist ein Überspringen ein Fehler.
pub fn test_database_url() -> Option<String> {
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

pub async fn fresh_schema(url: &str, schema: &str) -> Database {
    assert!(
        schema.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
        "Schemaname nur aus a-z und _"
    );
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
    let schema = schema.to_owned();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |conn, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .unwrap();
    let db = Database::from_pool(pool);
    db.migrate().await.expect("Migration");
    db
}
