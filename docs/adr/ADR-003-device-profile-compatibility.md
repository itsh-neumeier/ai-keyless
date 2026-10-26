# ADR-003: Geräteprofilbasierte Kompatibilität

- **Status:** Akzeptiert · **Datum:** 2026-10-26 · **Meilenstein:** AKL-M010/M030/M040

## Kontext

AI Key und AI Port sind unterschiedliche Gerätetypen mit eigener Adoption, eigenen Ports und
eigenen Kamerabeziehungen. Protect-Versionen ändern Verhalten. Falsche Kompatibilitätsaussagen
sind das größte Produktrisiko.

## Entscheidung

1. **Zwei strikt getrennte Profile** `ai_key` und `ai_port` (`DeviceProfileKind`), jeweils mit
   eigener Identität, eigenen Zertifikaten, eigenem Zustand, eigenen Ports, eigener
   Versionsmatrix und eigenen Feature-Flags. Keine gemeinsame Adoption, keine geteilten
   Kameraereignisse. DB-Constraint: höchstens ein aktives `ai_key`-Profil.
2. **Expliziter Zustandsautomat** (`crates/core/src/device.rs`):
   `unconfigured → discovered → pending_adoption → adopted → online ⇄ reconnecting → offline/error`,
   jederzeit `→ disabled` (sicherer Rückweg). Unzulässige Übergänge sind Fehler;
   jeder Übergang wird in `device_state_transitions` mit `correlation_id` protokolliert.
3. **Versionierte Capability-Profile** pro Protect-Version (`crates/protect-compat`, M030):
   bekannte Version → Profil mit belegten Fähigkeiten; unbekannte Version → konservativer Modus
   (nur belegte Basisfunktionen, keine Feature-Advertisements) mit sichtbarer Warnung.
4. **Evidenzstufen** (`EvidenceLevel`) an jeder Fähigkeit. Standardmäßig aktiviert ist nur
   `native_verified`; `synthetic_verified` nur per explizitem Opt-in.
5. Die API meldet unbekannte Zustände als `null` – nie als erfundenen Wert.

## Alternativen

- Ein generisches „Emulator“-Profil mit Flags – verworfen (Vermischungsgefahr).

## Konsequenzen

- Doppelter Implementierungsaufwand für gemeinsame Teile → gemeinsame Bausteine (TLS, Identität)
  als Bibliothek, aber getrennte Instanzen und Daten.
- UI zeigt pro Profil Zustand **und** Evidenz.
