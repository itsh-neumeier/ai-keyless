//! Konfiguration ausschließlich über Umgebungsvariablen und Secret-Dateien.

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use akl_device_lifecycle::adoption::{ADOPTION_PORT, ControllerTrust};
use akl_protect_compat::discovery::DISCOVERY_PORT;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{name} ist ungültig: {reason}")]
    Invalid { name: &'static str, reason: String },
}

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    /// Verbindungs-URL; enthält ggf. ein Passwort und wird deshalb nie geloggt.
    pub database_url: Option<String>,
    pub web_dir: Option<PathBuf>,
    pub log_json: bool,
    pub cookie_secure: bool,
    pub provider_allow_loopback: bool,
    pub provider_allow_private: bool,
    pub provider_allowed_hosts: Option<Vec<String>>,
    pub provider_timeout_secs: u64,
    pub secrets_dir: PathBuf,
    /// Identitätsverzeichnis der Geräteprofile (privat, beschreibbar).
    pub device_dir: Option<PathBuf>,
    /// Erzwingt den Geräte-Kill-Switch (`AKL_DEVICE_KILL_SWITCH`).
    pub device_kill_switch: bool,
    /// Geräte-Emulation (`AKL_DEVICE_EMULATION`), standardmäßig aus.
    pub device_emulation: Option<DeviceEmulation>,
    pub jobs_per_hour: i64,
}

/// Opt-in-Einstellungen der Geräte-Emulation (AKL-M040-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEmulation {
    pub discovery_bind: SocketAddr,
    pub adoption_bind: SocketAddr,
    /// Nur vollständig (IP, Port, Fingerprint) oder gar nicht.
    pub controller: Option<ControllerTrust>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field(
                "database_url",
                &self.database_url.as_ref().map(|_| "<redacted>"),
            )
            .field("web_dir", &self.web_dir)
            .field("log_json", &self.log_json)
            .field("cookie_secure", &self.cookie_secure)
            .field("provider_allow_loopback", &self.provider_allow_loopback)
            .field("provider_allow_private", &self.provider_allow_private)
            .field("provider_allowed_hosts", &self.provider_allowed_hosts)
            .field("provider_timeout_secs", &self.provider_timeout_secs)
            .field("secrets_dir", &self.secrets_dir)
            .field("device_dir", &self.device_dir)
            .field("device_kill_switch", &self.device_kill_switch)
            .field("device_emulation", &self.device_emulation)
            .field("jobs_per_hour", &self.jobs_per_hour)
            .finish()
    }
}

pub const DEFAULT_BIND: &str = "0.0.0.0:8090";

fn addr(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: SocketAddr,
) -> Result<SocketAddr, ConfigError> {
    get(name).map_or(Ok(default), |v| {
        v.parse()
            .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
                name,
                reason: e.to_string(),
            })
    })
}

fn device_emulation(
    get: &impl Fn(&str) -> Option<String>,
) -> Result<Option<DeviceEmulation>, ConfigError> {
    if !flag(get, "AKL_DEVICE_EMULATION", false)? {
        return Ok(None);
    }
    // Compose reicht nicht gesetzte Variablen als leere Strings durch.
    let get = |name: &str| get(name).filter(|v| !v.trim().is_empty());
    let invalid = |name, reason: String| ConfigError::Invalid { name, reason };
    let ip = get("AKL_CONTROLLER_IP")
        .map(|v| {
            v.parse::<IpAddr>()
                .map_err(|e| invalid("AKL_CONTROLLER_IP", e.to_string()))
        })
        .transpose()?;
    let port = get("AKL_CONTROLLER_PORT")
        .map(|v| {
            v.parse::<u16>().ok().filter(|p| *p > 0).ok_or_else(|| {
                invalid(
                    "AKL_CONTROLLER_PORT",
                    format!("1–65535 erwartet, erhalten: {v}"),
                )
            })
        })
        .transpose()?;
    let fingerprint = get("AKL_CONTROLLER_FINGERPRINT")
        .map(|v| {
            v.parse().map_err(|_| {
                invalid(
                    "AKL_CONTROLLER_FINGERPRINT",
                    "SHA-256 (64 Hex-Zeichen) erwartet".into(),
                )
            })
        })
        .transpose()?;
    // Halbe Controller-Angaben sind ein Konfigurationsfehler, kein stilles Weglassen.
    let controller = match (ip, port, fingerprint) {
        (Some(ip), Some(port), Some(fingerprint)) => Some(ControllerTrust {
            ip,
            port,
            fingerprint,
        }),
        (None, None, None) => None,
        _ => {
            return Err(invalid(
                "AKL_CONTROLLER_*",
                "IP, Port und Fingerprint nur gemeinsam angeben".into(),
            ));
        }
    };
    let any = |port| SocketAddr::new(IpAddr::from([0, 0, 0, 0]), port);
    Ok(Some(DeviceEmulation {
        discovery_bind: addr(&get, "AKL_DISCOVERY_BIND", any(DISCOVERY_PORT))?,
        adoption_bind: addr(&get, "AKL_ADOPTION_BIND", any(ADOPTION_PORT))?,
        controller,
    }))
}

fn flag(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: bool,
) -> Result<bool, ConfigError> {
    match get(name).as_deref() {
        None => Ok(default),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(other) => Err(ConfigError::Invalid {
            name,
            reason: format!("erwartet true/false, erhalten: {other}"),
        }),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let bind = get("AKL_BIND")
            .unwrap_or_else(|| DEFAULT_BIND.to_owned())
            .parse()
            .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
                name: "AKL_BIND",
                reason: e.to_string(),
            })?;

        // `_FILE` hat Vorrang (Docker-Secrets).
        let database_url = match get("AKL_DATABASE_URL_FILE") {
            Some(path) => Some(
                std::fs::read_to_string(&path)
                    .map_err(|e| ConfigError::Invalid {
                        name: "AKL_DATABASE_URL_FILE",
                        reason: e.to_string(),
                    })?
                    .trim()
                    .to_owned(),
            ),
            None => get("AKL_DATABASE_URL"),
        }
        .filter(|url| !url.is_empty());

        Ok(Self {
            bind,
            database_url,
            web_dir: get("AKL_WEB_DIR").map(PathBuf::from),
            log_json: get("AKL_LOG_FORMAT").is_some_and(|v| v == "json"),
            cookie_secure: flag(&get, "AKL_COOKIE_SECURE", false)?,
            provider_allow_loopback: flag(&get, "AKL_PROVIDER_ALLOW_LOOPBACK", false)?,
            provider_allow_private: flag(&get, "AKL_PROVIDER_ALLOW_PRIVATE", true)?,
            // Leer (z. B. aus Compose-Vorgabe) bedeutet: keine Einschränkung.
            provider_allowed_hosts: get("AKL_PROVIDER_ALLOWED_HOSTS")
                .map(|v| {
                    v.split(',')
                        .map(|h| h.trim().to_ascii_lowercase())
                        .filter(|h| !h.is_empty())
                        .collect::<Vec<_>>()
                })
                .filter(|hosts| !hosts.is_empty()),
            provider_timeout_secs: match get("AKL_PROVIDER_TIMEOUT_SECS") {
                None => 120,
                Some(v) => v.parse().ok().filter(|s| (1..=3600).contains(s)).ok_or(
                    ConfigError::Invalid {
                        name: "AKL_PROVIDER_TIMEOUT_SECS",
                        reason: format!("1–3600 erwartet, erhalten: {v}"),
                    },
                )?,
            },
            secrets_dir: get("AKL_SECRETS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/run/secrets")),
            device_dir: get("AKL_DEVICE_DIR").map(PathBuf::from),
            device_kill_switch: flag(&get, "AKL_DEVICE_KILL_SWITCH", false)?,
            device_emulation: device_emulation(&get)?,
            jobs_per_hour: match get("AKL_JOBS_PER_HOUR") {
                None => crate::DEFAULT_JOBS_PER_HOUR,
                Some(v) => v.parse().ok().filter(|n| (0..=100_000).contains(n)).ok_or(
                    ConfigError::Invalid {
                        name: "AKL_JOBS_PER_HOUR",
                        reason: format!("0–100000 erwartet, erhalten: {v}"),
                    },
                )?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn defaults() {
        let c = cfg(&[]).unwrap();
        assert_eq!(c.bind.to_string(), DEFAULT_BIND);
        assert!(c.database_url.is_none());
    }

    #[test]
    fn cookie_secure_flag() {
        assert!(!cfg(&[]).unwrap().cookie_secure);
        assert!(cfg(&[("AKL_COOKIE_SECURE", "true")]).unwrap().cookie_secure);
        assert!(cfg(&[("AKL_COOKIE_SECURE", "ja")]).is_err());
    }

    #[test]
    fn provider_policy_settings() {
        let c = cfg(&[]).unwrap();
        assert!(!c.provider_allow_loopback && c.provider_allow_private);
        assert!(c.provider_allowed_hosts.is_none());
        let c = cfg(&[("AKL_PROVIDER_ALLOWED_HOSTS", "Ollama.lan, ,lm.lan")]).unwrap();
        assert_eq!(
            c.provider_allowed_hosts.unwrap(),
            vec!["ollama.lan", "lm.lan"]
        );
        assert!(cfg(&[("AKL_PROVIDER_TIMEOUT_SECS", "0")]).is_err());
        assert!(cfg(&[("AKL_PROVIDER_ALLOW_PRIVATE", "vielleicht")]).is_err());
        assert_eq!(cfg(&[]).unwrap().jobs_per_hour, 12);
        assert_eq!(cfg(&[("AKL_JOBS_PER_HOUR", "0")]).unwrap().jobs_per_hour, 0);
        assert!(cfg(&[("AKL_JOBS_PER_HOUR", "-1")]).is_err());
    }

    #[test]
    fn device_emulation_is_opt_in_and_controller_complete() {
        assert_eq!(cfg(&[]).unwrap().device_emulation, None);
        // Controller-Angaben ohne Opt-in bleiben wirkungslos.
        assert_eq!(
            cfg(&[("AKL_CONTROLLER_IP", "192.168.10.181")])
                .unwrap()
                .device_emulation,
            None
        );
        let only_discovery = cfg(&[("AKL_DEVICE_EMULATION", "true")])
            .unwrap()
            .device_emulation
            .unwrap();
        assert_eq!(only_discovery.discovery_bind.to_string(), "0.0.0.0:10001");
        assert_eq!(only_discovery.adoption_bind.to_string(), "0.0.0.0:8080");
        assert_eq!(only_discovery.controller, None);
        // Leere Werte (Compose-Vorgabe) gelten als nicht gesetzt.
        let compose = cfg(&[
            ("AKL_DEVICE_EMULATION", "true"),
            ("AKL_CONTROLLER_IP", ""),
            ("AKL_CONTROLLER_PORT", ""),
            ("AKL_CONTROLLER_FINGERPRINT", ""),
        ])
        .unwrap()
        .device_emulation
        .unwrap();
        assert_eq!(compose.controller, None);

        let fp = "AB".repeat(32);
        let full = cfg(&[
            ("AKL_DEVICE_EMULATION", "true"),
            ("AKL_CONTROLLER_IP", "192.168.10.181"),
            ("AKL_CONTROLLER_PORT", "7442"),
            ("AKL_CONTROLLER_FINGERPRINT", &fp),
            ("AKL_DISCOVERY_BIND", "192.168.10.50:10001"),
        ])
        .unwrap()
        .device_emulation
        .unwrap();
        let trust = full.controller.unwrap();
        assert_eq!(trust.ip.to_string(), "192.168.10.181");
        assert_eq!(trust.port, 7442);
        assert_eq!(full.discovery_bind.to_string(), "192.168.10.50:10001");

        for bad in [
            vec![("AKL_CONTROLLER_IP", "192.168.10.181")],
            vec![("AKL_CONTROLLER_IP", "unvr.lan")],
            vec![("AKL_CONTROLLER_PORT", "0")],
            vec![("AKL_CONTROLLER_FINGERPRINT", "AB:CD")],
            vec![("AKL_DISCOVERY_BIND", "10001")],
        ] {
            let mut vars = vec![("AKL_DEVICE_EMULATION", "true")];
            vars.extend(bad.iter().copied());
            assert!(cfg(&vars).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn rejects_bad_bind() {
        assert!(cfg(&[("AKL_BIND", "nope")]).is_err());
    }

    #[test]
    fn debug_redacts_database_url() {
        let c = cfg(&[("AKL_DATABASE_URL", "postgres://u:geheim@db/akl")]).unwrap();
        let printed = format!("{c:?}");
        assert!(!printed.contains("geheim"));
        assert!(printed.contains("<redacted>"));
    }

    #[test]
    fn missing_secret_file_is_error() {
        assert!(cfg(&[("AKL_DATABASE_URL_FILE", "/nonexistent/akl-secret")]).is_err());
    }
}
