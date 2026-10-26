//! Geräteprofile und expliziter Gerätezustandsautomat.
//!
//! `ai_key` und `ai_port` sind strikt getrennte Profile (ADR-003). Der
//! Automat kennt nur zulässige Übergänge; alles andere ist ein Fehler und
//! wird nie stillschweigend akzeptiert.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceProfileKind {
    AiKey,
    AiPort,
}

impl DeviceProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AiKey => "ai_key",
            Self::AiPort => "ai_port",
        }
    }
}

impl std::str::FromStr for DeviceProfileKind {
    type Err = UnknownValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ai_key" => Ok(Self::AiKey),
            "ai_port" => Ok(Self::AiPort),
            other => Err(UnknownValue(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    /// Profil existiert, aber es ist noch keine Discovery erfolgt.
    Unconfigured,
    Discovered,
    PendingAdoption,
    Adopted,
    Online,
    Reconnecting,
    Offline,
    Error,
    /// Bewusst deaktiviert (sicherer Rückweg).
    Disabled,
}

impl DeviceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconfigured => "unconfigured",
            Self::Discovered => "discovered",
            Self::PendingAdoption => "pending_adoption",
            Self::Adopted => "adopted",
            Self::Online => "online",
            Self::Reconnecting => "reconnecting",
            Self::Offline => "offline",
            Self::Error => "error",
            Self::Disabled => "disabled",
        }
    }

    /// Zulässige Folgezustände.
    pub fn allowed_next(self) -> &'static [DeviceState] {
        use DeviceState::*;
        match self {
            Unconfigured => &[Discovered, Disabled],
            Discovered => &[PendingAdoption, Offline, Error, Disabled],
            PendingAdoption => &[Adopted, Discovered, Error, Disabled],
            Adopted => &[Online, Offline, Error, Disabled],
            Online => &[Reconnecting, Offline, Error, Disabled],
            Reconnecting => &[Online, Offline, Error, Disabled],
            Offline => &[Reconnecting, Error, Disabled],
            // Aus Fehlern führt nur ein expliziter Neustart des Lifecycles.
            Error => &[Discovered, Offline, Disabled],
            Disabled => &[Unconfigured],
        }
    }

    pub fn transition(self, next: DeviceState) -> Result<DeviceState, InvalidTransition> {
        if self.allowed_next().contains(&next) {
            Ok(next)
        } else {
            Err(InvalidTransition {
                from: self,
                to: next,
            })
        }
    }
}

impl std::str::FromStr for DeviceState {
    type Err = UnknownValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DeviceState::*;
        [
            Unconfigured,
            Discovered,
            PendingAdoption,
            Adopted,
            Online,
            Reconnecting,
            Offline,
            Error,
            Disabled,
        ]
        .into_iter()
        .find(|state| state.as_str() == s)
        .ok_or_else(|| UnknownValue(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unzulässiger Zustandsübergang {from:?} -> {to:?}")]
pub struct InvalidTransition {
    pub from: DeviceState,
    pub to: DeviceState,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannter Wert: {0}")]
pub struct UnknownValue(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use DeviceState::*;

    #[test]
    fn happy_path_lifecycle() {
        let mut state = Unconfigured;
        for next in [
            Discovered,
            PendingAdoption,
            Adopted,
            Online,
            Reconnecting,
            Online,
        ] {
            state = state.transition(next).unwrap();
        }
        assert_eq!(state, Online);
    }

    #[test]
    fn cannot_skip_adoption() {
        assert!(Discovered.transition(Online).is_err());
        assert!(Unconfigured.transition(Adopted).is_err());
        assert!(PendingAdoption.transition(Online).is_err());
    }

    #[test]
    fn error_recovery_restarts_lifecycle() {
        assert!(Error.transition(Online).is_err());
        assert_eq!(Error.transition(Discovered).unwrap(), Discovered);
    }

    #[test]
    fn every_state_can_be_disabled_except_disabled() {
        for state in [
            Unconfigured,
            Discovered,
            PendingAdoption,
            Adopted,
            Online,
            Reconnecting,
            Offline,
            Error,
        ] {
            assert!(state.transition(Disabled).is_ok(), "{state:?}");
        }
        assert!(Disabled.transition(Disabled).is_err());
    }

    #[test]
    fn string_roundtrip() {
        for state in [Unconfigured, PendingAdoption, Reconnecting, Disabled] {
            assert_eq!(state.as_str().parse::<DeviceState>().unwrap(), state);
        }
        assert!("ready".parse::<DeviceState>().is_err());
        assert_eq!(
            "ai_port".parse::<DeviceProfileKind>().unwrap(),
            DeviceProfileKind::AiPort
        );
    }
}
