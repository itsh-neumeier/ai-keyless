//! Geräteprofile (AKL-M040-02): anlegen, deaktivieren, reaktivieren,
//! Übergangsprotokoll, Kill-Switch.
//!
//! Protokollgetriebene Übergänge (Discovery, Adoption, …) gibt es hier bewusst
//! nicht; die API kann nur den sicheren Rückweg (`disabled`) und dessen
//! Aufhebung (`unconfigured`) auslösen. Eine gespeicherte Controller-Bindung
//! wird nur mit ihren öffentlichen Teilen gezeigt (nie das Token) und lässt
//! sich bei deaktiviertem Profil entfernen (Rückweg „Gerät entfernen“).

use akl_core::{DeviceProfileKind, DeviceState, Role};
use akl_device_lifecycle::service::{KillSwitchStatus, Lifecycle, LifecycleError};
use akl_persistence::devices::{DeviceProfileRecord, TransitionError, TransitionRequest};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, ControllerInfo, EmulationStatus,
    auth::CurrentUser,
    error::{ApiError, ApiJson},
    providers::correlation,
};

const TRANSITION_LIMIT: i64 = 50;

#[derive(Serialize)]
pub struct DeviceView {
    #[serde(flatten)]
    profile: DeviceProfileRecord,
    /// SHA-256 des Gerätezertifikats (öffentlich), `None` ohne Identität.
    fingerprint: Option<String>,
    /// Gespeicherte Adoption (öffentliche Teile), `None` ohne Bindung.
    controller: Option<ControllerInfo>,
}

#[derive(Serialize)]
pub struct DeviceList {
    kill_switch: KillSwitchStatus,
    emulation: EmulationStatus,
    profiles: Vec<DeviceView>,
}

fn lifecycle(state: &AppState) -> Result<&Lifecycle, ApiError> {
    state.lifecycle.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "lifecycle_unavailable",
            "Geräteverwaltung nicht verfügbar (Datenbank oder AKL_DEVICE_DIR fehlt)",
        )
    })
}

impl From<LifecycleError> for ApiError {
    fn from(err: LifecycleError) -> Self {
        let conflict = |code, message: &str| ApiError::new(StatusCode::CONFLICT, code, message);
        match err {
            LifecycleError::ActiveAiKeyExists
            | LifecycleError::Transition(TransitionError::ActiveAiKeyExists) => {
                conflict("ai_key_exists", "Es gibt bereits ein aktives AI-Key-Profil")
            }
            LifecycleError::KillSwitchActive
            | LifecycleError::Transition(TransitionError::KillSwitch) => {
                conflict("kill_switch_active", "Kill-Switch aktiv")
            }
            LifecycleError::KillSwitchForced => conflict(
                "kill_switch_forced",
                "Kill-Switch ist per AKL_DEVICE_KILL_SWITCH erzwungen",
            ),
            LifecycleError::Transition(TransitionError::Invalid(e)) => {
                conflict("invalid_transition", &e.to_string())
            }
            LifecycleError::Transition(TransitionError::Stale { .. }) => {
                conflict("state_changed", "Zustand hat sich inzwischen geändert")
            }
            LifecycleError::Transition(TransitionError::NotFound) => ApiError::new(
                StatusCode::NOT_FOUND,
                "not_found",
                "Geräteprofil nicht gefunden",
            ),
            LifecycleError::Transition(TransitionError::Persistence(e))
            | LifecycleError::Persistence(e) => e.into(),
            LifecycleError::Identity(e) => {
                tracing::error!(error = %e, "Geräteidentität");
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "identity_error",
                    "Geräteidentität nicht nutzbar",
                )
            }
        }
    }
}

fn view(lc: &Lifecycle, profile: DeviceProfileRecord) -> Result<DeviceView, ApiError> {
    let fingerprint = lc.fingerprint(&profile)?.map(|f| f.to_string());
    let controller = lc
        .identities()
        .load_binding(profile.id)
        .map_err(LifecycleError::from)?
        .map(|b| b.trust.into());
    Ok(DeviceView {
        profile,
        fingerprint,
        controller,
    })
}

pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<DeviceList>, ApiError> {
    let lc = lifecycle(&state)?;
    let profiles = lc
        .db()
        .list_device_profiles()
        .await?
        .into_iter()
        .map(|p| view(lc, p))
        .collect::<Result<_, _>>()?;
    Ok(Json(DeviceList {
        kill_switch: lc.kill_switch().await?,
        emulation: state.emulation.clone(),
        profiles,
    }))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    kind: DeviceProfileKind,
    display_name: String,
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Response, ApiError> {
    user.require(Role::Admin)?;
    let lc = lifecycle(&state)?;
    let name = req.display_name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_name",
            "Name: 1–64 Zeichen",
        ));
    }
    let (profile, fingerprint) = lc.create_profile(req.kind, name).await?;
    lc.db()
        .audit(
            &user.user.username,
            "device.created",
            Some(&profile.id.to_string()),
            serde_json::json!({ "kind": profile.kind, "fingerprint": fingerprint.to_string() }),
            correlation(&headers),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(DeviceView {
            profile,
            fingerprint: Some(fingerprint.to_string()),
            controller: None,
        }),
    )
        .into_response())
}

#[derive(Deserialize, Default)]
pub struct DisableRequest {
    #[serde(default)]
    reason: Option<String>,
}

/// Ein per API auslösbarer Übergang.
struct Change<'a> {
    to: DeviceState,
    expected_from: Option<DeviceState>,
    reason: &'a str,
    action: &'a str,
}

async fn change(
    state: &AppState,
    headers: &HeaderMap,
    user: &CurrentUser,
    id: Uuid,
    c: Change<'_>,
) -> Result<Json<DeviceView>, ApiError> {
    user.require(Role::Admin)?;
    let lc = lifecycle(state)?;
    let actor = format!("user:{}", user.user.username);
    let profile = lc
        .transition(
            id,
            TransitionRequest {
                to: c.to,
                expected_from: c.expected_from,
                reason: c.reason,
                actor: &actor,
                correlation_id: correlation(headers),
            },
        )
        .await?;
    lc.db()
        .audit(
            &user.user.username,
            c.action,
            Some(&id.to_string()),
            serde_json::json!({ "reason": c.reason }),
            correlation(headers),
        )
        .await?;
    Ok(Json(view(lc, profile)?))
}

/// Body optional: leer oder `{"reason": "…"}`.
pub async fn disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    body: Bytes,
) -> Result<Json<DeviceView>, ApiError> {
    let req: DisableRequest = if body.is_empty() {
        DisableRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_body",
                "Erwartet: {\"reason\": \"…\"}",
            )
        })?
    };
    let reason = req
        .reason
        .map(|r| r.trim().chars().take(200).collect::<String>())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "manuell deaktiviert".into());
    let c = Change {
        to: DeviceState::Disabled,
        expected_from: None,
        reason: &reason,
        action: "device.disabled",
    };
    change(&state, &headers, &user, id, c).await
}

pub async fn enable(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceView>, ApiError> {
    let c = Change {
        to: DeviceState::Unconfigured,
        expected_from: Some(DeviceState::Disabled),
        reason: "manuell reaktiviert",
        action: "device.enabled",
    };
    change(&state, &headers, &user, id, c).await
}

/// Entfernt die Controller-Bindung (Token und vertrauten Controller). Nur bei
/// deaktiviertem Profil, damit kein Kontrollkanal mehr darauf zugreift.
pub async fn forget_controller(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceView>, ApiError> {
    user.require(Role::Admin)?;
    let lc = lifecycle(&state)?;
    let profile = lc
        .db()
        .get_device_profile(id)
        .await?
        .ok_or(LifecycleError::Transition(TransitionError::NotFound))?;
    if profile.state != DeviceState::Disabled {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "must_be_disabled",
            "Bindung nur bei deaktiviertem Profil entfernbar",
        ));
    }
    let had_binding = lc
        .identities()
        .load_binding(id)
        .map_err(LifecycleError::from)?
        .is_some();
    lc.identities()
        .forget_binding(id)
        .map_err(LifecycleError::from)?;
    lc.db()
        .audit(
            &user.user.username,
            "device.controller_forgotten",
            Some(&id.to_string()),
            serde_json::json!({ "had_binding": had_binding }),
            correlation(&headers),
        )
        .await?;
    Ok(Json(view(lc, profile)?))
}

pub async fn transitions(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let lc = lifecycle(&state)?;
    if lc.db().get_device_profile(id).await?.is_none() {
        return Err(LifecycleError::Transition(TransitionError::NotFound).into());
    }
    let log = lc.db().device_transitions(id, TRANSITION_LIMIT).await?;
    Ok(Json(serde_json::json!({ "transitions": log })).into_response())
}

pub async fn kill_switch(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<KillSwitchStatus>, ApiError> {
    Ok(Json(lifecycle(&state)?.kill_switch().await?))
}

#[derive(Deserialize)]
pub struct KillSwitchRequest {
    active: bool,
}

pub async fn set_kill_switch(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: CurrentUser,
    ApiJson(req): ApiJson<KillSwitchRequest>,
) -> Result<Response, ApiError> {
    user.require(Role::Admin)?;
    let lc = lifecycle(&state)?;
    let actor = format!("user:{}", user.user.username);
    let disabled = lc
        .set_kill_switch(req.active, &actor, correlation(&headers))
        .await?;
    lc.db()
        .audit(
            &user.user.username,
            if req.active {
                "device.kill_switch_on"
            } else {
                "device.kill_switch_off"
            },
            None,
            serde_json::json!({ "disabled_profiles": disabled.len() }),
            correlation(&headers),
        )
        .await?;
    Ok(Json(serde_json::json!({
        "kill_switch": lc.kill_switch().await?,
        "disabled_profiles": disabled.len(),
    }))
    .into_response())
}
