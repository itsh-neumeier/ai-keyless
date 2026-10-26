//! Provider-Verwaltung (AKL-M060-02/05/06): anlegen, löschen, Verbindung und
//! Bildverständnis testen. Testergebnisse werden gespeichert, nicht angenommen.

use akl_core::Role;
use akl_persistence::providers::{
    NewProvider, NewProviderTest, ProviderRecord, ProviderTestRecord,
};
use akl_providers::{
    Provider, ProviderConfig, ProviderKind, SYNTHETIC_TEST_IMAGE_PNG, SYNTHETIC_TEST_PROMPT,
    policy::parse_base_url, synthetic_answer_matches,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    auth::CurrentUser,
    error::{ApiError, ApiJson},
};

#[derive(Serialize)]
pub struct ProviderView {
    #[serde(flatten)]
    provider: ProviderRecord,
    tests: Vec<ProviderTestRecord>,
}

fn unprocessable(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
}

fn valid_secret_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

fn correlation(headers: &HeaderMap) -> Option<&str> {
    headers.get("x-request-id").and_then(|v| v.to_str().ok())
}

pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<ProviderView>>, ApiError> {
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let mut out = Vec::new();
    for provider in db.list_providers().await? {
        let tests = db.latest_provider_tests(provider.id).await?;
        out.push(ProviderView { provider, tests });
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    name: String,
    kind: ProviderKind,
    base_url: String,
    #[serde(default)]
    api_key_secret: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Response, ApiError> {
    user.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;

    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(unprocessable("invalid_name", "Name: 1–64 Zeichen"));
    }
    let (url, _) = parse_base_url(&req.base_url, &state.provider_policy)
        .map_err(|e| unprocessable(e.code(), &e.to_string()))?;
    let base_url = url.as_str().trim_end_matches('/').to_owned();
    if base_url.len() > 512 {
        return Err(unprocessable("invalid_url", "URL zu lang"));
    }
    let secret = req
        .api_key_secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(secret) = secret {
        if !valid_secret_name(secret) {
            return Err(unprocessable(
                "invalid_secret_name",
                "Secret-Name: a–z, 0–9, _ und -",
            ));
        }
        if !state.secrets_dir.join(secret).is_file() {
            return Err(unprocessable(
                "secret_missing",
                "Secret-Datei nicht gefunden",
            ));
        }
    }
    let model = req
        .default_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if model.is_some_and(|m| m.chars().count() > 128) {
        return Err(unprocessable("invalid_model", "Modellname zu lang"));
    }

    let Some(created) = db
        .create_provider(NewProvider {
            name,
            kind: req.kind.as_str(),
            base_url: &base_url,
            api_key_secret: secret,
            default_model: model,
        })
        .await?
    else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "name_taken",
            "Name bereits vergeben",
        ));
    };
    db.audit(
        &user.user.username,
        "provider.created",
        Some(&created.name),
        serde_json::json!({ "kind": created.kind, "base_url": created.base_url }),
        correlation(&headers),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ProviderView {
            provider: created,
            tests: Vec::new(),
        }),
    )
        .into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let provider = db.get_provider(id).await?.ok_or_else(not_found)?;
    db.delete_provider(id).await?;
    db.audit(
        &user.user.username,
        "provider.deleted",
        Some(&provider.name),
        serde_json::json!({}),
        correlation(&headers),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "not_found",
        "Provider nicht gefunden",
    )
}

/// Baut den Client; ein nicht lesbares Secret wird als Testergebnis gemeldet.
fn build(state: &AppState, record: &ProviderRecord) -> Result<Provider, &'static str> {
    let kind: ProviderKind = record.kind.parse().map_err(|_| "invalid_kind")?;
    let api_key = match &record.api_key_secret {
        Some(name) => Some(
            std::fs::read_to_string(state.secrets_dir.join(name))
                .map_err(|_| "secret_unreadable")?
                .trim()
                .to_owned(),
        ),
        None => None,
    };
    Ok(Provider::new(
        ProviderConfig {
            kind,
            base_url: record.base_url.clone(),
            api_key,
            timeout: state.provider_timeout,
        },
        state.provider_policy.clone(),
    ))
}

#[derive(Serialize)]
pub struct ConnectionResult {
    ok: bool,
    latency_ms: Option<u64>,
    models: Vec<String>,
    error_code: Option<&'static str>,
}

pub async fn test_connection(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ConnectionResult>, ApiError> {
    user.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let record = db.get_provider(id).await?.ok_or_else(not_found)?;
    let result = match build(&state, &record) {
        Err(code) => ConnectionResult {
            ok: false,
            latency_ms: None,
            models: vec![],
            error_code: Some(code),
        },
        Ok(provider) => match provider.list_models().await {
            Ok(list) => ConnectionResult {
                ok: true,
                latency_ms: Some(list.latency_ms),
                models: list.models,
                error_code: None,
            },
            Err(err) => ConnectionResult {
                ok: false,
                latency_ms: None,
                models: vec![],
                error_code: Some(err.code()),
            },
        },
    };
    db.record_provider_test(
        id,
        NewProviderTest {
            kind: "connection",
            ok: result.ok,
            latency_ms: result.latency_ms.map(|v| v as i64),
            error_code: result.error_code,
            model: None,
            result_text: None,
            matched: None,
        },
    )
    .await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct VisionRequest {
    #[serde(default)]
    model: Option<String>,
}

#[derive(Serialize)]
pub struct VisionTestResult {
    ok: bool,
    model: String,
    latency_ms: Option<u64>,
    text: Option<String>,
    /// Hat das Modell das synthetische Testbild korrekt erkannt?
    matched: Option<bool>,
    error_code: Option<&'static str>,
}

/// Bildtest mit einem eigenen synthetischen Testbild (einfarbig rot). Löst eine
/// Inferenz aus – nur für lokale Provider-Typen vorgesehen.
pub async fn test_vision(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    ApiJson(req): ApiJson<VisionRequest>,
) -> Result<Json<VisionTestResult>, ApiError> {
    user.require(Role::Admin)?;
    let db = state.db.as_ref().ok_or_else(ApiError::database_required)?;
    let record = db.get_provider(id).await?.ok_or_else(not_found)?;
    let model = req
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .or(record.default_model.as_deref())
        .ok_or_else(|| {
            unprocessable(
                "model_required",
                "Modell angeben oder Standardmodell setzen",
            )
        })?
        .to_owned();

    let result = match build(&state, &record) {
        Err(code) => VisionTestResult {
            ok: false,
            model: model.clone(),
            latency_ms: None,
            text: None,
            matched: None,
            error_code: Some(code),
        },
        Ok(provider) => match provider
            .describe_image(&model, SYNTHETIC_TEST_PROMPT, SYNTHETIC_TEST_IMAGE_PNG)
            .await
        {
            Ok(r) => VisionTestResult {
                ok: true,
                model: model.clone(),
                latency_ms: Some(r.latency_ms),
                matched: Some(synthetic_answer_matches(&r.text)),
                text: Some(r.text),
                error_code: None,
            },
            Err(err) => VisionTestResult {
                ok: false,
                model: model.clone(),
                latency_ms: None,
                text: None,
                matched: None,
                error_code: Some(err.code()),
            },
        },
    };
    db.record_provider_test(
        id,
        NewProviderTest {
            kind: "vision",
            ok: result.ok,
            latency_ms: result.latency_ms.map(|v| v as i64),
            error_code: result.error_code,
            model: Some(&model),
            result_text: result.text.as_deref(),
            matched: result.matched,
        },
    )
    .await?;
    db.audit(
        &user.user.username,
        "provider.vision_tested",
        Some(&record.name),
        serde_json::json!({ "model": model, "ok": result.ok, "matched": result.matched }),
        correlation(&headers),
    )
    .await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::valid_secret_name;

    #[test]
    fn secret_names() {
        assert!(valid_secret_name("openai_key"));
        assert!(valid_secret_name("lm-studio-1"));
        for bad in ["", "../etc/passwd", "a/b", "Key", "x y", &"a".repeat(65)] {
            assert!(!valid_secret_name(bad), "{bad}");
        }
    }
}
