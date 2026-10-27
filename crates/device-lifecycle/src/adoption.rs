//! Adoption (AKL-M040-04) – Hypothese, Ziel `synthetic_verified`.
//!
//! Ablauf (eigene Modellierung; wie Protect adoptiert, ist offen, Q-006/Q-007):
//!
//! 1. Der Controller verbindet sich per TLS mit dem Adoptions-Endpunkt
//!    (berichtet: Port 8080, `needs_evidence`). Das Gerät weist sich mit dem
//!    **eigenen** Zertifikat aus der Identität aus.
//! 2. Nur der vom Admin bestätigte Controller (`ControllerTrust`: IP, Port,
//!    SHA-256-Fingerprint) wird angenommen; andere Absender werden ohne
//!    TLS-Handshake getrennt.
//! 3. `discovered → pending_adoption`. Das Gerät verbindet sich **gepinnt**
//!    zurück zum Controller und meldet sich mit dem erhaltenen Token an.
//!    Stimmt der Pin nicht (Bedrohung T1), wird das Token weder gesendet noch
//!    gespeichert; Rückweg `pending_adoption → discovered`.
//! 4. Erst nach bestätigtem Pin: Bindung speichern (Token nur als Datei mit
//!    Modus 0600 im Identitätsverzeichnis, nie DB/Log/API), `→ adopted`.
//!
//! Das Zeilenformat (`wire`) ist eine **synthetische Platzhalter-Rahmung** für
//! Tests gegen `akl-protect-sim`, nicht das Protect-Protokoll.

use std::{
    fmt, fs, io,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::Duration,
};

use akl_core::DeviceState;
use akl_persistence::devices::TransitionRequest;
use akl_protect_compat::tls::Fingerprint;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::{
    control,
    identity::{IdentityError, IdentityStore, sync_dir, write_file},
    service::{Lifecycle, LifecycleError},
};

/// Berichteter Port des Adoptions-Endpunkts (`needs_evidence`).
pub const ADOPTION_PORT: u16 = 8080;
/// Längste akzeptierte Anfragezeile.
pub(crate) const MAX_LINE: u64 = 4096;
const MAX_TOKEN: usize = 1024;

const BINDING: &str = "controller.json";
const TOKEN: &str = "controller.token";

/// Zugangsdaten vom Controller. `Debug` gibt sie nie aus.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Nur für die gepinnte Verbindung und die Secret-Datei.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// Vom Admin bestätigter Controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerTrust {
    pub ip: IpAddr,
    pub port: u16,
    pub fingerprint: Fingerprint,
}

/// Gespeicherte Adoption: vertrauter Controller + Token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerBinding {
    pub trust: ControllerTrust,
    pub token: Secret,
}

#[derive(Serialize, Deserialize)]
struct BindingMeta {
    ip: IpAddr,
    port: u16,
    fingerprint: String,
}

impl IdentityStore {
    /// Speichert die Bindung. Token zuerst (0600), `controller.json` zuletzt:
    /// erst dessen Vorhandensein macht die Bindung gültig.
    pub fn save_binding(
        &self,
        profile_id: Uuid,
        binding: &ControllerBinding,
    ) -> Result<(), IdentityError> {
        let dir = self.dir(profile_id);
        if !dir.is_dir() {
            return Err(IdentityError::Corrupt {
                profile_id,
                reason: "keine Identität für die Bindung".into(),
            });
        }
        let meta = BindingMeta {
            ip: binding.trust.ip,
            port: binding.trust.port,
            fingerprint: binding.trust.fingerprint.to_string(),
        };
        let json = serde_json::to_vec_pretty(&meta).map_err(io::Error::other)?;
        replace_file(&dir, TOKEN, binding.token.expose().as_bytes(), true)?;
        replace_file(&dir, BINDING, &json, false)?;
        Ok(())
    }

    pub fn load_binding(
        &self,
        profile_id: Uuid,
    ) -> Result<Option<ControllerBinding>, IdentityError> {
        let dir = self.dir(profile_id);
        let corrupt = |reason: String| IdentityError::Corrupt { profile_id, reason };
        let json = match fs::read(dir.join(BINDING)) {
            Ok(json) => json,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let meta: BindingMeta =
            serde_json::from_slice(&json).map_err(|e| corrupt(format!("{BINDING}: {e}")))?;
        let fingerprint = meta
            .fingerprint
            .parse()
            .map_err(|_| corrupt(format!("{BINDING}: Fingerprint ungültig")))?;
        let token =
            fs::read_to_string(dir.join(TOKEN)).map_err(|e| corrupt(format!("{TOKEN}: {e}")))?;
        Ok(Some(ControllerBinding {
            trust: ControllerTrust {
                ip: meta.ip,
                port: meta.port,
                fingerprint,
            },
            token: Secret(token),
        }))
    }

    /// Entfernt die Bindung (Rückweg „Gerät entfernen“); Identität bleibt.
    pub fn forget_binding(&self, profile_id: Uuid) -> Result<(), IdentityError> {
        let dir = self.dir(profile_id);
        for name in [BINDING, TOKEN] {
            match fs::remove_file(dir.join(name)) {
                Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err.into()),
                _ => {}
            }
        }
        if dir.is_dir() {
            sync_dir(&dir)?;
        }
        Ok(())
    }
}

/// Schreibt atomar über eine temporäre Datei im selben Verzeichnis.
fn replace_file(dir: &Path, name: &str, data: &[u8], private: bool) -> io::Result<()> {
    let tmp = dir.join(format!(".tmp-{}-{name}", Uuid::new_v4()));
    let result = write_file(&tmp, data, private).and_then(|()| fs::rename(&tmp, dir.join(name)));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    sync_dir(dir)
}

/// Synthetische Zeilenrahmung (JSON je Zeile) – Platzhalter, `needs_evidence`.
pub mod wire {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
    pub(crate) enum Request {
        Adopt { token: String },
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub struct Reply {
        pub ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<super::Refusal>,
    }

    /// Anfrage, wie sie der (simulierte) Controller sendet.
    pub fn adopt_request(token: &str) -> String {
        serde_json::json!({ "type": "adopt", "token": token }).to_string()
    }
}

/// Warum eine Adoption abgelehnt wurde (wird an den Controller gemeldet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    BadRequest,
    /// Kill-Switch aktiv oder Profil deaktiviert.
    Disabled,
    /// Profil ist nicht im Zustand `discovered`.
    WrongState,
    /// Rückverbindung: Controller-Zertifikat entspricht nicht dem Pin.
    ControllerUntrusted,
    ControllerUnreachable,
    Internal,
}

#[derive(Debug, thiserror::Error)]
pub enum AdoptionError {
    #[error("Adoptions-Endpunkt: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error("Profil hat keine Identität")]
    NoIdentity,
    #[error("TLS-Konfiguration: {0}")]
    Tls(#[from] rustls::Error),
}

/// Gebundener, noch nicht laufender Adoptions-Endpunkt für ein Profil.
pub struct AdoptionServer {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    lifecycle: Lifecycle,
    profile_id: Uuid,
    trust: ControllerTrust,
    timeout: Duration,
}

impl AdoptionServer {
    pub async fn bind(
        addr: SocketAddr,
        lifecycle: Lifecycle,
        profile_id: Uuid,
        trust: ControllerTrust,
        timeout: Duration,
    ) -> Result<Self, AdoptionError> {
        let profile = lifecycle
            .db()
            .get_device_profile(profile_id)
            .await
            .map_err(LifecycleError::from)?
            .ok_or(AdoptionError::NoIdentity)?;
        let identity = lifecycle
            .identities()
            .load(profile_id, profile.kind)
            .map_err(LifecycleError::from)?
            .ok_or(AdoptionError::NoIdentity)?;
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(vec![identity.cert_der()], identity.private_key_der())?;
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            acceptor: TlsAcceptor::from(Arc::new(config)),
            lifecycle,
            profile_id,
            trust,
            timeout,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, AdoptionError> {
        Ok(self.listener.local_addr()?)
    }

    /// Bearbeitet Verbindungen nacheinander: Adoption ist selten, und so kann
    /// es keine zwei gleichzeitigen Adoptionen geben. Jede Verbindung ist
    /// durch `timeout` (plus Rückverbindung) begrenzt.
    pub async fn run(self) -> Result<(), AdoptionError> {
        loop {
            let (tcp, peer) = self.listener.accept().await?;
            if peer.ip() != self.trust.ip {
                continue;
            }
            // Fehler einzelner Verbindungen beenden den Endpunkt nicht.
            let _ = self.handle(tcp).await;
        }
    }

    async fn handle(&self, tcp: TcpStream) -> io::Result<()> {
        let tls = tokio::time::timeout(self.timeout, self.acceptor.accept(tcp))
            .await
            .map_err(|_| io::ErrorKind::TimedOut)??;
        let (read, mut write) = tokio::io::split(tls);
        let mut line = String::new();
        let mut reader = BufReader::new(read.take(MAX_LINE));
        tokio::time::timeout(self.timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| io::ErrorKind::TimedOut)??;
        let reply = match self.adopt(&line).await {
            Ok(()) => wire::Reply {
                ok: true,
                error: None,
            },
            Err(refusal) => wire::Reply {
                ok: false,
                error: Some(refusal),
            },
        };
        let mut out = serde_json::to_vec(&reply).map_err(io::Error::other)?;
        out.push(b'\n');
        write.write_all(&out).await?;
        write.shutdown().await
    }

    async fn adopt(&self, line: &str) -> Result<(), Refusal> {
        let wire::Request::Adopt { token } =
            serde_json::from_str(line.trim_end()).map_err(|_| Refusal::BadRequest)?;
        if token.is_empty() || token.len() > MAX_TOKEN || token.chars().any(char::is_control) {
            return Err(Refusal::BadRequest);
        }
        let token = Secret(token);

        if self.lifecycle.kill_switch().await.map_err(internal)?.active {
            return Err(Refusal::Disabled);
        }
        let state = self.state().await?;
        match state {
            DeviceState::Discovered => {}
            DeviceState::Disabled => return Err(Refusal::Disabled),
            _ => return Err(Refusal::WrongState),
        }
        self.step(
            DeviceState::Discovered,
            DeviceState::PendingAdoption,
            "adoption_requested",
        )
        .await?;

        // Pin vor jeder Übergabe des Tokens prüfen (T1).
        if let Err(err) =
            control::Session::open(&self.trust, self.profile_id, &token, self.timeout).await
        {
            self.rollback().await;
            return Err(if err.is_pin_mismatch() {
                Refusal::ControllerUntrusted
            } else {
                Refusal::ControllerUnreachable
            });
        }

        let binding = ControllerBinding {
            trust: self.trust,
            token,
        };
        if self
            .lifecycle
            .identities()
            .save_binding(self.profile_id, &binding)
            .is_err()
        {
            self.rollback().await;
            return Err(Refusal::Internal);
        }
        if let Err(refusal) = self
            .step(
                DeviceState::PendingAdoption,
                DeviceState::Adopted,
                "adoption_confirmed",
            )
            .await
        {
            // Ohne `adopted` keine gespeicherten Zugangsdaten.
            let _ = self.lifecycle.identities().forget_binding(self.profile_id);
            self.rollback().await;
            return Err(refusal);
        }
        Ok(())
    }

    async fn state(&self) -> Result<DeviceState, Refusal> {
        Ok(self
            .lifecycle
            .db()
            .get_device_profile(self.profile_id)
            .await
            .map_err(|_| Refusal::Internal)?
            .ok_or(Refusal::Internal)?
            .state)
    }

    async fn step(&self, from: DeviceState, to: DeviceState, reason: &str) -> Result<(), Refusal> {
        let req = TransitionRequest {
            to,
            expected_from: Some(from),
            reason,
            actor: "system:adoption",
            correlation_id: None,
        };
        self.lifecycle
            .transition(self.profile_id, req)
            .await
            .map(drop)
            .map_err(|err| match err {
                LifecycleError::Transition(
                    akl_persistence::devices::TransitionError::KillSwitch,
                ) => Refusal::Disabled,
                _ => Refusal::WrongState,
            })
    }

    /// `pending_adoption → discovered`; scheitert still, wenn inzwischen z. B.
    /// der Kill-Switch das Profil deaktiviert hat.
    async fn rollback(&self) {
        let _ = self
            .step(
                DeviceState::PendingAdoption,
                DeviceState::Discovered,
                "adoption_failed",
            )
            .await;
    }
}

fn internal(_: LifecycleError) -> Refusal {
    Refusal::Internal
}
