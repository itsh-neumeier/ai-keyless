//! Modell-Provider (AKL-M060-02/05/06): Ollama und OpenAI-kompatible Server.
//!
//! Jede Anfrage läuft über die [`policy::NetworkPolicy`], folgt keinen
//! Weiterleitungen, ignoriert System-Proxys und begrenzt Zeit und Antwortgröße.
//! Fähigkeiten eines Modells werden nie angenommen, sondern per Testaufruf belegt.

pub mod policy;

use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use policy::NetworkPolicy;

/// Eigenes, synthetisches Testbild (64×64, einfarbig rot), keine Fremddaten.
pub const SYNTHETIC_TEST_IMAGE_PNG: &[u8] = include_bytes!("../assets/synthetic-red-64.png");
pub const SYNTHETIC_TEST_PROMPT: &str =
    "Which single color fills this image? Answer with one English word.";
/// Erwartete Antwort auf [`SYNTHETIC_TEST_PROMPT`].
pub const SYNTHETIC_TEST_EXPECTED: &str = "red";

/// Liest die Modellnamen aus der providerspezifischen Antwort.
type ModelExtractor = fn(&Value) -> Option<Vec<String>>;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Ollama-API (`/api/tags`, `/api/chat`).
    Ollama,
    /// OpenAI-kompatible API (`/models`, `/chat/completions`), z. B. vLLM,
    /// LM Studio, llama.cpp-Server. Die Basis-URL enthält üblicherweise `/v1`.
    OpenaiCompatible,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::OpenaiCompatible => "openai_compatible",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unbekannter Provider-Typ: {0}")]
pub struct UnknownKind(pub String);

impl std::str::FromStr for ProviderKind {
    type Err = UnknownKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ollama" => Ok(Self::Ollama),
            "openai_compatible" => Ok(Self::OpenaiCompatible),
            other => Err(UnknownKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    #[error("ungültige Provider-URL")]
    InvalidUrl,
    #[error("Zieladresse durch Netzwerkrichtlinie gesperrt")]
    BlockedAddress,
    #[error("Hostname nicht auflösbar")]
    Dns,
    #[error("Verbindung fehlgeschlagen")]
    Connect,
    #[error("Zeitüberschreitung")]
    Timeout,
    #[error("Weiterleitung blockiert")]
    RedirectBlocked,
    #[error("HTTP-Status {0}")]
    HttpStatus(u16),
    #[error("Antwort zu groß")]
    TooLarge,
    #[error("unerwartetes Antwortformat")]
    InvalidResponse,
}

impl ProviderError {
    /// Stabiler Fehlercode für API und UI.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "invalid_url",
            Self::BlockedAddress => "blocked_address",
            Self::Dns => "dns_failed",
            Self::Connect => "connect_failed",
            Self::Timeout => "timeout",
            Self::RedirectBlocked => "redirect_blocked",
            Self::HttpStatus(401 | 403) => "provider_auth_failed",
            Self::HttpStatus(_) => "provider_http_error",
            Self::TooLarge => "response_too_large",
            Self::InvalidResponse => "invalid_response",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: String,
    /// Klartext nur im Speicher; stammt aus einer Secret-Datei.
    pub api_key: Option<String>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelList {
    pub models: Vec<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisionResult {
    pub text: String,
    pub latency_ms: u64,
}

pub struct Provider {
    config: ProviderConfig,
    policy: NetworkPolicy,
}

impl Provider {
    pub fn new(config: ProviderConfig, policy: NetworkPolicy) -> Self {
        Self { config, policy }
    }

    /// Verfügbare Modelle (Verbindungstest, verursacht keine Inferenz).
    pub async fn list_models(&self) -> Result<ModelList, ProviderError> {
        let start = Instant::now();
        let (path, extract): (&str, ModelExtractor) = match self.config.kind {
            ProviderKind::Ollama => ("api/tags", |v| {
                v.get("models")?
                    .as_array()?
                    .iter()
                    .map(|m| m.get("name")?.as_str().map(str::to_owned))
                    .collect()
            }),
            ProviderKind::OpenaiCompatible => ("models", |v| {
                v.get("data")?
                    .as_array()?
                    .iter()
                    .map(|m| m.get("id")?.as_str().map(str::to_owned))
                    .collect()
            }),
        };
        let body = self.request(reqwest::Method::GET, path, None).await?;
        let mut models = extract(&body).ok_or(ProviderError::InvalidResponse)?;
        models.sort();
        Ok(ModelList {
            models,
            latency_ms: elapsed_ms(start),
        })
    }

    /// Bildbeschreibung für ein PNG/JPEG. Das Bild wird nur im Speicher gehalten
    /// und nicht protokolliert.
    pub async fn describe_image(
        &self,
        model: &str,
        prompt: &str,
        image: &[u8],
    ) -> Result<VisionResult, ProviderError> {
        let start = Instant::now();
        let b64 = STANDARD.encode(image);
        let text = match self.config.kind {
            ProviderKind::Ollama => {
                let body = json!({
                    "model": model,
                    "stream": false,
                    "messages": [{ "role": "user", "content": prompt, "images": [b64] }],
                });
                let v = self
                    .request(reqwest::Method::POST, "api/chat", Some(body))
                    .await?;
                v.pointer("/message/content")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }
            ProviderKind::OpenaiCompatible => {
                let mime = if image.starts_with(b"\x89PNG") {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                let body = json!({
                    "model": model,
                    "max_tokens": 300,
                    "messages": [{
                        "role": "user",
                        "content": [
                            { "type": "text", "text": prompt },
                            { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } },
                        ],
                    }],
                });
                let v = self
                    .request(reqwest::Method::POST, "chat/completions", Some(body))
                    .await?;
                v.pointer("/choices/0/message/content")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }
        };
        let text = text
            .ok_or(ProviderError::InvalidResponse)?
            .trim()
            .to_owned();
        Ok(VisionResult {
            text,
            latency_ms: elapsed_ms(start),
        })
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, ProviderError> {
        let target = policy::resolve(&self.config.base_url, &self.policy).await?;
        let mut base = target.url.clone();
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let url = base.join(path).map_err(|_| ProviderError::InvalidUrl)?;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(self.config.timeout)
            // Verbindung nur zu den geprüften Adressen (kein erneutes DNS).
            .resolve_to_addrs(&target.host, &target.addrs)
            .build()
            .map_err(|_| ProviderError::Connect)?;

        let mut req = client
            .request(method, url)
            .header("accept", "application/json");
        if let Some(key) = &self.config.api_key {
            req = req.bearer_auth(key);
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let mut res = req.send().await.map_err(map_reqwest)?;
        let status = res.status();
        if status.is_redirection() {
            return Err(ProviderError::RedirectBlocked);
        }
        if !status.is_success() {
            return Err(ProviderError::HttpStatus(status.as_u16()));
        }
        if res
            .content_length()
            .is_some_and(|len| len as usize > MAX_RESPONSE_BYTES)
        {
            return Err(ProviderError::TooLarge);
        }
        let mut buf = Vec::new();
        while let Some(chunk) = res.chunk().await.map_err(map_reqwest)? {
            if buf.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(ProviderError::TooLarge);
            }
            buf.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&buf).map_err(|_| ProviderError::InvalidResponse)
    }
}

fn map_reqwest(err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        ProviderError::Timeout
    } else {
        tracing::debug!(error = %err, "Provider-Anfrage fehlgeschlagen");
        ProviderError::Connect
    }
}

fn elapsed_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Prüft eine Antwort auf das synthetische Testbild (tolerant gegenüber
/// Groß-/Kleinschreibung und Satzzeichen).
pub fn synthetic_answer_matches(text: &str) -> bool {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word == SYNTHETIC_TEST_EXPECTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_image_is_png() {
        assert!(SYNTHETIC_TEST_IMAGE_PNG.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn answer_matching() {
        assert!(synthetic_answer_matches("Red."));
        assert!(synthetic_answer_matches("The color is red"));
        assert!(!synthetic_answer_matches("reddish"));
        assert!(!synthetic_answer_matches("blue"));
    }

    #[test]
    fn error_codes() {
        assert_eq!(
            ProviderError::HttpStatus(401).code(),
            "provider_auth_failed"
        );
        assert_eq!(ProviderError::HttpStatus(500).code(), "provider_http_error");
        assert_eq!(
            "ollama".parse::<ProviderKind>().unwrap(),
            ProviderKind::Ollama
        );
    }
}
