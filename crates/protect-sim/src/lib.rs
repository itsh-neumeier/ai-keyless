//! Simulierter Controller (AKL-M030-02) für deterministische Tests.
//!
//! Enthält **keinen** Protect-Code und bildet kein unbekanntes Protokoll nach.
//! Er stellt nur die überprüfbaren Rahmenbedingungen bereit: TLS mit
//! selbstsigniertem Zertifikat, Aufzeichnung empfangener Daten und gezielte
//! Fehlerfälle. Protokoll-Hypothesen kommen als Fixtures dazu (AKL-M030-03).

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use akl_protect_compat::tls::Fingerprint;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::TlsAcceptor;

/// Verhalten des Simulators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    /// TLS-Handshake, danach je Zeile eine Antwort `OK`.
    Normal,
    /// Nimmt TCP an, antwortet aber nie (Zeitüberschreitung beim Client).
    Silent,
    /// Schließt die Verbindung sofort nach dem Annehmen.
    Reset,
}

pub struct SimController {
    pub addr: SocketAddr,
    pub fingerprint: Fingerprint,
    received: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

impl SimController {
    /// Startet auf 127.0.0.1 mit zufälligem Port und frischem Zertifikat.
    pub async fn start(behavior: Behavior) -> std::io::Result<Self> {
        Self::start_on("127.0.0.1:0", behavior).await
    }

    pub async fn start_on(bind: &str, behavior: Behavior) -> std::io::Result<Self> {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
            .map_err(std::io::Error::other)?;
        let cert_der: CertificateDer<'static> = cert.cert.der().clone();
        let fingerprint = Fingerprint::of(cert_der.as_ref());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(std::io::Error::other)?
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key)
        .map_err(std::io::Error::other)?;
        let acceptor = TlsAcceptor::from(Arc::new(config));

        let listener = TcpListener::bind(bind).await?;
        let addr = listener.local_addr()?;
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    return;
                };
                let acceptor = acceptor.clone();
                let log = log.clone();
                tokio::spawn(async move {
                    match behavior {
                        Behavior::Reset => drop(tcp),
                        Behavior::Silent => {
                            tokio::time::sleep(Duration::from_secs(3600)).await;
                            drop(tcp);
                        }
                        Behavior::Normal => {
                            let Ok(tls) = acceptor.accept(tcp).await else {
                                return;
                            };
                            let (read, mut write) = tokio::io::split(tls);
                            let mut lines = BufReader::new(read).lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                log.lock().expect("Log-Mutex").push(line);
                                if write.write_all(b"OK\n").await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                });
            }
        });
        Ok(Self {
            addr,
            fingerprint,
            received,
            task,
        })
    }

    /// Alles, was Clients nach erfolgreichem Handshake gesendet haben.
    pub fn received(&self) -> Vec<String> {
        self.received.lock().expect("Log-Mutex").clone()
    }
}

impl Drop for SimController {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Sendet eine Discovery-Suchanfrage als Unicast an `target` (wie TP-002 von
/// der UNVR beobachtet) und wartet höchstens `timeout` auf eine Antwort.
/// Liefert die rohen Antwortbytes oder `None` bei Zeitüberschreitung.
pub async fn discovery_probe(
    target: SocketAddr,
    probe: &[u8],
    timeout: Duration,
) -> std::io::Result<Option<Vec<u8>>> {
    let socket = tokio::net::UdpSocket::bind(("127.0.0.1", 0)).await?;
    socket.send_to(probe, target).await?;
    let mut buf = vec![0u8; akl_protect_compat::discovery::MAX_PACKET + 1];
    match tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await {
        Err(_) => Ok(None),
        Ok(Err(err)) => Err(err),
        Ok(Ok((len, from))) if from == target => Ok(Some(buf[..len].to_vec())),
        // Antwort von fremder Adresse zählt nicht.
        Ok(Ok(_)) => Ok(None),
    }
}
