//! Zertifikats-Pinning für Controller-Verbindungen (Bedrohung T1).
//!
//! Protect-Konsolen nutzen in der Regel selbstsignierte Zertifikate. Statt einer
//! CA-Kette vertraut AI-Keyless genau einem Zertifikat, dessen SHA-256-Fingerprint
//! ein Admin einmalig bestätigt hat (Trust on first use mit menschlicher Prüfung).
//! Zugangsdaten dürfen erst über eine so gepinnte Verbindung gesendet werden.

use std::{fmt, str::FromStr, sync::Arc, time::Duration};

use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

/// SHA-256 über das DER-kodierte Endzertifikat.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint(pub [u8; 32]);

impl Fingerprint {
    pub fn of(cert_der: &[u8]) -> Self {
        Self(Sha256::digest(cert_der).into())
    }
}

impl fmt::Display for Fingerprint {
    /// Format wie in Browsern/OpenSSL: `AB:CD:…` (Großbuchstaben, Doppelpunkte).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(|b| format!("{b:02X}")).collect();
        f.write_str(&parts.join(":"))
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({self})")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ungültiger SHA-256-Fingerprint")]
pub struct InvalidFingerprint;

impl FromStr for Fingerprint {
    type Err = InvalidFingerprint;

    /// Akzeptiert Hex mit oder ohne `:`/Leerzeichen, Groß-/Kleinschreibung egal.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex: String = s
            .chars()
            .filter(|c| !matches!(c, ':' | ' ' | '-'))
            .collect();
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(InvalidFingerprint);
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte =
                u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| InvalidFingerprint)?;
        }
        Ok(Self(out))
    }
}

/// Wie wird das Serverzertifikat geprüft?
#[derive(Debug, Clone, Copy)]
enum Mode {
    /// Nur dieses Zertifikat ist gültig.
    Pinned(Fingerprint),
    /// Nur zum Auslesen des Fingerprints für die Erstbestätigung; über eine
    /// solche Verbindung werden niemals Daten gesendet.
    Inspect,
}

#[derive(Debug)]
struct PinVerifier {
    mode: Mode,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match self.mode {
            Mode::Inspect => Ok(ServerCertVerified::assertion()),
            Mode::Pinned(pin) if Fingerprint::of(end_entity.as_ref()) == pin => {
                Ok(ServerCertVerified::assertion())
            }
            Mode::Pinned(_) => Err(rustls::Error::General(
                "Zertifikat entspricht nicht dem Pin".into(),
            )),
        }
    }

    // Handshake-Signaturen werden echt geprüft: Der Server muss den privaten
    // Schlüssel zum gepinnten Zertifikat besitzen.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn client_config(mode: Mode) -> rustls::ClientConfig {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("Standardprotokolle")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinVerifier { mode, provider }))
        .with_no_client_auth()
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Zeitüberschreitung")]
    Timeout,
    #[error("Verbindung fehlgeschlagen: {0}")]
    Io(#[from] std::io::Error),
    #[error("ungültiger Hostname")]
    InvalidHost,
    #[error("kein Serverzertifikat erhalten")]
    NoCertificate,
}

impl ConnectError {
    /// Pin-Abweichung (vom TLS-Handshake gemeldet)?
    pub fn is_pin_mismatch(&self) -> bool {
        matches!(self, Self::Io(e) if e.to_string().contains("Pin"))
    }
}

async fn handshake(
    host: &str,
    port: u16,
    mode: Mode,
    timeout: Duration,
) -> Result<TlsStream<TcpStream>, ConnectError> {
    let name = ServerName::try_from(host.to_owned()).map_err(|_| ConnectError::InvalidHost)?;
    let connector = TlsConnector::from(Arc::new(client_config(mode)));
    tokio::time::timeout(timeout, async {
        let tcp = TcpStream::connect((host, port)).await?;
        connector.connect(name, tcp).await
    })
    .await
    .map_err(|_| ConnectError::Timeout)?
    .map_err(ConnectError::Io)
}

/// Liest den Fingerprint des Controllers aus, damit ein Admin ihn mit dem in der
/// Protect-Oberfläche/Konsole angezeigten Wert vergleichen kann. Sendet keine Daten.
pub async fn inspect_fingerprint(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<Fingerprint, ConnectError> {
    let stream = handshake(host, port, Mode::Inspect, timeout).await?;
    let (_, conn) = stream.get_ref();
    let cert = conn
        .peer_certificates()
        .and_then(|c| c.first())
        .ok_or(ConnectError::NoCertificate)?;
    Ok(Fingerprint::of(cert.as_ref()))
}

/// Baut eine TLS-Verbindung auf, die nur mit dem gepinnten Zertifikat zustande
/// kommt. Erst über diese Verbindung dürfen Zugangsdaten gesendet werden.
pub async fn connect_pinned(
    host: &str,
    port: u16,
    pin: Fingerprint,
    timeout: Duration,
) -> Result<TlsStream<TcpStream>, ConnectError> {
    handshake(host, port, Mode::Pinned(pin), timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_roundtrip_and_formats() {
        let fp = Fingerprint::of(b"zertifikat");
        let text = fp.to_string();
        assert_eq!(text.len(), 32 * 3 - 1);
        assert_eq!(text.parse::<Fingerprint>().unwrap(), fp);
        let compact = text.replace(':', "").to_lowercase();
        assert_eq!(compact.parse::<Fingerprint>().unwrap(), fp);
        for bad in ["", "AB:CD", &"Z".repeat(64), &"a".repeat(66)] {
            assert!(bad.parse::<Fingerprint>().is_err(), "{bad}");
        }
    }
}
