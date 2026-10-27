//! Discovery-Antwort (AKL-M040-03) – Hypothese, Ziel `synthetic_verified`.
//!
//! Der Responder lauscht auf UDP (Standard `0.0.0.0:10001`, nur im Netzmodus
//! host/macvlan sinnvoll, `docs/operations/NETWORKING.md`) und beantwortet
//! **ausschließlich** erkannte Suchanfragen (`Packet::probe`, DISC-H02/H03).
//! Alles andere wird verworfen. Ohne eingehende Anfrage wird nie gesendet.
//!
//! Schutzmaßnahmen, weil die Antwort größer als die Anfrage ist:
//! - Antwort nur als Unicast an den Absender und nur an lokale Adressen
//!   (Loopback, RFC 1918, Link-Local, ULA) – keine Verstärkung ins Internet.
//! - Höchstens eine Antwort je Absender und [`MIN_REPLY_INTERVAL`].
//!
//! Zustand ändert sich nur über den Lifecycle-Service: die erste beantwortete
//! Anfrage führt `unconfigured → discovered`. Kill-Switch aktiv oder Profil
//! deaktiviert ⇒ keine Antwort.
//!
//! Welche Feldwerte Protect von einem AI-Key-Gerät erwartet, ist offen
//! (DISC-H07). Die Standardwerte sind daher neutral und ohne Herstellerbezug;
//! der Betreiber kann sie für TP-001 gezielt setzen.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use akl_core::{DeviceProfileKind, DeviceState};
use akl_persistence::devices::{TransitionError, TransitionRequest};
use akl_protect_compat::discovery::{MAX_PACKET, Packet, Probe, Tlv, tlv};
use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::service::{Lifecycle, LifecycleError};

/// Mindestabstand zwischen zwei Antworten an denselben Absender.
pub const MIN_REPLY_INTERVAL: Duration = Duration::from_secs(1);
/// Obergrenze der gemerkten Absender (danach wird die Tabelle geleert).
const MAX_TRACKED_SOURCES: usize = 256;

/// Kommando der v2-Antwort. DISC-H04 nennt 0x06/0x09/0x0b; welches Protect
/// erwartet, ist offen (`needs_evidence`).
const V2_REPLY_COMMAND: u8 = 0x06;

/// Was das Gerät in der Antwort über sich mitteilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub hostname: String,
    pub product: String,
    pub model: String,
    pub version: String,
    /// MAC des lauschenden Interfaces, falls bekannt (sonst kein MAC-TLV).
    pub mac: Option<[u8; 6]>,
    /// Konfigurationsstatus „unmanaged“. Kodierung offen (DISC-H07): Ohne
    /// Wert wird kein TLV gesendet, statt eine Kodierung zu erfinden.
    pub config_status: Option<Vec<u8>>,
}

impl Announcement {
    /// Neutrale Standardwerte aus Profil-ID und -art.
    pub fn neutral(profile_id: Uuid, kind: DeviceProfileKind) -> Self {
        let short = &profile_id.simple().to_string()[..8];
        Self {
            hostname: format!("ai-keyless-{short}"),
            product: format!("ai-keyless-{}", kind.as_str()),
            model: "AKL-SYNTH".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            mac: None,
            config_status: None,
        }
    }

    /// Antwortpaket für die erkannte Suchanfrage.
    pub fn reply(&self, probe: Probe) -> Packet {
        let (version, command, model_kind) = match probe {
            Probe::V1 => (1, 0x00, tlv::MODEL_V1),
            Probe::V2 => (2, V2_REPLY_COMMAND, tlv::MODEL_V2),
        };
        let text = |kind, value: &str| Tlv {
            kind,
            value: value.as_bytes().to_vec(),
        };
        let mut tlvs = Vec::new();
        if let Some(mac) = self.mac {
            tlvs.push(Tlv {
                kind: tlv::MAC,
                value: mac.to_vec(),
            });
        }
        tlvs.push(text(tlv::HOSTNAME, &self.hostname));
        tlvs.push(text(tlv::PRODUCT, &self.product));
        tlvs.push(text(model_kind, &self.model));
        tlvs.push(text(tlv::VERSION, &self.version));
        if let Some(status) = &self.config_status {
            tlvs.push(Tlv {
                kind: tlv::CONFIG_STATUS,
                value: status.clone(),
            });
        }
        Packet {
            version,
            command,
            tlvs,
        }
    }
}

/// Antwortbytes für ein empfangenes Datagramm, oder `None` (verwerfen).
/// Rein, ohne Netz und Zustand – die eigentliche Filterlogik.
pub fn answer(datagram: &[u8], announcement: &Announcement) -> Option<Vec<u8>> {
    let probe = Packet::decode(datagram).ok()?.probe()?;
    announcement.reply(probe).encode().ok()
}

/// Nur Absender im lokalen Netz bekommen eine Antwort.
pub fn is_local_source(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Discovery-Socket: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error("Geräteprofil nicht gefunden")]
    ProfileNotFound,
}

/// Gebundener, noch nicht laufender Responder für genau ein Profil.
pub struct Responder {
    socket: UdpSocket,
    lifecycle: Lifecycle,
    profile_id: Uuid,
    announcement: Announcement,
}

impl Responder {
    pub async fn bind(
        addr: SocketAddr,
        lifecycle: Lifecycle,
        profile_id: Uuid,
        announcement: Announcement,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            socket: UdpSocket::bind(addr).await?,
            lifecycle,
            profile_id,
            announcement,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, DiscoveryError> {
        Ok(self.socket.local_addr()?)
    }

    /// Läuft, bis der Task abgebrochen wird oder der Socket ausfällt.
    pub async fn run(self) -> Result<(), DiscoveryError> {
        let mut buf = vec![0u8; MAX_PACKET + 1];
        let mut last_reply: HashMap<IpAddr, Instant> = HashMap::new();
        loop {
            let (len, from) = self.socket.recv_from(&mut buf).await?;
            if !is_local_source(from.ip()) {
                continue;
            }
            let Some(reply) = answer(&buf[..len], &self.announcement) else {
                continue;
            };
            let now = Instant::now();
            if last_reply
                .get(&from.ip())
                .is_some_and(|t| now.duration_since(*t) < MIN_REPLY_INTERVAL)
            {
                continue;
            }
            // Zustand je Anfrage frisch prüfen: Kill-Switch und Deaktivierung
            // wirken sofort, ohne Neustart des Responders.
            if !self.may_answer().await? {
                continue;
            }
            if last_reply.len() >= MAX_TRACKED_SOURCES {
                last_reply.clear();
            }
            last_reply.insert(from.ip(), now);
            self.socket.send_to(&reply, from).await?;
            self.mark_discovered().await?;
        }
    }

    async fn may_answer(&self) -> Result<bool, DiscoveryError> {
        if self.lifecycle.kill_switch().await?.active {
            return Ok(false);
        }
        let profile = self
            .lifecycle
            .db()
            .get_device_profile(self.profile_id)
            .await
            .map_err(LifecycleError::from)?
            .ok_or(DiscoveryError::ProfileNotFound)?;
        Ok(profile.state != DeviceState::Disabled)
    }

    /// `unconfigured → discovered`; in jedem anderen Zustand nichts tun.
    async fn mark_discovered(&self) -> Result<(), DiscoveryError> {
        let req = TransitionRequest {
            to: DeviceState::Discovered,
            expected_from: Some(DeviceState::Unconfigured),
            reason: "discovery_probe_answered",
            actor: "system:discovery",
            correlation_id: None,
        };
        match self.lifecycle.transition(self.profile_id, req).await {
            Ok(_) => Ok(()),
            // Schon weiter im Lifecycle oder inzwischen per Kill-Switch gesperrt.
            Err(LifecycleError::Transition(
                TransitionError::Stale { .. } | TransitionError::KillSwitch,
            )) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn announcement() -> Announcement {
        Announcement::neutral(Uuid::nil(), DeviceProfileKind::AiKey)
    }

    #[test]
    fn v1_probe_gets_v1_reply_with_identity_fields() {
        let a = announcement();
        let bytes = answer(&[1, 0, 0, 0], &a).expect("Antwort");
        let reply = Packet::decode(&bytes).unwrap();
        assert_eq!((reply.version, reply.command), (1, 0));
        assert_eq!(reply.find(tlv::HOSTNAME), Some(&b"ai-keyless-00000000"[..]));
        assert_eq!(reply.find(tlv::PRODUCT), Some(&b"ai-keyless-ai_key"[..]));
        assert_eq!(reply.find(tlv::MODEL_V1), Some(&b"AKL-SYNTH"[..]));
        assert_eq!(reply.find(tlv::MODEL_V2), None);
        assert_eq!(reply.find(tlv::VERSION), Some(a.version.as_bytes()));
        // Unbelegte Felder werden nicht erfunden.
        assert_eq!(reply.find(tlv::MAC), None);
        assert_eq!(reply.find(tlv::CONFIG_STATUS), None);
        // Eine Antwort ist selbst keine Suchanfrage (keine Antwortschleifen).
        assert_eq!(reply.probe(), None);
        assert!(answer(&bytes, &a).is_none());
    }

    #[test]
    fn v2_probe_gets_v2_reply() {
        let reply = Packet::decode(&answer(&[2, 8, 0, 0], &announcement()).unwrap()).unwrap();
        assert_eq!((reply.version, reply.command), (2, V2_REPLY_COMMAND));
        assert_eq!(reply.find(tlv::MODEL_V2), Some(&b"AKL-SYNTH"[..]));
    }

    #[test]
    fn optional_fields_are_sent_when_set() {
        let a = Announcement {
            mac: Some([2, 0, 0, 0, 0, 1]),
            config_status: Some(vec![1]),
            ..announcement()
        };
        let reply = Packet::decode(&answer(&[1, 0, 0, 0], &a).unwrap()).unwrap();
        assert_eq!(reply.find(tlv::MAC), Some(&[2, 0, 0, 0, 0, 1][..]));
        assert_eq!(reply.find(tlv::CONFIG_STATUS), Some(&[1][..]));
    }

    #[test]
    fn everything_else_is_dropped() {
        let a = announcement();
        for junk in [
            &[][..],
            &[1, 0, 0],
            &[1, 0, 0, 1],
            &[2, 6, 0, 0],
            &[3, 0, 0, 0],
            &[1, 0, 0, 3, 0x0b, 0, 0],
            b"GET / HTTP/1.1\r\n",
        ] {
            assert!(answer(junk, &a).is_none(), "{junk:?}");
        }
        assert!(answer(&vec![1; MAX_PACKET + 1], &a).is_none());
    }

    #[test]
    fn only_local_sources() {
        for ip in [
            "127.0.0.1",
            "192.168.10.5",
            "10.1.2.3",
            "169.254.1.1",
            "::1",
            "fd00::1",
            "fe80::1",
        ] {
            assert!(is_local_source(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "100.64.0.1", "2001:db8::1", "255.255.255.255"] {
            assert!(!is_local_source(ip.parse().unwrap()), "{ip}");
        }
    }
}
