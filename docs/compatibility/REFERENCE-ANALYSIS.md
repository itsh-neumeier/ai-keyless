# AKL-M000 – Bestands- und Machbarkeitsbericht

> Stand: 2026-10-26 · Quellen: ausschließlich öffentliche Dokumentation (siehe PROVENANCE.md)
> Referenz-Commit: `4c71b73` (2026-09-24). Alle Angaben unten sind **Berichte Dritter**,
> nicht von AI-Keyless verifiziert → in unserer Matrix durchgehend `needs_evidence`.

## 1. Was das Referenzprojekt berichtet

| Aussage | Protect-Version | Umfang laut Bericht | Unsere Einordnung |
|---|---|---|---|
| AI-Key-Adoption, Online-Status | 7.3.56 (UDM Pro Max, Apple container 1.4.1) | inkl. Zeitsync über Kontrollkanal, Rotation des Management-Passworts, Factory-Auth deaktiviert, Reconnect nach geplantem Neustart | `needs_evidence` |
| On-demand-Beschreibung | 7.3.60 | einmaliger Versuch, G5 Flex, OpenAI | `needs_evidence` |
| Automatische Ereignisbeschreibung, Persistenz nach Reload | 7.3.60 | **eine** automatische Caption auf G5 Flex (Single-Use-Permit). G4 Instant laut eigener BUILD-RESULT-Anmerkung **nicht eindeutig belegt** | `needs_evidence` |
| AI-Port-Kandidat | 7.3.60 | 1× G3 gekoppelt, Person-Event über Integration API beobachtet; 100+ Modellaufrufe ohne native Events in anderem Versuch | `needs_evidence` |
| Statische Protokollanalyse | Controller 7.2.105, AI-Key-Firmware 2.2.8 | Paket 7.3.56 war nicht öffentlich abrufbar | Hinweis: Versionsdrift |

**Explizit nicht belegt (laut Referenz selbst):** Dauerbetrieb, native Find-Anything-Suche,
Docker/NAS-Betrieb, echte PostgreSQL-Transaktionen/Migrationen, Mehrkamera-Kopplung,
Fresh-Token-Adoption, Gesicht/LPR/Audio, Legacy-Kamera-Enhancement.

## 2. Öffentlich beschriebene Architekturmerkmale (Fakten, keine Implementierung)

- Management/Adoption über HTTPS (Port 8080 wird für den AI Key genannt), stabile lokale Identität.
- Kontrollkanal „UCP4“ über WebSocket.
- UDP-Discovery (read-only v1 queries), Controller-Allowlist; Multicast unter Linux optional.
- TLS mit Client-Zertifikaten; Controller-Vertrauen per manuell geprüftem SHA-256-Fingerprint;
  Credentials erst nach Pin-Prüfung.
- Callback-Status `http_accepted` statt „indexiert“.
- AI Port: eigene LAN-Adresse pro Instanz, HTTPS 443, Pairing über ausgehendes RTSP TCP 7447.
- Suche: E5-Text-Embeddings (384 Dim.) vs. legacy CLIP – welcher Vertrag nativ gilt, ist offen.
- Widersprüchliche Herstellerangaben: 1.000 vs. 1.800 Detektionen/Stunde.

Diese Punkte sind **Hypothesen** für unser Contract-Labor (AKL-M030). Jede wird dort als
eigener Contract-Test mit synthetischen Fixtures modelliert und erst durch einen
autorisierten Live-Test (`native_verified`) bestätigt.

## 3. Offizielle Herstellerdokumentation

`help.ui.com/…/29221435686039` (AI Key Setup and FAQs) war aus dieser Umgebung nicht abrufbar
(HTTP 403). → Offene Frage Q-004: Inhalt manuell sichten und Funktionsliste/Limits erfassen.

## 4. Risiken

| ID | Risiko | Wirkung | Gegenmaßnahme |
|---|---|---|---|
| R1 | Herstellerprotokolle ändern sich mit Protect-Updates | Adoption bricht | Versionsprofile, konservativer Modus bei unbekannter Version, Kill-Switch |
| R2 | Geräteimitation kann AGB/Nutzungsbedingungen berühren | rechtlich | Nur eigene/autorisierte Geräte, keine Umgehung von Lizenzprüfungen, Rechtsprüfung vor Release (Q-002) |
| R3 | Reverse-Engineering-Grenzen (DE/EU: §69e UrhG Interoperabilität) | rechtlich | Nur Black-Box-Beobachtung eigener Systeme, keine Firmware-Dekompilierung ohne Freigabe |
| R4 | Versionssplit 7.3.56 (Adoption) vs. 7.3.60 (Captions) | falsche Kompatibilitätsaussagen | Matrix pro Version, kein Übertragen von Evidenz |
| R5 | Docker-Bridge verhindert Discovery | Adoption scheitert | Netzmodus-Bewertung (AKL-M080-01), host/macvlan |
| R6 | Kosten/Datenschutz externer Vision-APIs | Kosten, DSGVO | Opt-in, 12 Jobs/h Startlimit, lokale Provider bevorzugt, keine Fremdmedien |

## 5. Go/No-Go-Gates

| Gate | Entscheidung |
|---|---|
| G1 Übernahme Referenzcode | **No-Go** (keine Lizenz) |
| G2 Eigenständige Implementierung aus öffentlicher Doku + Black-Box | **Go** (ADR-002) |
| G3 Projektfundament (M010) | **Go** – umgesetzt |
| G4 Protokollimplementierung AI Key (M040) | **Go nur nach** M030-Contract-Labor + autorisierter Testumgebung + Q-002 |
| G5 Native Live-Tests | **Blockiert** bis Testplan freigegeben und eigene Protect-Testumgebung benannt (Q-003) |
| G6 Öffentliches Release / GHCR | **Blockiert** bis Lizenzwahl (Q-001) und M120 |
