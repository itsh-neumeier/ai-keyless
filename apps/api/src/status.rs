//! Ermittlung des ehrlichen Systemstatus. Unbekanntes bleibt unbekannt.

use akl_core::{
    DeviceProfileKind, EvidenceLevel,
    status::{ComponentStatus, DatabaseStatus, DeviceProfileStatus, SystemStatus},
};

use crate::AppState;

/// Evidenz der Gerätelebenszyklus-Implementierung. Wird nur durch Code- und
/// Testfortschritt geändert, niemals zur Laufzeit hochgestuft.
const LIFECYCLE_EVIDENCE: [(DeviceProfileKind, EvidenceLevel); 2] = [
    (DeviceProfileKind::AiKey, EvidenceLevel::Planned),
    (DeviceProfileKind::AiPort, EvidenceLevel::Planned),
];

pub async fn collect(state: &AppState) -> SystemStatus {
    let (database, stored) = match &state.db {
        None => (
            DatabaseStatus {
                status: ComponentStatus::NotConfigured,
                schema_version: None,
            },
            Vec::new(),
        ),
        Some(db) => match db.ping().await {
            Err(err) => {
                tracing::warn!(error = %err, "Datenbank nicht erreichbar");
                (
                    DatabaseStatus {
                        status: ComponentStatus::Unavailable,
                        schema_version: None,
                    },
                    Vec::new(),
                )
            }
            Ok(()) => {
                let schema_version = db.schema_version().await.ok().flatten();
                let devices = db.device_states().await.unwrap_or_else(|err| {
                    tracing::warn!(error = %err, "Gerätestatus nicht lesbar");
                    Vec::new()
                });
                let status = if schema_version.is_some() {
                    ComponentStatus::Ok
                } else {
                    ComponentStatus::Degraded
                };
                (
                    DatabaseStatus {
                        status,
                        schema_version,
                    },
                    devices,
                )
            }
        },
    };

    let devices = LIFECYCLE_EVIDENCE
        .iter()
        .map(|&(profile, lifecycle_evidence)| DeviceProfileStatus {
            profile,
            state: stored
                .iter()
                .find(|(kind, _)| *kind == profile)
                .map(|(_, s)| *s),
            lifecycle_evidence,
        })
        .collect();

    SystemStatus {
        service: "ai-keyless",
        version: env!("CARGO_PKG_VERSION"),
        unofficial: true,
        database,
        devices,
    }
}
