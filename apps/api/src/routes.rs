use std::{convert::Infallible, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{any, get, post},
};
use futures::Stream;

use crate::{
    AppState,
    auth::{self, CurrentUser},
    devices, jobs, providers, status,
};

/// Intervall, in dem der Server den Status prüft. Clients erhalten nur
/// Änderungen – kein Polling durch den Browser.
const STATUS_CHECK_INTERVAL: Duration = Duration::from_secs(5);

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/auth/state", get(auth::auth_state))
        .route("/api/v1/auth/setup", post(auth::setup))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/session", get(auth::session))
        .route(
            "/api/v1/users",
            get(auth::list_users).post(auth::create_user),
        )
        .route(
            "/api/v1/users/{id}",
            axum::routing::patch(auth::update_user),
        )
        .route("/api/v1/auth/password", post(auth::change_password))
        .route("/api/v1/compat/profiles", get(compat_profiles))
        .route(
            "/api/v1/providers",
            get(providers::list).post(providers::create),
        )
        .route(
            "/api/v1/providers/{id}",
            axum::routing::delete(providers::delete),
        )
        .route(
            "/api/v1/providers/{id}/test",
            post(providers::test_connection),
        )
        .route(
            "/api/v1/providers/{id}/test-vision",
            post(providers::test_vision),
        )
        .route(
            "/api/v1/providers/{id}/probe-job",
            post(providers::enqueue_probe),
        )
        .route("/api/v1/devices", get(devices::list).post(devices::create))
        .route(
            "/api/v1/devices/kill-switch",
            get(devices::kill_switch).put(devices::set_kill_switch),
        )
        .route("/api/v1/devices/{id}/disable", post(devices::disable))
        .route("/api/v1/devices/{id}/enable", post(devices::enable))
        .route(
            "/api/v1/devices/{id}/transitions",
            get(devices::transitions),
        )
        .route("/api/v1/jobs", get(jobs::list))
        .route("/api/v1/jobs/stats", get(jobs::stats))
        .route("/api/v1/jobs/{id}/cancel", post(jobs::cancel))
        .route("/api/v1/jobs/{id}/requeue", post(jobs::requeue))
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

/// Versionierte Kompatibilitätsprofile mit Evidenzstufen (jede Rolle).
async fn compat_profiles(State(state): State<AppState>, _user: CurrentUser) -> impl IntoResponse {
    use akl_core::DeviceProfileKind::{AiKey, AiPort};
    let profiles: Vec<_> = [AiKey, AiPort]
        .into_iter()
        .map(|kind| {
            serde_json::json!({
                "profile": kind,
                "ranges": state.compat.ranges(kind).collect::<Vec<_>>(),
            })
        })
        .collect();
    Json(serde_json::json!({ "profiles": profiles }))
}

/// Erfordert eine Sitzung (jede Rolle).
async fn system_status(State(state): State<AppState>, _user: CurrentUser) -> impl IntoResponse {
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
