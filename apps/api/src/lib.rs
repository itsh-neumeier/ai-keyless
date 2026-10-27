//! AI-Keyless Management-API.

pub mod auth;
pub mod config;
pub mod crypto;
mod error;
mod jobs;
mod providers;
mod routes;
mod status;
pub mod worker;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use akl_persistence::Database;
use axum::{
    Router,
    http::{HeaderName, HeaderValue, header},
};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Option<Database>,
    /// Setzt das `Secure`-Attribut am Sitzungscookie (bei HTTPS bzw. TLS-Proxy).
    pub cookie_secure: bool,
    pub login_limiter: Arc<auth::LoginLimiter>,
    pub compat: Arc<akl_protect_compat::Registry>,
    /// SSRF-Richtlinie für Provider-URLs.
    pub provider_policy: akl_providers::NetworkPolicy,
    /// Gesamtzeitlimit pro Provider-Anfrage (CPU-Inferenz kann dauern).
    pub provider_timeout: Duration,
    /// Verzeichnis, aus dem Provider-Secrets per Name gelesen werden.
    pub secrets_dir: PathBuf,
    /// Globales, dauerhaftes Limit gestarteter Jobs pro rollierender Stunde.
    pub jobs_per_hour: i64,
    /// Weckt den Worker, sobald ein Job eingereiht wurde.
    pub job_signal: Arc<tokio::sync::Notify>,
}

impl AppState {
    pub fn new(db: Option<Database>, cookie_secure: bool) -> Self {
        Self {
            db,
            cookie_secure,
            login_limiter: Arc::default(),
            // Profildaten werden per Test validiert; ein Fehler ist ein Build-Fehler.
            compat: Arc::new(
                akl_protect_compat::Registry::embedded()
                    .expect("eingebettete Kompatibilitätsprofile"),
            ),
            provider_policy: akl_providers::NetworkPolicy::default(),
            provider_timeout: Duration::from_secs(120),
            secrets_dir: PathBuf::from("/run/secrets"),
            jobs_per_hour: DEFAULT_JOBS_PER_HOUR,
            job_signal: Arc::default(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(None, false)
    }
}

/// Anfängliche Sicherheitsvorgabe (keine Leistungsgrenze), siehe DECISIONS.md.
pub const DEFAULT_JOBS_PER_HOUR: i64 = 12;

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Baut den vollständigen Router. `web_dir` liefert die statisch gebaute
/// SvelteKit-App aus (SPA-Fallback auf `index.html`).
pub fn app(state: AppState, web_dir: Option<&Path>) -> Router {
    let mut router = routes::api()
        .with_state(state)
        .layer(axum::middleware::from_fn(auth::origin_guard));

    if let Some(dir) = web_dir {
        let spa = ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")));
        router = router.fallback_service(spa);
    }

    router
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; \
                 style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
                 connect-src 'self'; frame-ancestors 'none'; base-uri 'self'",
            ),
        ))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
                let correlation_id = req
                    .headers()
                    .get(REQUEST_ID)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-");
                tracing::info_span!(
                    "http",
                    method = %req.method(),
                    path = %req.uri().path(),
                    correlation_id = %correlation_id,
                )
            }),
        )
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
}
