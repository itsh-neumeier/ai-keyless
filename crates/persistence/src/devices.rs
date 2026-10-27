//! Repository für Geräteprofile, Zustandsübergänge und Kill-Switch (AKL-M040-02).
//!
//! Jeder Übergang läuft in einer Transaktion: Profilzeile sperren, gegen den
//! Zustandsautomaten aus `akl-core` prüfen, Zustand schreiben und den Übergang
//! protokollieren. Ist der Kill-Switch aktiv, ist nur noch `→ disabled` erlaubt.

use akl_core::{DeviceProfileKind, DeviceState, InvalidTransition};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{Database, PersistenceError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceProfileRecord {
    pub id: Uuid,
    pub kind: DeviceProfileKind,
    pub state: DeviceState,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransitionRecord {
    pub id: i64,
    pub from_state: String,
    pub to_state: String,
    pub reason: Option<String>,
    pub actor: String,
    pub correlation_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KillSwitchRecord {
    pub active: bool,
    pub changed_at: Option<DateTime<Utc>>,
    pub changed_by: Option<String>,
}

/// Ein angeforderter Übergang.
#[derive(Debug, Clone)]
pub struct TransitionRequest<'a> {
    pub to: DeviceState,
    /// Optimistische Prüfung: nur übergehen, wenn das Profil noch in diesem Zustand ist.
    pub expected_from: Option<DeviceState>,
    pub reason: &'a str,
    pub actor: &'a str,
    pub correlation_id: Option<&'a str>,
}

#[derive(Debug, thiserror::Error)]
pub enum TransitionError {
    #[error("Geräteprofil nicht gefunden")]
    NotFound,
    #[error(transparent)]
    Invalid(#[from] InvalidTransition),
    #[error("Zustand hat sich geändert: erwartet {expected:?}, aktuell {actual:?}")]
    Stale {
        expected: DeviceState,
        actual: DeviceState,
    },
    #[error("Kill-Switch aktiv: nur Deaktivieren ist erlaubt")]
    KillSwitch,
    #[error("es gibt bereits ein aktives ai_key-Profil")]
    ActiveAiKeyExists,
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

impl From<sqlx::Error> for TransitionError {
    fn from(err: sqlx::Error) -> Self {
        Self::Persistence(err.into())
    }
}

type ProfileRow = (Uuid, String, String, String, DateTime<Utc>, DateTime<Utc>);

const PROFILE_COLUMNS: &str = "id, kind, state, display_name, created_at, updated_at";

fn parse<T: std::str::FromStr<Err = akl_core::device::UnknownValue>>(
    s: &str,
) -> Result<T, PersistenceError> {
    s.parse()
        .map_err(|e: akl_core::device::UnknownValue| PersistenceError::InvalidValue(e.0))
}

fn profile_from_row(r: ProfileRow) -> Result<DeviceProfileRecord, PersistenceError> {
    Ok(DeviceProfileRecord {
        id: r.0,
        kind: parse(&r.1)?,
        state: parse(&r.2)?,
        display_name: r.3,
        created_at: r.4,
        updated_at: r.5,
    })
}

impl Database {
    /// Legt ein Profil im Zustand `unconfigured` an. `None`, wenn bereits ein
    /// aktives `ai_key`-Profil existiert (ADR-003).
    pub async fn create_device_profile(
        &self,
        id: Uuid,
        kind: DeviceProfileKind,
        display_name: &str,
    ) -> Result<Option<DeviceProfileRecord>, PersistenceError> {
        let result: Result<ProfileRow, sqlx::Error> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "INSERT INTO device_profiles (id, kind, display_name) VALUES ($1, $2, $3) \
                 RETURNING {PROFILE_COLUMNS}"
        )))
        .bind(id)
        .bind(kind.as_str())
        .bind(display_name)
        .fetch_one(self.pool())
        .await;
        match result {
            Ok(row) => profile_from_row(row).map(Some),
            Err(err) if crate::auth::is_unique_violation(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn list_device_profiles(&self) -> Result<Vec<DeviceProfileRecord>, PersistenceError> {
        let rows: Vec<ProfileRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {PROFILE_COLUMNS} FROM device_profiles ORDER BY created_at, id"
        )))
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(profile_from_row).collect()
    }

    pub async fn get_device_profile(
        &self,
        id: Uuid,
    ) -> Result<Option<DeviceProfileRecord>, PersistenceError> {
        let row: Option<ProfileRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {PROFILE_COLUMNS} FROM device_profiles WHERE id = $1"
        )))
        .bind(id)
        .fetch_optional(self.pool())
        .await?;
        row.map(profile_from_row).transpose()
    }

    /// Führt einen geprüften Übergang aus und protokolliert ihn.
    pub async fn transition_device(
        &self,
        id: Uuid,
        req: TransitionRequest<'_>,
    ) -> Result<DeviceProfileRecord, TransitionError> {
        let mut tx = self.pool().begin().await?;
        // Kill-Switch zuerst sperren: ein gleichzeitiges Einschalten wartet
        // auf diesen Übergang bzw. dieser Übergang sieht den neuen Wert.
        let kill: bool =
            sqlx::query_scalar("SELECT kill_switch FROM device_settings WHERE id FOR SHARE")
                .fetch_one(&mut *tx)
                .await?;
        let current: Option<String> =
            sqlx::query_scalar("SELECT state FROM device_profiles WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let current: DeviceState = parse(&current.ok_or(TransitionError::NotFound)?)?;
        if kill && req.to != DeviceState::Disabled {
            return Err(TransitionError::KillSwitch);
        }
        if let Some(expected) = req.expected_from
            && expected != current
        {
            return Err(TransitionError::Stale {
                expected,
                actual: current,
            });
        }
        let next = current.transition(req.to)?;

        let updated: Result<ProfileRow, sqlx::Error> =
            sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "UPDATE device_profiles SET state = $2, updated_at = now() WHERE id = $1 \
                 RETURNING {PROFILE_COLUMNS}"
            )))
            .bind(id)
            .bind(next.as_str())
            .fetch_one(&mut *tx)
            .await;
        let updated = match updated {
            Ok(row) => row,
            // Reaktivieren eines ai_key-Profils, während ein anderes aktiv ist.
            Err(err) if crate::auth::is_unique_violation(&err) => {
                return Err(TransitionError::ActiveAiKeyExists);
            }
            Err(err) => return Err(err.into()),
        };
        log_transition(&mut tx, id, current, next, &req).await?;
        tx.commit().await?;
        Ok(profile_from_row(updated)?)
    }

    /// Letzte Übergänge eines Profils, neueste zuerst.
    pub async fn device_transitions(
        &self,
        id: Uuid,
        limit: i64,
    ) -> Result<Vec<TransitionRecord>, PersistenceError> {
        type Row = (
            i64,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            DateTime<Utc>,
        );
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, from_state, to_state, reason, actor, correlation_id, occurred_at \
             FROM device_state_transitions WHERE profile_id = $1 \
             ORDER BY occurred_at DESC, id DESC LIMIT $2",
        )
        .bind(id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| TransitionRecord {
                id: r.0,
                from_state: r.1,
                to_state: r.2,
                reason: r.3,
                actor: r.4,
                correlation_id: r.5,
                occurred_at: r.6,
            })
            .collect())
    }

    pub async fn kill_switch(&self) -> Result<KillSwitchRecord, PersistenceError> {
        let (active, changed_at, changed_by): (bool, Option<DateTime<Utc>>, Option<String>) =
            sqlx::query_as(
                "SELECT kill_switch, kill_switch_changed_at, kill_switch_changed_by \
                 FROM device_settings WHERE id",
            )
            .fetch_one(self.pool())
            .await?;
        Ok(KillSwitchRecord {
            active,
            changed_at,
            changed_by,
        })
    }

    /// Schaltet den Kill-Switch. Beim Einschalten werden in derselben
    /// Transaktion alle nicht deaktivierten Profile deaktiviert (protokolliert).
    /// Liefert die dabei deaktivierten Profile.
    pub async fn set_kill_switch(
        &self,
        active: bool,
        actor: &str,
        correlation_id: Option<&str>,
    ) -> Result<Vec<DeviceProfileRecord>, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query(
            "UPDATE device_settings SET kill_switch = $1, kill_switch_changed_at = now(), \
             kill_switch_changed_by = $2 WHERE id",
        )
        .bind(active)
        .bind(actor)
        .execute(&mut *tx)
        .await?;
        let mut disabled = Vec::new();
        if active {
            let rows: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT id, state FROM device_profiles WHERE state <> 'disabled' \
                 ORDER BY created_at, id FOR UPDATE",
            )
            .fetch_all(&mut *tx)
            .await?;
            let req = TransitionRequest {
                to: DeviceState::Disabled,
                expected_from: None,
                reason: "kill_switch",
                actor,
                correlation_id,
            };
            for (id, state) in rows {
                let from: DeviceState = parse(&state)?;
                let row: ProfileRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                    "UPDATE device_profiles SET state = 'disabled', updated_at = now() \
                     WHERE id = $1 RETURNING {PROFILE_COLUMNS}"
                )))
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
                log_transition(&mut tx, id, from, DeviceState::Disabled, &req).await?;
                disabled.push(profile_from_row(row)?);
            }
        }
        tx.commit().await?;
        Ok(disabled)
    }
}

async fn log_transition(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    from: DeviceState,
    to: DeviceState,
    req: &TransitionRequest<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO device_state_transitions \
         (profile_id, from_state, to_state, reason, actor, correlation_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(from.as_str())
    .bind(to.as_str())
    .bind(req.reason)
    .bind(req.actor)
    .bind(req.correlation_id)
    .execute(tx)
    .await?;
    Ok(())
}
