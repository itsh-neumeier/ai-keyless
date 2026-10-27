//! Einsicht in die Verarbeitungs-Queue und Operator-Aktionen (AKL-M060-01/03).

use akl_core::{Role, job::JobState};
use akl_persistence::jobs::JobRecord;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, auth::CurrentUser, error::ApiError};

#[derive(Deserialize)]
pub struct ListQuery {
    state: Option<String>,
    limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<JobRecord>>, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let filter = match q.state.as_deref().filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(s.parse::<JobState>().map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_request",
                "unbekannter Zustand",
            )
        })?),
    };
    Ok(Json(db.list_jobs(filter, q.limit.unwrap_or(50)).await?))
}

#[derive(Serialize)]
pub struct Stats {
    /// Anzahl je Zustand; fehlende Zustände sind 0.
    counts: std::collections::BTreeMap<&'static str, i64>,
    oldest_ready_at: Option<DateTime<Utc>>,
    starts_last_hour: i64,
    limit_per_hour: i64,
}

pub async fn stats(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Stats>, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let raw = db.queue_stats().await?;
    let counts = JobState::ALL
        .into_iter()
        .map(|s| {
            let n = raw
                .counts
                .iter()
                .find(|(k, _)| k == s.as_str())
                .map_or(0, |(_, n)| *n);
            (s.as_str(), n)
        })
        .collect();
    Ok(Json(Stats {
        counts,
        oldest_ready_at: raw.oldest_ready_at,
        starts_last_hour: raw.starts_last_hour,
        limit_per_hour: state.jobs_per_hour,
    }))
}

async fn operator_action(
    state: &AppState,
    headers: &HeaderMap,
    user: &CurrentUser,
    id: Uuid,
    action: &'static str,
) -> Result<StatusCode, ApiError> {
    user.require(Role::Operator)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let changed = match action {
        "job.cancelled" => db.cancel_job(id).await?,
        _ => db.requeue_job(id).await?,
    };
    if !changed {
        return Err(if db.get_job(id).await?.is_none() {
            ApiError::new(StatusCode::NOT_FOUND, "not_found", "Job nicht gefunden")
        } else {
            ApiError::new(
                StatusCode::CONFLICT,
                "invalid_transition",
                "Für diesen Zustand nicht zulässig",
            )
        });
    }
    if action == "job.requeued" {
        state.job_signal.notify_one();
    }
    let correlation = headers.get("x-request-id").and_then(|v| v.to_str().ok());
    db.audit(
        &user.user.username,
        action,
        Some(&id.to_string()),
        serde_json::json!({}),
        correlation,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    operator_action(&state, &headers, &user, id, "job.cancelled").await
}

pub async fn requeue(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    operator_action(&state, &headers, &user, id, "job.requeued").await
}
