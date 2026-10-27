//! Authentifizierung (AKL-M020-01/02): Einrichtung per Einmal-Token, Anmeldung,
//! Sitzungen per HttpOnly-Cookie, Rollenprüfung, CSRF- und Origin-Schutz, Audit.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration as StdDuration, Instant},
};

use akl_core::{
    Role,
    auth::{normalize_username, validate_password},
};
use akl_persistence::{
    Database,
    auth::{CreateUserOutcome, DisableOutcome, SessionRecord, SetupOutcome, User},
};
use axum::{
    Json,
    extract::{FromRequestParts, Path, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, crypto,
    error::{ApiError, ApiJson},
};

pub const SESSION_COOKIE: &str = "akl_session";
pub const CSRF_HEADER: &str = "x-csrf-token";
/// Absolute Lebensdauer einer Sitzung.
pub const SESSION_MAX_AGE: Duration = Duration::days(7);
/// Sitzungen ohne Aktivität verfallen nach dieser Zeit.
pub const SESSION_IDLE: Duration = Duration::hours(12);
/// `last_seen_at` wird höchstens in diesem Abstand geschrieben.
const TOUCH_INTERVAL: Duration = Duration::minutes(5);
pub const SETUP_TOKEN_TTL: Duration = Duration::minutes(60);

// ---------------------------------------------------------------------------
// Anmeldedrosselung

const FREE_ATTEMPTS: u32 = 5;
const BASE_LOCK: StdDuration = StdDuration::from_secs(30);
const MAX_LOCK: StdDuration = StdDuration::from_secs(15 * 60);
const MAX_TRACKED: usize = 10_000;

#[derive(Default)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
}

/// Drosselt Fehlversuche pro Benutzername (exponentielle Sperre ab dem
/// 6. Fehlversuch, max. 15 min). Nur im Speicher: ein Neustart setzt zurück.
#[derive(Default)]
pub struct LoginLimiter {
    entries: Mutex<HashMap<String, Attempts>>,
}

impl LoginLimiter {
    /// Verbleibende Sperrzeit, falls gesperrt.
    pub fn check(&self, username: &str) -> Option<StdDuration> {
        let entries = self.entries.lock().expect("Limiter-Mutex");
        let until = entries.get(username)?.locked_until?;
        until.checked_duration_since(Instant::now())
    }

    pub fn fail(&self, username: &str) {
        let mut entries = self.entries.lock().expect("Limiter-Mutex");
        if entries.len() >= MAX_TRACKED {
            let now = Instant::now();
            entries.retain(|_, a| a.locked_until.is_some_and(|t| t > now));
        }
        let entry = entries.entry(username.to_owned()).or_default();
        entry.failures += 1;
        if entry.failures > FREE_ATTEMPTS {
            let exp = (entry.failures - FREE_ATTEMPTS - 1).min(10);
            let lock = (BASE_LOCK * 2u32.pow(exp)).min(MAX_LOCK);
            entry.locked_until = Some(Instant::now() + lock);
        }
    }

    pub fn reset(&self, username: &str) {
        self.entries.lock().expect("Limiter-Mutex").remove(username);
    }
}

// ---------------------------------------------------------------------------
// Antwort-Typen

#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
}

impl From<&User> for UserView {
    fn from(u: &User) -> Self {
        Self {
            id: u.id,
            username: u.username.clone(),
            role: u.role,
            disabled: u.disabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SessionView {
    pub user: UserView,
    /// Muss bei schreibenden Anfragen im Header `x-csrf-token` mitgesendet werden.
    pub csrf_token: String,
}

// ---------------------------------------------------------------------------
// Extraktor für angemeldete Nutzer

pub struct CurrentUser {
    pub user: User,
    pub token_hash: String,
}

impl CurrentUser {
    pub fn require(&self, role: Role) -> Result<(), ApiError> {
        if self.user.role.allows(role) {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_owned())
        .filter(|v| !v.is_empty())
}

fn is_unsafe(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Lädt die Sitzung zu einem Token-Hash; `None` bei ungültiger Sitzung.
pub async fn load_session(
    db: &Database,
    token_hash: &str,
) -> Result<Option<SessionRecord>, ApiError> {
    let now = Utc::now();
    let Some(record) = db.find_session(token_hash, now - SESSION_IDLE).await? else {
        return Ok(None);
    };
    if now - record.last_seen_at > TOUCH_INTERVAL {
        db.touch_session(token_hash).await?;
    }
    Ok(Some(record))
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
        let token = session_token(&parts.headers).ok_or_else(ApiError::unauthenticated)?;
        let token_hash = crypto::token_hash(&token);
        let record = load_session(db, &token_hash)
            .await?
            .ok_or_else(ApiError::unauthenticated)?;

        if is_unsafe(&parts.method) {
            let sent = parts.headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok());
            if !sent.is_some_and(|s| crypto::constant_time_eq(s, &record.csrf_token)) {
                return Err(ApiError::new(
                    StatusCode::FORBIDDEN,
                    "csrf_failed",
                    "CSRF-Token fehlt oder ist ungültig",
                ));
            }
        }
        Ok(Self {
            user: record.user,
            token_hash,
        })
    }
}

// ---------------------------------------------------------------------------
// Origin-Schutz für alle schreibenden Anfragen (auch Anmeldung/Einrichtung)

fn host_of(origin: &str) -> Option<&str> {
    let rest = origin.split_once("://")?.1;
    Some(rest.split('/').next().unwrap_or(rest))
}

pub async fn origin_guard(req: Request, next: Next) -> Response {
    if is_unsafe(req.method()) {
        let headers = req.headers();
        let cross_site = headers
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v == "cross-site");
        let origin_mismatch = headers
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|origin| {
                let Some(origin_host) = host_of(origin) else {
                    return true;
                };
                // X-Forwarded-Host kann ein Browser cross-origin nicht ohne
                // CORS-Preflight setzen, den die API nie beantwortet.
                ![header::HOST.as_str(), "x-forwarded-host"]
                    .iter()
                    .any(|name| {
                        headers
                            .get(*name)
                            .and_then(|v| v.to_str().ok())
                            .is_some_and(|h| h.eq_ignore_ascii_case(origin_host))
                    })
            });
        if cross_site || origin_mismatch {
            return ApiError::new(
                StatusCode::FORBIDDEN,
                "origin_rejected",
                "Anfrage von fremder Herkunft abgelehnt",
            )
            .into_response();
        }
    }
    next.run(req).await
}

// ---------------------------------------------------------------------------
// Hilfen

fn correlation_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn session_cookie(token: &str, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{secure}",
        SESSION_MAX_AGE.num_seconds()
    ))
    .expect("Cookie-Wert ist ASCII")
}

fn clear_cookie(secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}"
    ))
    .expect("Cookie-Wert ist ASCII")
}

async fn hash_password_blocking(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || crypto::hash_password(&password))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(|err| {
            tracing::error!(error = %err, "Passwort-Hashing fehlgeschlagen");
            ApiError::internal()
        })
}

/// Vergleichs-Hash, damit unbekannte Konten gleich lange prüfen wie bekannte.
fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| crypto::hash_password(&crypto::random_token()).expect("Dummy-Hash"))
}

async fn start_session(
    state: &AppState,
    db: &Database,
    user: &User,
) -> Result<(HeaderValue, SessionView), ApiError> {
    let token = crypto::random_token();
    let csrf_token = crypto::random_token();
    db.create_session(
        &crypto::token_hash(&token),
        user.id,
        &csrf_token,
        Utc::now() + SESSION_MAX_AGE,
    )
    .await?;
    Ok((
        session_cookie(&token, state.cookie_secure),
        SessionView {
            user: user.into(),
            csrf_token,
        },
    ))
}

fn credential_error(err: akl_core::auth::CredentialError) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        err.code(),
        err.to_string(),
    )
}

// ---------------------------------------------------------------------------
// Handler

#[derive(Serialize)]
pub struct AuthState {
    /// `null`, wenn die Datenbank nicht verfügbar ist (Zustand unbekannt).
    setup_required: Option<bool>,
}

pub async fn auth_state(State(state): State<AppState>) -> Result<Json<AuthState>, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let count = db.user_count().await?;
    Ok(Json(AuthState {
        setup_required: Some(count == 0),
    }))
}

#[derive(Deserialize)]
pub struct SetupRequest {
    token: String,
    username: String,
    password: String,
}

pub async fn setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<SetupRequest>,
) -> Result<Response, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let username = normalize_username(&req.username).map_err(credential_error)?;
    validate_password(&req.password).map_err(credential_error)?;
    let password_hash = hash_password_blocking(req.password).await?;
    let correlation = correlation_id(&headers);

    match db
        .complete_setup(
            &crypto::token_hash(req.token.trim()),
            &username,
            &password_hash,
        )
        .await?
    {
        SetupOutcome::Completed(user) => {
            db.audit(
                &user.username,
                "auth.setup_completed",
                None,
                serde_json::json!({}),
                correlation.as_deref(),
            )
            .await?;
            let (cookie, view) = start_session(&state, db, &user).await?;
            Ok((
                StatusCode::CREATED,
                [(header::SET_COOKIE, cookie)],
                Json(view),
            )
                .into_response())
        }
        SetupOutcome::AlreadyCompleted => Err(ApiError::new(
            StatusCode::CONFLICT,
            "setup_completed",
            "Einrichtung bereits abgeschlossen",
        )),
        SetupOutcome::InvalidToken => {
            db.audit(
                "anonymous",
                "auth.setup_rejected",
                None,
                serde_json::json!({}),
                correlation.as_deref(),
            )
            .await?;
            Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_setup_token",
                "Einrichtungs-Token ungültig, abgelaufen oder bereits verwendet",
            ))
        }
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> Result<Response, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let correlation = correlation_id(&headers);
    // Ungültige Namen gelten als falsche Anmeldedaten (keine Auskunft über Regeln).
    let username = req.username.trim().to_lowercase();

    if let Some(wait) = state.login_limiter.check(&username) {
        let mut err = ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_attempts",
            "Zu viele Fehlversuche – bitte später erneut versuchen",
        )
        .into_response();
        if let Ok(v) = HeaderValue::from_str(&wait.as_secs().max(1).to_string()) {
            err.headers_mut().insert(header::RETRY_AFTER, v);
        }
        return Ok(err);
    }

    let found = db.find_credentials(&username).await?;
    let phc = found
        .as_ref()
        .map(|(_, hash)| hash.clone())
        .unwrap_or_else(|| dummy_hash().to_owned());
    let password = req.password;
    let valid = tokio::task::spawn_blocking(move || crypto::verify_password(&password, &phc))
        .await
        .map_err(|_| ApiError::internal())?;

    match found {
        Some((user, _)) if valid && !user.disabled => {
            state.login_limiter.reset(&username);
            db.audit(
                &user.username,
                "auth.login",
                None,
                serde_json::json!({}),
                correlation.as_deref(),
            )
            .await?;
            let (cookie, view) = start_session(&state, db, &user).await?;
            Ok(([(header::SET_COOKIE, cookie)], Json(view)).into_response())
        }
        _ => {
            state.login_limiter.fail(&username);
            db.audit(
                "anonymous",
                "auth.login_failed",
                Some(&username),
                serde_json::json!({}),
                correlation.as_deref(),
            )
            .await?;
            Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Benutzername oder Passwort falsch",
            ))
        }
    }
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    current: CurrentUser,
) -> Result<Response, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    db.delete_session(&current.token_hash).await?;
    db.audit(
        &current.user.username,
        "auth.logout",
        None,
        serde_json::json!({}),
        correlation_id(&headers).as_deref(),
    )
    .await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, clear_cookie(state.cookie_secure))],
    )
        .into_response())
}

pub async fn session(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<SessionView>, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let record = db
        .find_session(&current.token_hash, Utc::now() - SESSION_IDLE)
        .await?
        .ok_or_else(ApiError::unauthenticated)?;
    Ok(Json(SessionView {
        user: (&record.user).into(),
        csrf_token: record.csrf_token,
    }))
}

pub async fn list_users(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<Vec<UserView>>, ApiError> {
    current.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    Ok(Json(
        db.list_users().await?.iter().map(UserView::from).collect(),
    ))
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    username: String,
    password: String,
    role: Role,
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    current: CurrentUser,
    ApiJson(req): ApiJson<CreateUserRequest>,
) -> Result<Response, ApiError> {
    current.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let username = normalize_username(&req.username).map_err(credential_error)?;
    validate_password(&req.password).map_err(credential_error)?;
    let password_hash = hash_password_blocking(req.password).await?;
    match db.create_user(&username, &password_hash, req.role).await? {
        CreateUserOutcome::Created(user) => {
            db.audit(
                &current.user.username,
                "user.created",
                Some(&user.username),
                serde_json::json!({ "role": user.role }),
                correlation_id(&headers).as_deref(),
            )
            .await?;
            Ok((StatusCode::CREATED, Json(UserView::from(&user))).into_response())
        }
        CreateUserOutcome::UsernameTaken => Err(ApiError::new(
            StatusCode::CONFLICT,
            "username_taken",
            "Benutzername bereits vergeben",
        )),
    }
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    disabled: bool,
}

/// Konto aktivieren/deaktivieren (Admin). Nicht für das eigene Konto; das
/// letzte aktive Admin-Konto bleibt geschützt. Deaktivieren beendet Sitzungen.
pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    current: CurrentUser,
    Path(id): Path<Uuid>,
    ApiJson(req): ApiJson<UpdateUserRequest>,
) -> Result<Json<UserView>, ApiError> {
    current.require(Role::Admin)?;
    if current.user.id == id {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "cannot_modify_self",
            "Das eigene Konto kann nicht deaktiviert werden",
        ));
    }
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    match db.set_user_disabled_by_id(id, req.disabled).await? {
        DisableOutcome::NotFound => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Benutzer nicht gefunden",
        )),
        DisableOutcome::LastAdmin => Err(ApiError::new(
            StatusCode::CONFLICT,
            "last_admin",
            "Das letzte aktive Administrator-Konto kann nicht deaktiviert werden",
        )),
        DisableOutcome::Updated(user) => {
            let action = if user.disabled {
                "user.disabled"
            } else {
                "user.enabled"
            };
            db.audit(
                &current.user.username,
                action,
                Some(&user.username),
                serde_json::json!({}),
                correlation_id(&headers).as_deref(),
            )
            .await?;
            Ok(Json(UserView::from(&user)))
        }
    }
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

/// Eigenes Passwort ändern. Erfordert das aktuelle Passwort (mit derselben
/// Drosselung wie die Anmeldung) und beendet alle anderen Sitzungen.
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    current: CurrentUser,
    ApiJson(req): ApiJson<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let username = current.user.username.clone();
    if state.login_limiter.check(&username).is_some() {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_attempts",
            "Zu viele Fehlversuche – bitte später erneut versuchen",
        ));
    }
    validate_password(&req.new_password).map_err(credential_error)?;
    let (_, phc) = db
        .find_credentials(&username)
        .await?
        .ok_or_else(ApiError::unauthenticated)?;
    let current_password = req.current_password;
    let valid =
        tokio::task::spawn_blocking(move || crypto::verify_password(&current_password, &phc))
            .await
            .map_err(|_| ApiError::internal())?;
    if !valid {
        state.login_limiter.fail(&username);
        db.audit(
            &username,
            "auth.password_change_failed",
            None,
            serde_json::json!({}),
            correlation_id(&headers).as_deref(),
        )
        .await?;
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_current_password",
            "Aktuelles Passwort falsch",
        ));
    }
    state.login_limiter.reset(&username);
    let hash = hash_password_blocking(req.new_password).await?;
    let ended = db
        .change_password(current.user.id, &hash, &current.token_hash)
        .await?;
    db.audit(
        &username,
        "auth.password_changed",
        None,
        serde_json::json!({ "other_sessions_ended": ended }),
        correlation_id(&headers).as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_locks_after_free_attempts_and_resets() {
        let limiter = LoginLimiter::default();
        for _ in 0..FREE_ATTEMPTS {
            limiter.fail("a");
            assert!(limiter.check("a").is_none());
        }
        limiter.fail("a");
        let wait = limiter.check("a").expect("gesperrt");
        assert!(wait <= BASE_LOCK && wait > StdDuration::from_secs(25));
        assert!(limiter.check("b").is_none(), "andere Konten unberührt");
        limiter.reset("a");
        assert!(limiter.check("a").is_none());
    }

    #[test]
    fn limiter_lock_is_capped() {
        let limiter = LoginLimiter::default();
        for _ in 0..40 {
            limiter.fail("a");
        }
        assert!(limiter.check("a").unwrap() <= MAX_LOCK);
    }

    #[test]
    fn cookie_parsing() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            HeaderValue::from_static("x=1; akl_session=abc; y=2"),
        );
        assert_eq!(session_token(&h).as_deref(), Some("abc"));
        h.insert(header::COOKIE, HeaderValue::from_static("akl_session="));
        assert!(session_token(&h).is_none());
    }

    #[test]
    fn cookie_attributes() {
        let c = session_cookie("t", true);
        let s = c.to_str().unwrap();
        for attr in [
            "HttpOnly",
            "SameSite=Strict",
            "Path=/",
            "Secure",
            "Max-Age=604800",
        ] {
            assert!(s.contains(attr), "{attr}");
        }
        assert!(
            !session_cookie("t", false)
                .to_str()
                .unwrap()
                .contains("Secure")
        );
    }

    #[test]
    fn origin_host_parsing() {
        assert_eq!(host_of("http://nas.lan:8090"), Some("nas.lan:8090"));
        assert_eq!(host_of("https://a.b/x"), Some("a.b"));
        assert_eq!(host_of("null"), None);
    }
}
