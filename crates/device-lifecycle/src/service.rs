//! Lifecycle-Service (AKL-M040-02): einziger Weg, Geräteprofile anzulegen und
//! Zustände zu ändern.
//!
//! - Identität wird **vor** dem Profil angelegt; scheitert die Anlage, wird sie
//!   wieder entfernt. Es gibt nie ein Profil ohne Identität.
//! - Übergänge prüft der Zustandsautomat (`akl-core`) in der Datenbank-
//!   Transaktion (`akl-persistence::devices`).
//! - Kill-Switch: gespeichert in der Datenbank oder per `AKL_DEVICE_KILL_SWITCH`
//!   erzwungen. Aktiv → alle Profile deaktiviert, nur `→ disabled` erlaubt,
//!   keine neuen Profile. Erzwungen → per API nicht abschaltbar.

use akl_core::{DeviceProfileKind, DeviceState};
use akl_persistence::{
    Database, PersistenceError,
    devices::{DeviceProfileRecord, TransitionError, TransitionRequest},
};
use akl_protect_compat::tls::Fingerprint;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::identity::{IdentityError, IdentityStore};

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Transition(#[from] TransitionError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("es gibt bereits ein aktives ai_key-Profil")]
    ActiveAiKeyExists,
    #[error("Kill-Switch aktiv")]
    KillSwitchActive,
    #[error("Kill-Switch ist per AKL_DEVICE_KILL_SWITCH erzwungen")]
    KillSwitchForced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KillSwitchStatus {
    pub active: bool,
    /// Per Umgebungsvariable erzwungen (nicht per API abschaltbar).
    pub forced: bool,
    pub changed_at: Option<DateTime<Utc>>,
    pub changed_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Lifecycle {
    db: Database,
    identities: IdentityStore,
    forced_kill_switch: bool,
}

impl Lifecycle {
    pub fn new(db: Database, identities: IdentityStore, forced_kill_switch: bool) -> Self {
        Self {
            db,
            identities,
            forced_kill_switch,
        }
    }

    pub async fn kill_switch(&self) -> Result<KillSwitchStatus, LifecycleError> {
        let stored = self.db.kill_switch().await?;
        Ok(KillSwitchStatus {
            active: self.forced_kill_switch || stored.active,
            forced: self.forced_kill_switch,
            changed_at: stored.changed_at,
            changed_by: stored.changed_by,
        })
    }

    /// Bei erzwungenem Kill-Switch: gespeicherten Zustand angleichen, damit
    /// alle Profile tatsächlich deaktiviert sind (beim Start aufrufen).
    pub async fn enforce_forced_kill_switch(
        &self,
    ) -> Result<Vec<DeviceProfileRecord>, LifecycleError> {
        if !self.forced_kill_switch {
            return Ok(Vec::new());
        }
        Ok(self
            .db
            .set_kill_switch(true, "system:AKL_DEVICE_KILL_SWITCH", None)
            .await?)
    }

    pub async fn set_kill_switch(
        &self,
        active: bool,
        actor: &str,
        correlation_id: Option<&str>,
    ) -> Result<Vec<DeviceProfileRecord>, LifecycleError> {
        if !active && self.forced_kill_switch {
            return Err(LifecycleError::KillSwitchForced);
        }
        Ok(self
            .db
            .set_kill_switch(active, actor, correlation_id)
            .await?)
    }

    /// Legt Identität und Profil (Zustand `unconfigured`) an.
    pub async fn create_profile(
        &self,
        kind: DeviceProfileKind,
        display_name: &str,
    ) -> Result<(DeviceProfileRecord, Fingerprint), LifecycleError> {
        if self.kill_switch().await?.active {
            return Err(LifecycleError::KillSwitchActive);
        }
        let id = Uuid::new_v4();
        let identity = self.identities.load_or_create(id, kind)?.identity;
        match self.db.create_device_profile(id, kind, display_name).await {
            Ok(Some(profile)) => Ok((profile, identity.fingerprint)),
            Ok(None) => {
                self.identities.remove(id)?;
                Err(LifecycleError::ActiveAiKeyExists)
            }
            Err(err) => {
                self.identities.remove(id)?;
                Err(err.into())
            }
        }
    }

    /// Öffentlicher Fingerprint der Identität eines Profils (`None`, falls keine).
    pub fn fingerprint(
        &self,
        profile: &DeviceProfileRecord,
    ) -> Result<Option<Fingerprint>, LifecycleError> {
        Ok(self
            .identities
            .load(profile.id, profile.kind)?
            .map(|identity| identity.fingerprint))
    }

    pub async fn transition(
        &self,
        id: Uuid,
        req: TransitionRequest<'_>,
    ) -> Result<DeviceProfileRecord, LifecycleError> {
        if self.forced_kill_switch && req.to != DeviceState::Disabled {
            return Err(LifecycleError::Transition(TransitionError::KillSwitch));
        }
        Ok(self.db.transition_device(id, req).await?)
    }

    pub fn db(&self) -> &Database {
        &self.db
    }
}
