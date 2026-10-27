//! Persistente Job-Queue mit Leases, Idempotenz, Backoff und dauerhaftem
//! Stundenlimit. Alle Übergänge sind atomar und prüfen den erwarteten
//! Vorzustand in SQL; ein veralteter Worker kann nichts überschreiben.

use akl_core::job::{JobState, backoff_seconds};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{Database, PersistenceError};

/// Fester Schlüssel für die Advisory-Lock-Serialisierung der Limitprüfung.
const CLAIM_LOCK_KEY: i64 = 0x414b_4c5f_4a4f_4253; // "AKL_JOBS"

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobRecord {
    pub id: Uuid,
    pub idempotency_key: String,
    pub kind: String,
    pub state: JobState,
    pub phase: String,
    pub priority: i16,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error_code: Option<String>,
    pub payload: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnqueueOutcome {
    Created(JobRecord),
    /// Gleicher Idempotenzschlüssel existiert bereits (Duplikat).
    Duplicate(JobRecord),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClaimOutcome {
    Claimed(Box<JobRecord>),
    /// Nichts zu tun.
    Empty,
    /// Stundenlimit erreicht; nächster Start frühestens dann.
    RateLimited {
        retry_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueueStats {
    pub counts: Vec<(String, i64)>,
    pub oldest_ready_at: Option<DateTime<Utc>>,
    pub starts_last_hour: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryReport {
    pub retried: u64,
    pub uncertain: u64,
}

const JOB_COLUMNS: &str = "id, idempotency_key, kind, state, phase, priority, attempts, max_attempts, \
     next_attempt_at, last_error_code, payload, result, created_at, started_at, finished_at";

#[derive(sqlx::FromRow)]
struct JobRow {
    id: Uuid,
    idempotency_key: String,
    kind: String,
    state: String,
    phase: String,
    priority: i16,
    attempts: i32,
    max_attempts: i32,
    next_attempt_at: DateTime<Utc>,
    last_error_code: Option<String>,
    payload: serde_json::Value,
    result: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

impl TryFrom<JobRow> for JobRecord {
    type Error = PersistenceError;

    fn try_from(r: JobRow) -> Result<Self, PersistenceError> {
        Ok(Self {
            id: r.id,
            idempotency_key: r.idempotency_key,
            kind: r.kind,
            state: r
                .state
                .parse()
                .map_err(|e: akl_core::device::UnknownValue| PersistenceError::InvalidValue(e.0))?,
            phase: r.phase,
            priority: r.priority,
            attempts: r.attempts,
            max_attempts: r.max_attempts,
            next_attempt_at: r.next_attempt_at,
            last_error_code: r.last_error_code,
            payload: r.payload,
            result: r.result,
            created_at: r.created_at,
            started_at: r.started_at,
            finished_at: r.finished_at,
        })
    }
}

fn sql(template: &str) -> sqlx::AssertSqlSafe<String> {
    // Nur die feste Spaltenliste wird eingesetzt, keine Benutzereingaben.
    sqlx::AssertSqlSafe(template.replace("{COLS}", JOB_COLUMNS))
}

impl Database {
    pub async fn enqueue_job(
        &self,
        idempotency_key: &str,
        kind: &str,
        payload: &serde_json::Value,
        priority: i16,
        max_attempts: i32,
    ) -> Result<EnqueueOutcome, PersistenceError> {
        let inserted: Option<JobRow> = sqlx::query_as(sql(
            "INSERT INTO jobs (id, idempotency_key, kind, payload, priority, max_attempts) \
             VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (idempotency_key) DO NOTHING RETURNING {COLS}",
        ))
        .bind(Uuid::new_v4())
        .bind(idempotency_key)
        .bind(kind)
        .bind(payload)
        .bind(priority)
        .bind(max_attempts)
        .fetch_optional(self.pool())
        .await?;
        if let Some(row) = inserted {
            return Ok(EnqueueOutcome::Created(row.try_into()?));
        }
        let existing: JobRow =
            sqlx::query_as(sql("SELECT {COLS} FROM jobs WHERE idempotency_key = $1"))
                .bind(idempotency_key)
                .fetch_one(self.pool())
                .await?;
        Ok(EnqueueOutcome::Duplicate(existing.try_into()?))
    }

    /// Holt den nächsten fälligen Job (Priorität, dann Alter), sofern das
    /// Stundenlimit es zulässt. Limitprüfung und Start sind serialisiert.
    pub async fn claim_job(
        &self,
        worker_id: &str,
        lease_seconds: i64,
        max_starts_per_hour: i64,
    ) -> Result<ClaimOutcome, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(CLAIM_LOCK_KEY)
            .execute(&mut *tx)
            .await?;
        let (count, oldest): (i64, Option<DateTime<Utc>>) = sqlx::query_as(
            "SELECT count(*), min(started_at) FROM job_starts WHERE started_at > now() - interval '1 hour'",
        )
        .fetch_one(&mut *tx)
        .await?;
        if count >= max_starts_per_hour {
            // Das Limit ist rollierend: frei wird es, sobald der älteste Start
            // im Fenster eine Stunde alt ist.
            let retry_at = oldest
                .map(|t| t + chrono::Duration::hours(1))
                .unwrap_or_else(Utc::now);
            return Ok(ClaimOutcome::RateLimited { retry_at });
        }
        let row: Option<JobRow> = sqlx::query_as(sql(
            "UPDATE jobs SET state = 'running', phase = 'processing', attempts = attempts + 1, \
                 worker_id = $1, lease_until = now() + make_interval(secs => $2), \
                 started_at = coalesce(started_at, now()), updated_at = now() \
             WHERE id = (SELECT id FROM jobs WHERE state IN ('queued', 'retryable') AND next_attempt_at <= now() \
                         ORDER BY priority DESC, created_at FOR UPDATE SKIP LOCKED LIMIT 1) \
             RETURNING {COLS}",
        ))
        .bind(worker_id)
        .bind(lease_seconds as f64)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(ClaimOutcome::Empty);
        };
        sqlx::query("INSERT INTO job_starts (job_id) VALUES ($1)")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(ClaimOutcome::Claimed(Box::new(row.try_into()?)))
    }

    /// Vor einer Nebenwirkung nach außen (z. B. Callback an Protect) aufrufen.
    /// Bricht der Worker danach ab, wird der Job `uncertain` statt wiederholt.
    pub async fn mark_job_callback_phase(
        &self,
        id: Uuid,
        worker_id: &str,
    ) -> Result<bool, PersistenceError> {
        Ok(sqlx::query(
            "UPDATE jobs SET phase = 'callback', updated_at = now() \
             WHERE id = $1 AND state = 'running' AND worker_id = $2",
        )
        .bind(id)
        .bind(worker_id)
        .execute(self.pool())
        .await?
        .rows_affected()
            == 1)
    }

    pub async fn complete_job(
        &self,
        id: Uuid,
        worker_id: &str,
        result: &serde_json::Value,
    ) -> Result<bool, PersistenceError> {
        Ok(sqlx::query(
            "UPDATE jobs SET state = 'completed', result = $3, lease_until = NULL, last_error_code = NULL, \
                 finished_at = now(), updated_at = now() \
             WHERE id = $1 AND state = 'running' AND worker_id = $2",
        )
        .bind(id)
        .bind(worker_id)
        .bind(result)
        .execute(self.pool())
        .await?
        .rows_affected()
            == 1)
    }

    /// Fehlschlag: bei `retryable` und verbleibenden Versuchen mit Backoff
    /// erneut einreihen, sonst endgültig `failed` (Dead Letter).
    pub async fn fail_job(
        &self,
        id: Uuid,
        worker_id: &str,
        error_code: &str,
        retryable: bool,
    ) -> Result<Option<JobState>, PersistenceError> {
        let current: Option<(i32, i32)> = sqlx::query_as(
            "SELECT attempts, max_attempts FROM jobs WHERE id = $1 AND state = 'running' AND worker_id = $2",
        )
        .bind(id)
        .bind(worker_id)
        .fetch_optional(self.pool())
        .await?;
        let Some((attempts, max)) = current else {
            return Ok(None);
        };
        let next = if retryable && attempts < max {
            JobState::Retryable
        } else {
            JobState::Failed
        };
        let delay = backoff_seconds(attempts.max(1) as u32) as f64;
        let updated = sqlx::query(
            "UPDATE jobs SET state = $3, last_error_code = $4, lease_until = NULL, updated_at = now(), \
                 phase = 'pending', next_attempt_at = now() + make_interval(secs => $5), \
                 finished_at = CASE WHEN $3 = 'failed' THEN now() ELSE NULL END \
             WHERE id = $1 AND state = 'running' AND worker_id = $2",
        )
        .bind(id)
        .bind(worker_id)
        .bind(next.as_str())
        .bind(error_code)
        .bind(delay)
        .execute(self.pool())
        .await?
        .rows_affected();
        Ok((updated == 1).then_some(next))
    }

    pub async fn mark_job_uncertain(
        &self,
        id: Uuid,
        worker_id: &str,
        error_code: &str,
    ) -> Result<bool, PersistenceError> {
        Ok(sqlx::query(
            "UPDATE jobs SET state = 'uncertain', last_error_code = $3, lease_until = NULL, \
                 finished_at = now(), updated_at = now() \
             WHERE id = $1 AND state = 'running' AND worker_id = $2",
        )
        .bind(id)
        .bind(worker_id)
        .bind(error_code)
        .execute(self.pool())
        .await?
        .rows_affected()
            == 1)
    }

    /// Jobs mit abgelaufener Lease (Worker abgestürzt/neu gestartet):
    /// vor dem Callback → erneut versuchen (falls Versuche übrig), sonst `failed`;
    /// im Callback → `uncertain` (keine automatische Wiederholung).
    pub async fn recover_expired_jobs(&self) -> Result<RecoveryReport, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        let uncertain = sqlx::query(
            "UPDATE jobs SET state = 'uncertain', last_error_code = 'lease_expired_during_callback', \
                 lease_until = NULL, finished_at = now(), updated_at = now() \
             WHERE state = 'running' AND lease_until < now() AND phase = 'callback'",
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        let retried = sqlx::query(
            "UPDATE jobs SET state = CASE WHEN attempts < max_attempts THEN 'retryable' ELSE 'failed' END, \
                 finished_at = CASE WHEN attempts < max_attempts THEN NULL ELSE now() END, \
                 last_error_code = 'lease_expired', phase = 'pending', lease_until = NULL, \
                 next_attempt_at = now(), updated_at = now() \
             WHERE state = 'running' AND lease_until < now()",
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        tx.commit().await?;
        Ok(RecoveryReport { retried, uncertain })
    }

    /// Operator-Aktion: abbrechen (nur nicht laufende Jobs).
    pub async fn cancel_job(&self, id: Uuid) -> Result<bool, PersistenceError> {
        Ok(sqlx::query(
            "UPDATE jobs SET state = 'cancelled', lease_until = NULL, finished_at = now(), updated_at = now() \
             WHERE id = $1 AND state IN ('queued', 'retryable', 'uncertain', 'failed')",
        )
        .bind(id)
        .execute(self.pool())
        .await?
        .rows_affected()
            == 1)
    }

    /// Operator-Aktion: `failed`/`uncertain` nach Prüfung erneut einreihen
    /// (gibt genau einen weiteren Versuch frei).
    pub async fn requeue_job(&self, id: Uuid) -> Result<bool, PersistenceError> {
        Ok(sqlx::query(
            "UPDATE jobs SET state = 'queued', phase = 'pending', next_attempt_at = now(), \
                 max_attempts = greatest(max_attempts, attempts + 1), finished_at = NULL, updated_at = now() \
             WHERE id = $1 AND state IN ('failed', 'uncertain')",
        )
        .bind(id)
        .execute(self.pool())
        .await?
        .rows_affected()
            == 1)
    }

    pub async fn get_job(&self, id: Uuid) -> Result<Option<JobRecord>, PersistenceError> {
        let row: Option<JobRow> = sqlx::query_as(sql("SELECT {COLS} FROM jobs WHERE id = $1"))
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list_jobs(
        &self,
        state: Option<JobState>,
        limit: i64,
    ) -> Result<Vec<JobRecord>, PersistenceError> {
        let rows: Vec<JobRow> = sqlx::query_as(sql(
            "SELECT {COLS} FROM jobs WHERE ($1::text IS NULL OR state = $1) ORDER BY created_at DESC LIMIT $2",
        ))
        .bind(state.map(JobState::as_str))
        .bind(limit.clamp(1, 500))
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn queue_stats(&self) -> Result<QueueStats, PersistenceError> {
        let counts: Vec<(String, i64)> =
            sqlx::query_as("SELECT state, count(*) FROM jobs GROUP BY state ORDER BY state")
                .fetch_all(self.pool())
                .await?;
        let oldest_ready_at: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT min(created_at) FROM jobs WHERE state IN ('queued', 'retryable')",
        )
        .fetch_one(self.pool())
        .await?;
        let starts_last_hour: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM job_starts WHERE started_at > now() - interval '1 hour'",
        )
        .fetch_one(self.pool())
        .await?;
        Ok(QueueStats {
            counts,
            oldest_ready_at,
            starts_last_hour,
        })
    }
}
