//! Typisierte Statusverträge der Management-API (`/api/v1/system/status`).
//!
//! Diese Typen sind die Quelle für `docs/api/openapi.yaml` und die
//! TypeScript-Typen in `apps/web/src/lib/api/types.ts`.

use serde::Serialize;

use crate::{DeviceProfileKind, DeviceState, EvidenceLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Ok,
    Degraded,
    Unavailable,
    NotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatabaseStatus {
    pub status: ComponentStatus,
    /// Höchste angewendete Migrationsversion, falls erreichbar.
    pub schema_version: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceProfileStatus {
    pub profile: DeviceProfileKind,
    /// `None` bedeutet: kein Profil angelegt bzw. Zustand unbekannt.
    /// Es wird niemals ein Zustand erfunden.
    pub state: Option<DeviceState>,
    /// Evidenz der Lifecycle-Implementierung dieses Profils.
    pub lifecycle_evidence: EvidenceLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemStatus {
    pub service: &'static str,
    pub version: &'static str,
    /// Immer `true`: AI-Keyless ist ein inoffizielles, unabhängiges Projekt.
    pub unofficial: bool,
    pub database: DatabaseStatus,
    pub devices: Vec<DeviceProfileStatus>,
}
