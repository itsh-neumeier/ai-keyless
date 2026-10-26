use std::{convert::Infallible, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{any, get},
};
use futures::Stream;

use crate::{AppState, status};

/// Intervall, in dem der Server den Status prüft. Clients erhalten nur
/// Änderungen – kein Polling durch den Browser.
const STATUS_CHECK_INTERVAL: Duration = Duration::from_secs(5);

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/system/status/stream", get(system_status_stream))
        // Unbekannte API-Pfade nie an den SPA-Fallback weiterreichen.
        .route("/api", any(api_not_found))
        .route("/api/{*rest}", any(api_not_found))
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": { "code": "not_found" } })),
    )
}

/// Liveness: der Prozess antwortet. Sagt nichts über Abhängigkeiten aus.
async fn healthz() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Readiness: Datenbank (falls konfiguriert) erreichbar und migriert.
async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let report = status::collect(&state).await;
    let ready = matches!(
        report.database.status,
        akl_core::status::ComponentStatus::Ok | akl_core::status::ComponentStatus::NotConfigured
    );
    let code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        code,
        Json(serde_json::json!({ "ready": ready, "database": report.database })),
    )
}

async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(status::collect(&state).await)
}

async fn system_status_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::unfold((state, None::<String>), |(state, mut last)| async move {
        loop {
            if last.is_some() {
                tokio::time::sleep(STATUS_CHECK_INTERVAL).await;
            }
            let payload = serde_json::to_string(&status::collect(&state).await)
                .expect("SystemStatus ist serialisierbar");
            if last.as_deref() != Some(payload.as_str()) {
                let event = Event::default().event("status").data(payload.clone());
                return Some((Ok(event), (state, Some(payload))));
            }
            last = Some(payload);
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
