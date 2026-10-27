//! Laufzeit der Geräte-Emulation (AKL-M040-07): startet Discovery-Antwort,
//! Adoptions-Endpunkt und Kontrollkanal für das aktive `ai_key`-Profil.
//!
//! Standardmäßig aus; der Dienst ruft [`supervise`] nur bei ausdrücklichem
//! Opt-in auf (`AKL_DEVICE_EMULATION=true`). Adoption und Kontrollkanal laufen
//! nur, wenn ein vom Admin bestätigter Controller konfiguriert ist – ohne Pin
//! gibt es keinen Endpunkt, der Zugangsdaten annimmt.
//!
//! Der Supervisor prüft periodisch, welches Profil aktiv ist. Wechselt es
//! (neu angelegt, deaktiviert, Kill-Switch), werden die Dienste des alten
//! Profils beendet. Kill-Switch und Zustände prüfen die Dienste zusätzlich
//! selbst bei jeder Anfrage.

use std::{net::SocketAddr, time::Duration};

use akl_core::{DeviceProfileKind, DeviceState};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    adoption::{AdoptionServer, ControllerTrust},
    control::{self, ControlConfig, Exit, RunError},
    discovery::{Announcement, Responder},
    service::Lifecycle,
};

#[derive(Debug, Clone)]
pub struct EmulationConfig {
    pub discovery_bind: SocketAddr,
    pub adoption_bind: SocketAddr,
    /// Vom Admin bestätigter Controller; `None` ⇒ nur Discovery.
    pub controller: Option<ControllerTrust>,
    pub control: ControlConfig,
    /// Frist je Adoptionsverbindung.
    pub adoption_timeout: Duration,
    /// Abstand der Prüfung auf Profilwechsel.
    pub poll_interval: Duration,
}

/// Läuft dauerhaft (bis der Task abgebrochen wird).
pub async fn supervise(lifecycle: Lifecycle, cfg: EmulationConfig) {
    if cfg.controller.is_none() {
        tracing::warn!(
            "Geräte-Emulation ohne bestätigten Controller: nur Discovery, keine Adoption"
        );
    }
    let mut running: Option<(Uuid, JoinSet<()>)> = None;
    loop {
        let target = match active_ai_key(&lifecycle).await {
            Ok(target) => target,
            Err(err) => {
                tracing::debug!(%err, "Profil nicht lesbar (Datenbank/Migrationen?)");
                None
            }
        };
        if running.as_ref().map(|(id, _)| *id) != target {
            if let Some((id, mut tasks)) = running.take() {
                tasks.shutdown().await;
                tracing::info!(profile_id = %id, "Geräte-Emulation für Profil beendet");
            }
            if let Some(id) = target {
                match start(&lifecycle, id, &cfg).await {
                    Ok(tasks) => {
                        tracing::info!(profile_id = %id, "Geräte-Emulation für Profil gestartet");
                        running = Some((id, tasks));
                    }
                    Err(err) => {
                        tracing::error!(%err, profile_id = %id, "Geräte-Emulation nicht startbar")
                    }
                }
            }
        }
        tokio::time::sleep(cfg.poll_interval).await;
    }
}

/// Das aktive `ai_key`-Profil (es gibt höchstens eines, ADR-003); bei aktivem
/// Kill-Switch keines.
async fn active_ai_key(
    lifecycle: &Lifecycle,
) -> Result<Option<Uuid>, crate::service::LifecycleError> {
    if lifecycle.kill_switch().await?.active {
        return Ok(None);
    }
    Ok(lifecycle
        .db()
        .list_device_profiles()
        .await?
        .into_iter()
        .find(|p| p.kind == DeviceProfileKind::AiKey && p.state != DeviceState::Disabled)
        .map(|p| p.id))
}

async fn start(
    lifecycle: &Lifecycle,
    profile_id: Uuid,
    cfg: &EmulationConfig,
) -> Result<JoinSet<()>, Box<dyn std::error::Error + Send + Sync>> {
    let mut tasks = JoinSet::new();
    let responder = Responder::bind(
        cfg.discovery_bind,
        lifecycle.clone(),
        profile_id,
        Announcement::neutral(profile_id, DeviceProfileKind::AiKey),
    )
    .await?;
    tracing::info!(addr = %responder.local_addr()?, "Discovery-Antwort lauscht (UDP)");
    tasks.spawn(async move {
        if let Err(err) = responder.run().await {
            tracing::error!(%err, "Discovery-Antwort beendet");
        }
    });

    let Some(trust) = cfg.controller else {
        return Ok(tasks);
    };
    let server = AdoptionServer::bind(
        cfg.adoption_bind,
        lifecycle.clone(),
        profile_id,
        trust,
        cfg.adoption_timeout,
    )
    .await?;
    tracing::info!(
        addr = %server.local_addr()?,
        controller = %trust.ip,
        fingerprint = %trust.fingerprint,
        "Adoptions-Endpunkt lauscht (TLS)"
    );
    tasks.spawn(async move {
        if let Err(err) = server.run().await {
            tracing::error!(%err, "Adoptions-Endpunkt beendet");
        }
    });

    let lc = lifecycle.clone();
    let control_cfg = cfg.control;
    let wait = cfg.poll_interval;
    tasks.spawn(async move {
        // Wartet auf eine Adoption; endet bei Pin-Wechsel (Zustand `error`,
        // erst ein Admin entscheidet über das weitere Vorgehen).
        loop {
            match control::run(&lc, profile_id, control_cfg).await {
                Ok(Exit::PinMismatch) => {
                    tracing::error!(
                        profile_id = %profile_id,
                        "Controller-Zertifikat entspricht nicht dem Pin – Kontrollkanal gestoppt"
                    );
                    return;
                }
                Ok(Exit::Stopped) | Err(RunError::NotAdopted) => {}
                Err(err) => tracing::warn!(%err, "Kontrollkanal-Fehler"),
            }
            tokio::time::sleep(wait).await;
        }
    });
    Ok(tasks)
}
