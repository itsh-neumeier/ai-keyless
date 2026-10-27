//! Kontrollkanal mit Keepalive und Reconnect (AKL-M040-04) – Hypothese.
//!
//! Berichtet wird „UCP4 über WebSocket“ (Q-007, `needs_evidence`). Bis dahin
//! modelliert der Kanal nur das Überprüfbare: gepinnte TLS-Verbindung zum
//! vertrauten Controller, Anmeldung mit dem gespeicherten Token, periodischer
//! Keepalive mit Bestätigung. Die Zeilenrahmung ist ein synthetischer
//! Platzhalter für `akl-protect-sim`.
//!
//! Zustände (nur über den Lifecycle):
//! - Verbindung steht: `adopted → online`, `reconnecting → online`; nach einem
//!   Neustart (`online`/`offline` aus der DB) über `reconnecting → online` –
//!   ohne erneute Adoption.
//! - Verbindung verloren: `online → reconnecting`; nach
//!   [`ControlConfig::offline_after`] Fehlversuchen `→ offline` (weiter versuchen).
//! - Pin stimmt nicht mehr (T1): `→ error`, Kanal endet; Token wird nicht gesendet.
//! - Kill-Switch oder Profil nicht mehr adoptiert (z. B. `→ disabled`): Kanal endet.

use std::time::Duration;

use akl_core::DeviceState;
use akl_persistence::devices::{TransitionError, TransitionRequest};
use akl_protect_compat::tls::{self, ConnectError};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

use crate::{
    adoption::{ControllerTrust, MAX_LINE, Secret},
    service::{Lifecycle, LifecycleError},
};

#[derive(Debug, Clone, Copy)]
pub struct ControlConfig {
    pub keepalive_interval: Duration,
    /// Frist für Verbindungsaufbau und jede Bestätigung.
    pub timeout: Duration,
    pub retry_delay: Duration,
    pub offline_after: u32,
}

impl Default for ControlConfig {
    /// Werte ohne Protect-Beleg; nur vorsichtige Annahmen.
    fn default() -> Self {
        Self {
            keepalive_interval: Duration::from_secs(15),
            timeout: Duration::from_secs(10),
            retry_delay: Duration::from_secs(5),
            offline_after: 6,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error(transparent)]
    Connect(#[from] ConnectError),
    #[error("Kontrollkanal: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zeitüberschreitung")]
    Timeout,
    #[error("Controller hat nicht bestätigt")]
    NotAcknowledged,
}

impl ControlError {
    pub fn is_pin_mismatch(&self) -> bool {
        matches!(self, Self::Connect(err) if err.is_pin_mismatch())
    }
}

/// Bestehende, angemeldete Verbindung zum Controller.
pub(crate) struct Session {
    reader: BufReader<tokio::io::Take<ReadHalf<TlsStream<TcpStream>>>>,
    writer: WriteHalf<TlsStream<TcpStream>>,
    seq: u64,
    timeout: Duration,
}

impl Session {
    /// Gepinnt verbinden und mit dem Token anmelden. Bei Pin-Abweichung
    /// scheitert bereits der Handshake – das Token verlässt das Gerät nicht.
    pub(crate) async fn open(
        trust: &ControllerTrust,
        profile_id: Uuid,
        token: &Secret,
        timeout: Duration,
    ) -> Result<Self, ControlError> {
        let stream = tls::connect_pinned(
            &trust.ip.to_string(),
            trust.port,
            trust.fingerprint,
            timeout,
        )
        .await?;
        let (read, writer) = tokio::io::split(stream);
        let mut session = Self {
            // Obergrenze über die ganze Sitzung; reicht für viele Bestätigungen.
            reader: BufReader::new(read.take(MAX_LINE * 1024)),
            writer,
            seq: 0,
            timeout,
        };
        let hello = serde_json::json!({
            "type": "hello",
            "profile_id": profile_id,
            "token": token.expose(),
        });
        session.exchange(&hello).await?;
        Ok(session)
    }

    async fn keepalive(&mut self) -> Result<(), ControlError> {
        self.seq += 1;
        let msg = serde_json::json!({ "type": "keepalive", "seq": self.seq });
        self.exchange(&msg).await
    }

    async fn exchange(&mut self, msg: &serde_json::Value) -> Result<(), ControlError> {
        let mut line = msg.to_string();
        line.push('\n');
        let timeout = self.timeout;
        tokio::time::timeout(timeout, async {
            self.writer.write_all(line.as_bytes()).await?;
            let mut ack = String::new();
            self.reader.read_line(&mut ack).await?;
            if ack.trim_end() == "OK" {
                Ok(())
            } else {
                Err(ControlError::NotAcknowledged)
            }
        })
        .await
        .map_err(|_| ControlError::Timeout)?
    }
}

/// Warum der Kanal beendet wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Kill-Switch, Deaktivierung oder Profil nicht (mehr) adoptiert.
    Stopped,
    /// Controller-Zertifikat entspricht nicht mehr dem Pin (`→ error`).
    PinMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error("Profil ist nicht adoptiert (keine Bindung)")]
    NotAdopted,
}

/// Hält den Kontrollkanal, bis er gestoppt wird. Wird nach jeder Adoption und
/// bei jedem Start des Dienstes aufgerufen.
pub async fn run(
    lifecycle: &Lifecycle,
    profile_id: Uuid,
    cfg: ControlConfig,
) -> Result<Exit, RunError> {
    let binding = lifecycle
        .identities()
        .load_binding(profile_id)
        .map_err(LifecycleError::from)?
        .ok_or(RunError::NotAdopted)?;
    let mut failures = 0u32;
    loop {
        let Some(state) = active_state(lifecycle, profile_id).await? else {
            return Ok(Exit::Stopped);
        };
        match Session::open(&binding.trust, profile_id, &binding.token, cfg.timeout).await {
            Ok(mut session) => {
                failures = 0;
                go_online(lifecycle, profile_id, state).await?;
                loop {
                    tokio::time::sleep(cfg.keepalive_interval).await;
                    if active_state(lifecycle, profile_id).await?.is_none() {
                        return Ok(Exit::Stopped);
                    }
                    if session.keepalive().await.is_err() {
                        break;
                    }
                }
                step(
                    lifecycle,
                    profile_id,
                    DeviceState::Online,
                    DeviceState::Reconnecting,
                    "keepalive_lost",
                )
                .await?;
            }
            Err(err) if err.is_pin_mismatch() => {
                step(
                    lifecycle,
                    profile_id,
                    state,
                    DeviceState::Error,
                    "controller_pin_mismatch",
                )
                .await?;
                return Ok(Exit::PinMismatch);
            }
            Err(_) => {
                failures += 1;
                if failures >= cfg.offline_after
                    && matches!(
                        state,
                        DeviceState::Reconnecting | DeviceState::Adopted | DeviceState::Online
                    )
                {
                    step(
                        lifecycle,
                        profile_id,
                        state,
                        DeviceState::Offline,
                        "controller_unreachable",
                    )
                    .await?;
                } else if state == DeviceState::Online {
                    step(
                        lifecycle,
                        profile_id,
                        state,
                        DeviceState::Reconnecting,
                        "controller_unreachable",
                    )
                    .await?;
                }
            }
        }
        tokio::time::sleep(cfg.retry_delay).await;
    }
}

/// Aktueller Zustand, falls der Kanal laufen darf; sonst `None`.
async fn active_state(
    lifecycle: &Lifecycle,
    profile_id: Uuid,
) -> Result<Option<DeviceState>, LifecycleError> {
    if lifecycle.kill_switch().await?.active {
        return Ok(None);
    }
    let state = lifecycle
        .db()
        .get_device_profile(profile_id)
        .await?
        .map(|p| p.state);
    Ok(state.filter(|s| {
        matches!(
            s,
            DeviceState::Adopted
                | DeviceState::Online
                | DeviceState::Reconnecting
                | DeviceState::Offline
        )
    }))
}

async fn go_online(
    lifecycle: &Lifecycle,
    profile_id: Uuid,
    state: DeviceState,
) -> Result<(), LifecycleError> {
    use DeviceState::*;
    match state {
        Adopted | Reconnecting => {
            step(lifecycle, profile_id, state, Online, "control_channel_up").await
        }
        // Nach Neustart: gespeicherter Zustand ist veraltet, Readback über reconnecting.
        Online | Offline => {
            step(
                lifecycle,
                profile_id,
                state,
                Reconnecting,
                "control_channel_resume",
            )
            .await?;
            step(
                lifecycle,
                profile_id,
                Reconnecting,
                Online,
                "control_channel_up",
            )
            .await
        }
        _ => Ok(()),
    }
}

/// Ein Übergang mit optimistischer Prüfung. Hat sich der Zustand inzwischen
/// geändert (z. B. deaktiviert) oder greift der Kill-Switch, ist das kein
/// Fehler: Die nächste Prüfung in `run` beendet den Kanal.
async fn step(
    lifecycle: &Lifecycle,
    profile_id: Uuid,
    from: DeviceState,
    to: DeviceState,
    reason: &str,
) -> Result<(), LifecycleError> {
    let req = TransitionRequest {
        to,
        expected_from: Some(from),
        reason,
        actor: "system:control",
        correlation_id: None,
    };
    match lifecycle.transition(profile_id, req).await {
        Ok(_)
        | Err(LifecycleError::Transition(
            TransitionError::Stale { .. } | TransitionError::KillSwitch,
        )) => Ok(()),
        Err(err) => Err(err),
    }
}
