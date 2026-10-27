//! Verarbeitungs-Worker (AKL-M060-01/03): holt Jobs aus der persistenten Queue
//! unter Beachtung des globalen Stundenlimits und führt sie aus.
//!
//! Bisher gibt es nur den Job-Typ `vision_probe` (Bildtest eines Providers mit
//! dem synthetischen Testbild). Protect-Ereignisse folgen mit AKL-M070.

use std::time::Duration;

use akl_persistence::{
    Database,
    jobs::{ClaimOutcome, JobRecord},
    providers::NewProviderTest,
};
use akl_providers::{
    ProviderError, SYNTHETIC_TEST_IMAGE_PNG, SYNTHETIC_TEST_PROMPT, synthetic_answer_matches,
};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, providers::build_provider};

pub const KIND_VISION_PROBE: &str = "vision_probe";
const IDLE_POLL: Duration = Duration::from_secs(10);
const RECOVERY_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    Processed,
    Empty,
    RateLimited,
}

pub struct Worker {
    state: AppState,
    db: Database,
    id: String,
}

impl Worker {
    pub fn new(state: AppState, db: Database) -> Self {
        let id = format!("worker-{}", &Uuid::new_v4().simple().to_string()[..12]);
        Self { state, db, id }
    }

    /// Lease-Dauer: länger als das Provider-Zeitlimit, damit ein laufender
    /// Aufruf nie als abgestürzt gilt.
    fn lease_seconds(&self) -> i64 {
        self.state.provider_timeout.as_secs() as i64 + 60
    }

    /// Verarbeitet höchstens einen Job.
    pub async fn tick(&self) -> Result<Tick, akl_persistence::PersistenceError> {
        match self
            .db
            .claim_job(&self.id, self.lease_seconds(), self.state.jobs_per_hour)
            .await?
        {
            ClaimOutcome::Empty => Ok(Tick::Empty),
            ClaimOutcome::RateLimited { .. } => Ok(Tick::RateLimited),
            ClaimOutcome::Claimed(job) => {
                self.process(*job).await?;
                Ok(Tick::Processed)
            }
        }
    }

    async fn process(&self, job: JobRecord) -> Result<(), akl_persistence::PersistenceError> {
        tracing::info!(job_id = %job.id, kind = %job.kind, attempt = job.attempts, "Job gestartet");
        match job.kind.as_str() {
            KIND_VISION_PROBE => self.vision_probe(job).await,
            other => {
                tracing::warn!(job_id = %job.id, kind = other, "unbekannter Job-Typ");
                self.db
                    .fail_job(job.id, &self.id, "unknown_kind", false)
                    .await?;
                Ok(())
            }
        }
    }

    async fn vision_probe(&self, job: JobRecord) -> Result<(), akl_persistence::PersistenceError> {
        let provider_id = job.payload["provider_id"]
            .as_str()
            .and_then(|s| s.parse::<Uuid>().ok());
        let model = job.payload["model"].as_str().map(str::to_owned);
        let (Some(provider_id), Some(model)) = (provider_id, model) else {
            self.db
                .fail_job(job.id, &self.id, "invalid_payload", false)
                .await?;
            return Ok(());
        };
        let Some(record) = self.db.get_provider(provider_id).await? else {
            self.db
                .fail_job(job.id, &self.id, "provider_missing", false)
                .await?;
            return Ok(());
        };
        let provider = match build_provider(&self.state, &record) {
            Ok(p) => p,
            Err(code) => {
                self.db.fail_job(job.id, &self.id, code, false).await?;
                return Ok(());
            }
        };
        match provider
            .describe_image(&model, SYNTHETIC_TEST_PROMPT, SYNTHETIC_TEST_IMAGE_PNG)
            .await
        {
            Ok(r) => {
                let matched = synthetic_answer_matches(&r.text);
                self.db
                    .record_provider_test(
                        provider_id,
                        NewProviderTest {
                            kind: "vision",
                            ok: true,
                            latency_ms: Some(r.latency_ms as i64),
                            error_code: None,
                            model: Some(&model),
                            result_text: Some(&r.text),
                            matched: Some(matched),
                        },
                    )
                    .await?;
                let text: String = r.text.chars().take(500).collect();
                self.db
                    .complete_job(
                        job.id,
                        &self.id,
                        &json!({ "model": model, "latency_ms": r.latency_ms, "text": text, "matched": matched }),
                    )
                    .await?;
            }
            Err(err) => {
                // Netzwerk-/Zeitfehler sind vorübergehend, alles andere nicht.
                let retryable = matches!(
                    err,
                    ProviderError::Timeout | ProviderError::Connect | ProviderError::Dns
                ) || matches!(err, ProviderError::HttpStatus(s) if s >= 500 || s == 429);
                self.db
                    .fail_job(job.id, &self.id, err.code(), retryable)
                    .await?;
            }
        }
        Ok(())
    }

    /// Endlosschleife: Jobs abarbeiten, bei Leerlauf/Limit warten oder auf
    /// ein Signal (neuer Job) reagieren, abgelaufene Leases wiederherstellen.
    pub async fn run(self) {
        let mut last_recovery = tokio::time::Instant::now() - RECOVERY_INTERVAL;
        loop {
            if last_recovery.elapsed() >= RECOVERY_INTERVAL {
                match self.db.recover_expired_jobs().await {
                    Ok(r) if r.retried + r.uncertain > 0 => {
                        tracing::warn!(
                            retried = r.retried,
                            uncertain = r.uncertain,
                            "abgelaufene Jobs wiederhergestellt"
                        );
                    }
                    Ok(_) => {}
                    Err(err) => tracing::debug!(error = %err, "Recovery übersprungen"),
                }
                last_recovery = tokio::time::Instant::now();
            }
            let tick = match self.tick().await {
                Ok(t) => t,
                Err(err) => {
                    tracing::debug!(error = %err, "Queue nicht verfügbar");
                    Tick::Empty
                }
            };
            if tick != Tick::Processed {
                tokio::select! {
                    _ = self.state.job_signal.notified() => {}
                    _ = tokio::time::sleep(IDLE_POLL) => {}
                }
            }
        }
    }
}
