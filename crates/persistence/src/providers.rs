//! Repository für Modell-Provider und deren Testergebnisse.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{Database, PersistenceError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderRecord {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub api_key_secret: Option<String>,
    pub default_model: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderTestRecord {
    pub kind: String,
    pub ok: bool,
    pub latency_ms: Option<i64>,
    pub error_code: Option<String>,
    pub model: Option<String>,
    pub result_text: Option<String>,
    pub matched: Option<bool>,
    pub tested_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewProvider<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub base_url: &'a str,
    pub api_key_secret: Option<&'a str>,
    pub default_model: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct NewProviderTest<'a> {
    pub kind: &'a str,
    pub ok: bool,
    pub latency_ms: Option<i64>,
    pub error_code: Option<&'a str>,
    pub model: Option<&'a str>,
    pub result_text: Option<&'a str>,
    pub matched: Option<bool>,
}

type ProviderRow = (
    Uuid,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    bool,
    DateTime<Utc>,
);
type TestRow = (
    String,
    bool,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<bool>,
    DateTime<Utc>,
);

const PROVIDER_COLUMNS: &str =
    "id, name, kind, base_url, api_key_secret, default_model, enabled, created_at";

fn provider_from_row(r: ProviderRow) -> ProviderRecord {
    ProviderRecord {
        id: r.0,
        name: r.1,
        kind: r.2,
        base_url: r.3,
        api_key_secret: r.4,
        default_model: r.5,
        enabled: r.6,
        created_at: r.7,
    }
}

impl Database {
    /// `None`, falls der Name bereits vergeben ist.
    pub async fn create_provider(
        &self,
        p: NewProvider<'_>,
    ) -> Result<Option<ProviderRecord>, PersistenceError> {
        let result: Result<ProviderRow, sqlx::Error> =
            sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "INSERT INTO providers (id, name, kind, base_url, api_key_secret, default_model) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING {PROVIDER_COLUMNS}"
            )))
            .bind(Uuid::new_v4())
            .bind(p.name)
            .bind(p.kind)
            .bind(p.base_url)
            .bind(p.api_key_secret)
            .bind(p.default_model)
            .fetch_one(self.pool())
            .await;
        match result {
            Ok(row) => Ok(Some(provider_from_row(row))),
            Err(err) if crate::auth::is_unique_violation(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn list_providers(&self) -> Result<Vec<ProviderRecord>, PersistenceError> {
        let rows: Vec<ProviderRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {PROVIDER_COLUMNS} FROM providers ORDER BY name"
        )))
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(provider_from_row).collect())
    }

    pub async fn get_provider(&self, id: Uuid) -> Result<Option<ProviderRecord>, PersistenceError> {
        let row: Option<ProviderRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {PROVIDER_COLUMNS} FROM providers WHERE id = $1"
        )))
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.map(provider_from_row))
    }

    pub async fn delete_provider(&self, id: Uuid) -> Result<bool, PersistenceError> {
        Ok(sqlx::query("DELETE FROM providers WHERE id = $1")
            .bind(id)
            .execute(self.pool())
            .await?
            .rows_affected()
            == 1)
    }

    pub async fn record_provider_test(
        &self,
        provider_id: Uuid,
        t: NewProviderTest<'_>,
    ) -> Result<(), PersistenceError> {
        // Antwort hart auf 500 Zeichen kürzen (CHECK-Constraint).
        let text = t
            .result_text
            .map(|s| s.chars().take(500).collect::<String>());
        sqlx::query(
            "INSERT INTO provider_tests (provider_id, kind, ok, latency_ms, error_code, model, result_text, matched) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(provider_id)
        .bind(t.kind)
        .bind(t.ok)
        .bind(t.latency_ms)
        .bind(t.error_code)
        .bind(t.model)
        .bind(text)
        .bind(t.matched)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Letztes Ergebnis je Testart für einen Provider.
    pub async fn latest_provider_tests(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<ProviderTestRecord>, PersistenceError> {
        let rows: Vec<TestRow> = sqlx::query_as(
            "SELECT DISTINCT ON (kind) kind, ok, latency_ms, error_code, model, result_text, matched, tested_at \
             FROM provider_tests WHERE provider_id = $1 ORDER BY kind, tested_at DESC, id DESC",
        )
        .bind(provider_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ProviderTestRecord {
                kind: r.0,
                ok: r.1,
                latency_ms: r.2,
                error_code: r.3,
                model: r.4,
                result_text: r.5,
                matched: r.6,
                tested_at: r.7,
            })
            .collect())
    }
}
