//! Kompatibilitäts-Evidenzstufen.
//!
//! Jede Fähigkeitsbehauptung in API, UI und Dokumentation wird genau einer
//! dieser Stufen zugeordnet. Siehe `docs/compatibility/MATRIX.md`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    /// Spezifiziert, nicht gebaut.
    Planned,
    /// Code vorhanden, nicht ausreichend getestet.
    Implemented,
    /// Lokale Fixtures/Mocks erfolgreich.
    SyntheticVerified,
    /// Autorisierter Live-Test mit echter Protect-Version inkl. Readback.
    NativeVerified,
    /// Nachweislich nicht angeboten oder bewusst deaktiviert.
    Unsupported,
    /// Schnittstelle oder Wirkung offen.
    NeedsEvidence,
}

impl EvidenceLevel {
    /// Nur `native_verified` darf in der UI als nativ funktionsfähig gelten.
    pub fn is_native(self) -> bool {
        matches!(self, Self::NativeVerified)
    }

    /// Darf eine Funktion mit dieser Evidenz standardmäßig aktiviert werden?
    ///
    /// Synthetisch verifizierte Funktionen sind nur hinter explizitem Opt-in
    /// erlaubt; die Grundeinstellung bleibt deaktiviert.
    pub fn enabled_by_default(self) -> bool {
        self.is_native()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_snake_case() {
        let json = serde_json::to_string(&EvidenceLevel::SyntheticVerified).unwrap();
        assert_eq!(json, "\"synthetic_verified\"");
        let back: EvidenceLevel = serde_json::from_str("\"needs_evidence\"").unwrap();
        assert_eq!(back, EvidenceLevel::NeedsEvidence);
    }

    #[test]
    fn only_native_is_enabled_by_default() {
        use EvidenceLevel::*;
        for level in [
            Planned,
            Implemented,
            SyntheticVerified,
            Unsupported,
            NeedsEvidence,
        ] {
            assert!(!level.enabled_by_default(), "{level:?}");
        }
        assert!(NativeVerified.enabled_by_default());
    }
}
