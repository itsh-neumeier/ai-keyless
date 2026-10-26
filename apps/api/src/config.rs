//! Konfiguration ausschließlich über Umgebungsvariablen und Secret-Dateien.

use std::{net::SocketAddr, path::PathBuf};

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
            .finish()
    }
}

pub const DEFAULT_BIND: &str = "0.0.0.0:8090";

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
