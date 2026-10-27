//! Versionierte Protect-Kompatibilitätsprofile (AKL-M030-01, ADR-003).
//!
//! Die Profile liegen als Daten in `profiles/*.json` und werden beim Kompilieren
//! eingebettet. Jede Fähigkeit trägt eine Evidenzstufe; freigegeben wird nur, was
//! belegt ist. Unbekannte Protect-Versionen führen in den konservativen Modus.

use std::{fmt, str::FromStr};

pub mod tls;

use akl_core::{DeviceProfileKind, EvidenceLevel};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Versionen

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtectVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtectVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ungültige Protect-Version: {0:?}")]
pub struct InvalidVersion(pub String);

impl FromStr for ProtectVersion {
    type Err = InvalidVersion;

    /// Akzeptiert `7.3.60` und `v7.3.60`; Vorabversionen/Suffixe werden abgelehnt,
    /// damit sie nicht stillschweigend als stabile Version gelten.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || InvalidVersion(s.to_owned());
        let trimmed = s.trim();
        let core = trimmed.strip_prefix('v').unwrap_or(trimmed);
        let mut parts = core.split('.');
        let mut next = || -> Result<u32, InvalidVersion> {
            let part = parts.next().ok_or_else(err)?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) || part.len() > 6 {
                return Err(err());
            }
            part.parse().map_err(|_| err())
        };
        let version = Self::new(next()?, next()?, next()?);
        if parts.next().is_some() {
            return Err(err());
        }
        Ok(version)
    }
}

impl fmt::Display for ProtectVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for ProtectVersion {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ProtectVersion {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Fähigkeiten

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // AI Key
    Discovery,
    Adoption,
    ControlChannel,
    Keepalive,
    CaptionOnDemand,
    CaptionAutomatic,
    NativeSearch,
    AiAlerts,
    SpeechToText,
    LicensePlates,
    Faces,
    Enhancement,
    // AI Port
    PortPairing,
    PortStreamProcessing,
    PortSmartDetections,
    PortSmartZones,
}

impl Capability {
    /// Zu welchem Geräteprofil gehört die Fähigkeit? Profile teilen keine Fähigkeiten.
    pub fn profile(self) -> DeviceProfileKind {
        use Capability::*;
        match self {
            PortPairing | PortStreamProcessing | PortSmartDetections | PortSmartZones => {
                DeviceProfileKind::AiPort
            }
            _ => DeviceProfileKind::AiKey,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub capability: Capability,
    pub evidence: EvidenceLevel,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    pub id: String,
    pub min: ProtectVersion,
    pub max_exclusive: ProtectVersion,
    pub source: String,
    pub claims: Vec<Claim>,
}

impl VersionRange {
    pub fn contains(&self, v: ProtectVersion) -> bool {
        self.min <= v && v < self.max_exclusive
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFile {
    pub profile: DeviceProfileKind,
    #[serde(default)]
    pub comment: String,
    pub ranges: Vec<VersionRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("Profildatei {0}: JSON ungültig: {1}")]
    Parse(&'static str, String),
    #[error("{range}: leerer Versionsbereich")]
    EmptyRange { range: String },
    #[error("{a} und {b} überlappen")]
    Overlap { a: String, b: String },
    #[error("{range}: Fähigkeit {capability:?} gehört nicht zum Profil {profile:?}")]
    ForeignCapability {
        range: String,
        capability: Capability,
        profile: DeviceProfileKind,
    },
    #[error("{range}: Fähigkeit {capability:?} doppelt")]
    DuplicateCapability {
        range: String,
        capability: Capability,
    },
    #[error("{range}: native_verified ohne Nachweis-Notiz")]
    NativeWithoutEvidenceNote { range: String },
}

fn validate(file: &ProfileFile) -> Result<(), ProfileError> {
    let mut ranges: Vec<&VersionRange> = file.ranges.iter().collect();
    ranges.sort_by_key(|r| r.min);
    for r in &ranges {
        if r.min >= r.max_exclusive {
            return Err(ProfileError::EmptyRange {
                range: r.id.clone(),
            });
        }
        let mut seen = Vec::new();
        for claim in &r.claims {
            if claim.capability.profile() != file.profile {
                return Err(ProfileError::ForeignCapability {
                    range: r.id.clone(),
                    capability: claim.capability,
                    profile: file.profile,
                });
            }
            if seen.contains(&claim.capability) {
                return Err(ProfileError::DuplicateCapability {
                    range: r.id.clone(),
                    capability: claim.capability,
                });
            }
            // Nativ belegte Fähigkeiten müssen auf ihren Nachweis verweisen.
            if claim.evidence == EvidenceLevel::NativeVerified && !claim.note.contains("evidence/")
            {
                return Err(ProfileError::NativeWithoutEvidenceNote {
                    range: r.id.clone(),
                });
            }
            seen.push(claim.capability);
        }
    }
    for pair in ranges.windows(2) {
        if pair[1].min < pair[0].max_exclusive {
            return Err(ProfileError::Overlap {
                a: pair[0].id.clone(),
                b: pair[1].id.clone(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Registry und Auflösung

#[derive(Debug, Clone)]
pub struct Registry {
    files: Vec<ProfileFile>,
}

const EMBEDDED: [(&str, &str); 2] = [
    ("ai_key.json", include_str!("../profiles/ai_key.json")),
    ("ai_port.json", include_str!("../profiles/ai_port.json")),
];

impl Registry {
    /// Lädt die eingebetteten Profile. Fehler hier sind Build-Fehler der Daten
    /// und werden durch Tests abgefangen.
    pub fn embedded() -> Result<Self, ProfileError> {
        let files = EMBEDDED
            .iter()
            .map(|(name, json)| {
                serde_json::from_str::<ProfileFile>(json)
                    .map_err(|e| ProfileError::Parse(name, e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_files(files)
    }

    pub fn from_files(files: Vec<ProfileFile>) -> Result<Self, ProfileError> {
        for file in &files {
            validate(file)?;
        }
        Ok(Self { files })
    }

    pub fn ranges(&self, profile: DeviceProfileKind) -> impl Iterator<Item = &VersionRange> {
        self.files
            .iter()
            .filter(move |f| f.profile == profile)
            .flat_map(|f| f.ranges.iter())
    }

    /// Ermittelt das Profil für eine gemeldete Protect-Version. Eine nicht
    /// parsebare oder unbekannte Version führt in den konservativen Modus.
    pub fn resolve(&self, profile: DeviceProfileKind, reported_version: &str) -> Resolution {
        let version = match reported_version.parse::<ProtectVersion>() {
            Ok(v) => v,
            Err(_) => {
                return Resolution {
                    profile,
                    version: None,
                    range: None,
                    mode: Mode::Conservative,
                    warning: Some(Warning::UnparsableVersion(reported_version.to_owned())),
                };
            }
        };
        match self.ranges(profile).find(|r| r.contains(version)) {
            Some(range) => Resolution {
                profile,
                version: Some(version),
                range: Some(range.clone()),
                mode: Mode::Profiled,
                warning: None,
            },
            None => Resolution {
                profile,
                version: Some(version),
                range: None,
                mode: Mode::Conservative,
                warning: Some(Warning::UnknownVersion(version)),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Bekanntes Versionsprofil.
    Profiled,
    /// Unbekannte Version: keine Fähigkeit wird angeboten oder aktiviert.
    Conservative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum Warning {
    UnknownVersion(ProtectVersion),
    UnparsableVersion(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Enabled,
    /// Synthetisch verifiziert, aber nicht ausdrücklich freigeschaltet.
    RequiresOptIn,
    /// Evidenz reicht nicht (`planned`, `implemented`, `needs_evidence`).
    InsufficientEvidence,
    Unsupported,
    /// Fähigkeit gehört zum anderen Geräteprofil.
    WrongProfile,
    /// Konservativer Modus wegen unbekannter Version.
    UnknownVersion,
    /// Für diese Version nicht beschrieben.
    NotDescribed,
}

impl Decision {
    pub fn is_enabled(self) -> bool {
        self == Self::Enabled
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Resolution {
    pub profile: DeviceProfileKind,
    pub version: Option<ProtectVersion>,
    pub range: Option<VersionRange>,
    pub mode: Mode,
    pub warning: Option<Warning>,
}

impl Resolution {
    pub fn evidence(&self, capability: Capability) -> Option<EvidenceLevel> {
        self.range
            .as_ref()?
            .claims
            .iter()
            .find(|c| c.capability == capability)
            .map(|c| c.evidence)
    }

    /// Darf die Fähigkeit genutzt bzw. gegenüber Protect angeboten werden?
    /// `synthetic_opt_in`: Betreiber hat synthetisch verifizierte Funktionen
    /// ausdrücklich freigeschaltet (Testbetrieb).
    pub fn decide(&self, capability: Capability, synthetic_opt_in: bool) -> Decision {
        if capability.profile() != self.profile {
            return Decision::WrongProfile;
        }
        if self.mode == Mode::Conservative {
            return Decision::UnknownVersion;
        }
        match self.evidence(capability) {
            None => Decision::NotDescribed,
            Some(EvidenceLevel::NativeVerified) => Decision::Enabled,
            Some(EvidenceLevel::SyntheticVerified) if synthetic_opt_in => Decision::Enabled,
            Some(EvidenceLevel::SyntheticVerified) => Decision::RequiresOptIn,
            Some(EvidenceLevel::Unsupported) => Decision::Unsupported,
            Some(
                EvidenceLevel::Planned | EvidenceLevel::Implemented | EvidenceLevel::NeedsEvidence,
            ) => Decision::InsufficientEvidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeviceProfileKind::{AiKey, AiPort};

    fn v(s: &str) -> ProtectVersion {
        s.parse().unwrap()
    }

    #[test]
    fn version_parsing() {
        assert_eq!(v("7.3.60"), ProtectVersion::new(7, 3, 60));
        assert_eq!(v(" v7.3.56 "), ProtectVersion::new(7, 3, 56));
        for bad in [
            "",
            "7",
            "7.3",
            "7.3.60.1",
            "7.3.x",
            "7.3.60-beta",
            "-1.0.0",
            "7..1",
            "9999999.0.0",
        ] {
            assert!(bad.parse::<ProtectVersion>().is_err(), "{bad:?}");
        }
        assert!(v("7.10.0") > v("7.9.99"));
        assert_eq!(v("7.3.60").to_string(), "7.3.60");
    }

    #[test]
    fn embedded_profiles_are_valid_and_honest() {
        let reg = Registry::embedded().expect("eingebettete Profile gültig");
        // Ehrlichkeitsprüfung: Solange es keine Nachweise unter
        // docs/compatibility/evidence/ gibt, darf nichts nativ verifiziert sein.
        for kind in [AiKey, AiPort] {
            for range in reg.ranges(kind) {
                for claim in &range.claims {
                    assert_ne!(
                        claim.evidence,
                        EvidenceLevel::NativeVerified,
                        "{}",
                        range.id
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_or_unparsable_version_is_conservative() {
        let reg = Registry::embedded().unwrap();
        let r = reg.resolve(AiKey, "8.0.0");
        assert_eq!(r.mode, Mode::Conservative);
        assert_eq!(r.warning, Some(Warning::UnknownVersion(v("8.0.0"))));
        assert_eq!(
            r.decide(Capability::Adoption, true),
            Decision::UnknownVersion
        );

        let r = reg.resolve(AiKey, "irgendwas");
        assert_eq!(r.mode, Mode::Conservative);
        assert!(matches!(r.warning, Some(Warning::UnparsableVersion(_))));
    }

    #[test]
    fn known_version_without_evidence_enables_nothing() {
        let reg = Registry::embedded().unwrap();
        let r = reg.resolve(AiKey, "7.3.60");
        assert_eq!(r.mode, Mode::Profiled);
        assert_eq!(
            r.decide(Capability::CaptionAutomatic, true),
            Decision::InsufficientEvidence
        );
        assert_eq!(
            r.decide(Capability::Keepalive, true),
            Decision::InsufficientEvidence
        );
    }

    #[test]
    fn profiles_are_strictly_separated() {
        let reg = Registry::embedded().unwrap();
        let key = reg.resolve(AiKey, "7.3.60");
        assert_eq!(
            key.decide(Capability::PortPairing, true),
            Decision::WrongProfile
        );
        let port = reg.resolve(AiPort, "7.3.60");
        assert_eq!(
            port.decide(Capability::Adoption, true),
            Decision::WrongProfile
        );
    }

    fn file(profile: DeviceProfileKind, ranges: Vec<VersionRange>) -> ProfileFile {
        ProfileFile {
            profile,
            comment: String::new(),
            ranges,
        }
    }

    fn range(id: &str, min: &str, max: &str, claims: Vec<Claim>) -> VersionRange {
        VersionRange {
            id: id.into(),
            min: v(min),
            max_exclusive: v(max),
            source: String::new(),
            claims,
        }
    }

    fn claim(capability: Capability, evidence: EvidenceLevel, note: &str) -> Claim {
        Claim {
            capability,
            evidence,
            note: note.into(),
        }
    }

    #[test]
    fn decisions_follow_evidence() {
        let reg = Registry::from_files(vec![file(
            AiKey,
            vec![range(
                "r",
                "7.3.0",
                "7.4.0",
                vec![
                    claim(
                        Capability::Adoption,
                        EvidenceLevel::NativeVerified,
                        "evidence/2026-11-01-adoption.md",
                    ),
                    claim(Capability::Keepalive, EvidenceLevel::SyntheticVerified, ""),
                    claim(Capability::Faces, EvidenceLevel::Unsupported, ""),
                ],
            )],
        )])
        .unwrap();
        let r = reg.resolve(AiKey, "7.3.1");
        assert_eq!(r.decide(Capability::Adoption, false), Decision::Enabled);
        assert_eq!(
            r.decide(Capability::Keepalive, false),
            Decision::RequiresOptIn
        );
        assert_eq!(r.decide(Capability::Keepalive, true), Decision::Enabled);
        assert_eq!(r.decide(Capability::Faces, true), Decision::Unsupported);
        assert_eq!(
            r.decide(Capability::NativeSearch, true),
            Decision::NotDescribed
        );
        // Obergrenze ist exklusiv.
        assert_eq!(reg.resolve(AiKey, "7.4.0").mode, Mode::Conservative);
    }

    #[test]
    fn invalid_profile_data_is_rejected() {
        let overlap = vec![
            range("a", "7.0.0", "7.4.0", vec![]),
            range("b", "7.3.0", "7.5.0", vec![]),
        ];
        assert!(matches!(
            Registry::from_files(vec![file(AiKey, overlap)]),
            Err(ProfileError::Overlap { .. })
        ));

        let empty = vec![range("a", "7.3.0", "7.3.0", vec![])];
        assert!(matches!(
            Registry::from_files(vec![file(AiKey, empty)]),
            Err(ProfileError::EmptyRange { .. })
        ));

        let foreign = vec![range(
            "a",
            "7.3.0",
            "7.4.0",
            vec![claim(Capability::PortPairing, EvidenceLevel::Planned, "")],
        )];
        assert!(matches!(
            Registry::from_files(vec![file(AiKey, foreign)]),
            Err(ProfileError::ForeignCapability { .. })
        ));

        let dup = vec![range(
            "a",
            "7.3.0",
            "7.4.0",
            vec![
                claim(Capability::Adoption, EvidenceLevel::Planned, ""),
                claim(Capability::Adoption, EvidenceLevel::NeedsEvidence, ""),
            ],
        )];
        assert!(matches!(
            Registry::from_files(vec![file(AiKey, dup)]),
            Err(ProfileError::DuplicateCapability { .. })
        ));

        let native_no_proof = vec![range(
            "a",
            "7.3.0",
            "7.4.0",
            vec![claim(
                Capability::Adoption,
                EvidenceLevel::NativeVerified,
                "",
            )],
        )];
        assert!(matches!(
            Registry::from_files(vec![file(AiKey, native_no_proof)]),
            Err(ProfileError::NativeWithoutEvidenceNote { .. })
        ));
    }
}
