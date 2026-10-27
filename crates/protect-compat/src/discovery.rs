//! Discovery-Paketformat (AKL-M030-03) – Hypothese, `needs_evidence`.
//!
//! Grundlage sind öffentliche Protokollbeschreibungen (Fakten, kein Code;
//! `docs/legal/PROVENANCE.md` Abschnitt 4a) und die Fixtures unter
//! `tests/contracts/discovery/`. Ob Protect AI-Key-Geräte über dieses Format
//! findet und welche Felder es erwartet, ist **nicht** belegt (Q-006, TP-002).
//!
//! Aufbau: `Version (1) · Kommando (1) · Länge (2, Big-Endian) · TLV*`, die
//! Länge zählt die TLV-Bytes. TLV: `Typ (1) · Länge (2, BE) · Wert`.
//!
//! Der Decoder verarbeitet Daten aus dem Netz: harte Obergrenzen, keine
//! Panics, jede Längenangabe wird gegen die tatsächlichen Bytes geprüft.

/// UDP-Port laut öffentlicher Beschreibung (`needs_evidence`).
pub const DISCOVERY_PORT: u16 = 10001;

/// Größtes akzeptierte Paket (ein Ethernet-Frame ohne Fragmentierung).
pub const MAX_PACKET: usize = 1500;
/// Höchstzahl TLVs pro Paket.
pub const MAX_TLVS: usize = 64;

/// TLV-Typen laut öffentlicher Beschreibung (Bedeutung `needs_evidence`).
pub mod tlv {
    pub const MAC: u8 = 0x01;
    pub const MAC_IP: u8 = 0x02;
    pub const FIRMWARE: u8 = 0x03;
    pub const UPTIME: u8 = 0x0a;
    pub const HOSTNAME: u8 = 0x0b;
    pub const PRODUCT: u8 = 0x0c;
    pub const MODEL_V1: u8 = 0x14;
    pub const MODEL_V2: u8 = 0x15;
    pub const VERSION: u8 = 0x16;
    pub const CONFIG_STATUS: u8 = 0x17;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tlv {
    pub kind: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub version: u8,
    pub command: u8,
    pub tlvs: Vec<Tlv>,
}

/// Erkannte Suchanfrage (Hypothese DISC-H02/H03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    V1,
    V2,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("Paket kürzer als der 4-Byte-Kopf")]
    TooShort,
    #[error("Paket größer als {MAX_PACKET} Byte")]
    TooLarge,
    #[error("Längenfeld {declared} passt nicht zu {actual} Nutzbytes")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("TLV bei Offset {offset} läuft über das Paketende")]
    TruncatedTlv { offset: usize },
    #[error("mehr als {MAX_TLVS} TLVs")]
    TooManyTlvs,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("TLV-Wert länger als 65535 Byte")]
    TlvTooLong,
    #[error("Paket größer als {MAX_PACKET} Byte")]
    TooLarge,
    #[error("mehr als {MAX_TLVS} TLVs")]
    TooManyTlvs,
}

impl Packet {
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() > MAX_PACKET {
            return Err(DecodeError::TooLarge);
        }
        let [version, command, hi, lo, body @ ..] = buf else {
            return Err(DecodeError::TooShort);
        };
        let declared = usize::from(u16::from_be_bytes([*hi, *lo]));
        if declared != body.len() {
            return Err(DecodeError::LengthMismatch {
                declared,
                actual: body.len(),
            });
        }
        let mut tlvs = Vec::new();
        let mut rest = body;
        while !rest.is_empty() {
            if tlvs.len() == MAX_TLVS {
                return Err(DecodeError::TooManyTlvs);
            }
            let offset = 4 + body.len() - rest.len();
            let [kind, hi, lo, tail @ ..] = rest else {
                return Err(DecodeError::TruncatedTlv { offset });
            };
            let len = usize::from(u16::from_be_bytes([*hi, *lo]));
            if len > tail.len() {
                return Err(DecodeError::TruncatedTlv { offset });
            }
            let (value, next) = tail.split_at(len);
            tlvs.push(Tlv {
                kind: *kind,
                value: value.to_vec(),
            });
            rest = next;
        }
        Ok(Self {
            version: *version,
            command: *command,
            tlvs,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        if self.tlvs.len() > MAX_TLVS {
            return Err(EncodeError::TooManyTlvs);
        }
        let mut body = Vec::new();
        for t in &self.tlvs {
            let len = u16::try_from(t.value.len()).map_err(|_| EncodeError::TlvTooLong)?;
            body.push(t.kind);
            body.extend_from_slice(&len.to_be_bytes());
            body.extend_from_slice(&t.value);
        }
        if body.len() + 4 > MAX_PACKET {
            return Err(EncodeError::TooLarge);
        }
        // MAX_PACKET < 65536, daher passt die Länge immer in u16.
        let len = u16::try_from(body.len()).map_err(|_| EncodeError::TooLarge)?;
        let mut out = Vec::with_capacity(body.len() + 4);
        out.extend_from_slice(&[self.version, self.command]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Suchanfrage ohne Nutzdaten: v1 `01 00`, v2 `02 08` (Hypothese).
    pub fn probe(&self) -> Option<Probe> {
        if !self.tlvs.is_empty() {
            return None;
        }
        match (self.version, self.command) {
            (1, 0) => Some(Probe::V1),
            (2, 8) => Some(Probe::V2),
            _ => None,
        }
    }

    pub fn find(&self, kind: u8) -> Option<&[u8]> {
        self.tlvs
            .iter()
            .find(|t| t.kind == kind)
            .map(|t| t.value.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_input_without_panicking() {
        assert_eq!(Packet::decode(&[]), Err(DecodeError::TooShort));
        assert_eq!(Packet::decode(&[1, 0, 0]), Err(DecodeError::TooShort));
        assert!(matches!(
            Packet::decode(&[1, 0, 0, 5, 1]),
            Err(DecodeError::LengthMismatch {
                declared: 5,
                actual: 1
            })
        ));
        // TLV verspricht 9 Byte, es folgen 2.
        assert!(matches!(
            Packet::decode(&[1, 0, 0, 5, 0x0b, 0, 9, b'a', b'b']),
            Err(DecodeError::TruncatedTlv { offset: 4 })
        ));
        // Halber TLV-Kopf.
        assert!(matches!(
            Packet::decode(&[1, 0, 0, 2, 0x0b, 0]),
            Err(DecodeError::TruncatedTlv { .. })
        ));
        assert_eq!(
            Packet::decode(&vec![0; MAX_PACKET + 1]),
            Err(DecodeError::TooLarge)
        );
    }

    #[test]
    fn too_many_tlvs_rejected() {
        let mut buf = vec![1, 0];
        let n = MAX_TLVS + 1;
        buf.extend_from_slice(&u16::try_from(n * 3).unwrap().to_be_bytes());
        for _ in 0..n {
            buf.extend_from_slice(&[0x0b, 0, 0]);
        }
        assert_eq!(Packet::decode(&buf), Err(DecodeError::TooManyTlvs));
    }

    #[test]
    fn every_prefix_of_a_valid_packet_is_handled() {
        let packet = Packet {
            version: 1,
            command: 0,
            tlvs: vec![
                Tlv {
                    kind: tlv::HOSTNAME,
                    value: b"akl-test".to_vec(),
                },
                Tlv {
                    kind: tlv::MODEL_V1,
                    value: b"SYNTH".to_vec(),
                },
            ],
        };
        let bytes = packet.encode().unwrap();
        assert_eq!(Packet::decode(&bytes).unwrap(), packet);
        for cut in 0..bytes.len() {
            assert!(Packet::decode(&bytes[..cut]).is_err(), "Präfix {cut}");
        }
    }

    #[test]
    fn encode_limits() {
        let big = Packet {
            version: 1,
            command: 0,
            tlvs: vec![Tlv {
                kind: 1,
                value: vec![0; MAX_PACKET],
            }],
        };
        assert_eq!(big.encode(), Err(EncodeError::TooLarge));
        let huge = Packet {
            version: 1,
            command: 0,
            tlvs: vec![Tlv {
                kind: 1,
                value: vec![0; 70_000],
            }],
        };
        assert_eq!(huge.encode(), Err(EncodeError::TlvTooLong));
    }

    #[test]
    fn probes_are_only_empty_v1_or_v2() {
        let p = |version, command| Packet {
            version,
            command,
            tlvs: Vec::new(),
        };
        assert_eq!(p(1, 0).probe(), Some(Probe::V1));
        assert_eq!(p(2, 8).probe(), Some(Probe::V2));
        assert_eq!(p(2, 6).probe(), None);
        assert_eq!(p(3, 0).probe(), None);
        let with_tlv = Packet {
            tlvs: vec![Tlv {
                kind: 1,
                value: vec![],
            }],
            ..p(1, 0)
        };
        assert_eq!(with_tlv.probe(), None);
    }
}
