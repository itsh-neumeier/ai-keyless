//! PostgreSQL-Zugriff und versionierte SQLx-Migrationen.

pub mod auth;
pub mod providers;
#[cfg(feature = "testing")]
pub mod testing;

use std::time::Duration;

use akl_core::{DeviceProfileKind, DeviceState};
use sqlx::postgres::{PgPool, PgPoolOptions};

pub use sqlx::Error as SqlxError;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("Datenbankfehler: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migrationsfehler: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("ungültiger gespeicherter Wert: {0}")]
    InvalidValue(String),
}

#[derive(Clone, Debug)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Baut einen Pool auf, ohne sofort zu verbinden. So startet die API
    /// auch bei nicht erreichbarer Datenbank und meldet den Zustand ehrlich.
    pub fn connect_lazy(url: &str) -> Result<Self, PersistenceError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect_lazy(url)?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), PersistenceError> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    pub async fn ping(&self) -> Result<(), PersistenceError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    /// Höchste erfolgreich angewendete Migrationsversion.
    pub async fn schema_version(&self) -> Result<Option<i64>, PersistenceError> {
        let version: Option<i64> =
            sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
                .fetch_one(&self.pool)
                .await?;
        Ok(version)
    }

    /// Gespeicherte Zustände aller nicht deaktivierten Geräteprofile.
    pub async fn device_states(
        &self,
    ) -> Result<Vec<(DeviceProfileKind, DeviceState)>, PersistenceError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT kind, state FROM device_profiles WHERE state <> 'disabled' ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|(kind, state)| {
                let kind = kind.parse().map_err(|e: akl_core::device::UnknownValue| {
                    PersistenceError::InvalidValue(e.0)
                })?;
                let state = state.parse().map_err(|e: akl_core::device::UnknownValue| {
                    PersistenceError::InvalidValue(e.0)
                })?;
                Ok((kind, state))
            })
            .collect()
    }
}
