//! Contract-Fixtures der Discovery (AKL-M030-03): jede Fixture trägt Quelle und
//! Evidenzstufe, und der Codec verhält sich gemäß den Hypothesen.

use std::collections::HashSet;

use akl_core::EvidenceLevel;
use akl_protect_compat::discovery::{Packet, Probe, tlv};
use serde::Deserialize;

const FIXTURES: &str = include_str!("../../../tests/contracts/discovery/hypotheses.json");

#[derive(Deserialize)]
struct File {
    contract: String,
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
struct Fixture {
    id: String,
    kind: String,
    statement: String,
    source: String,
    evidence: EvidenceLevel,
    #[serde(default)]
    bytes_hex: Option<String>,
}

fn fixtures() -> Vec<Fixture> {
    let file: File = serde_json::from_str(FIXTURES).expect("hypotheses.json");
    assert_eq!(file.contract, "discovery");
    file.fixtures
}

fn hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "{s}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes(id: &str) -> Vec<u8> {
    let f = fixtures().into_iter().find(|f| f.id == id).unwrap();
    hex(f.bytes_hex.as_deref().unwrap())
}

#[test]
fn every_fixture_has_source_and_honest_evidence() {
    let mut ids = HashSet::new();
    for f in fixtures() {
        assert!(ids.insert(f.id.clone()), "doppelte ID {}", f.id);
        assert!(!f.statement.trim().is_empty(), "{}", f.id);
        assert!(!f.source.trim().is_empty(), "{}: Quelle fehlt", f.id);
        assert!(
            ["fact", "probe", "reply", "open"].contains(&f.kind.as_str()),
            "{}",
            f.id
        );
        // Ohne eigenen Live-Test gibt es hier keinen nativen Nachweis.
        assert!(
            !f.evidence.is_native(),
            "{}: native_verified ohne Nachweis",
            f.id
        );
        // Nur selbst konstruierte Beispiele dürfen synthetisch verifiziert sein.
        if f.evidence == EvidenceLevel::SyntheticVerified {
            assert!(f.source.starts_with("eigene Konstruktion"), "{}", f.id);
        }
    }
}

#[test]
fn probe_fixtures_decode_as_probes() {
    assert_eq!(
        Packet::decode(&bytes("DISC-H02")).unwrap().probe(),
        Some(Probe::V1)
    );
    assert_eq!(
        Packet::decode(&bytes("DISC-H03")).unwrap().probe(),
        Some(Probe::V2)
    );
}

#[test]
fn synthetic_reply_roundtrips() {
    let raw = bytes("DISC-H06");
    let packet = Packet::decode(&raw).unwrap();
    assert_eq!((packet.version, packet.command), (1, 0));
    assert_eq!(packet.probe(), None);
    assert_eq!(packet.find(tlv::HOSTNAME), Some(&b"akl-test"[..]));
    assert_eq!(packet.find(tlv::MODEL_V1), Some(&b"SYNTH"[..]));
    assert_eq!(packet.encode().unwrap(), raw);
}
