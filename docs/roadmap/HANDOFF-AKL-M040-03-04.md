# Übergabe: Discovery-Antwort und Adoption (AKL-M040-03, AKL-M040-04)

> **Stand 2026-10-27 (spät): synthetisch umgesetzt** in `crates/device-lifecycle/src/{discovery,adoption,control}.rs`
> (Branches `feat/AKL-M040-03-discovery-responder`, `feat/AKL-M040-04-adoption`). Offen: Verdrahtung in
> `akl-api`, Credential-Rotation, danach TP-001. Details: `docs/ai-memory/SESSION_HANDOFF.md`.

> Zweck: Diese Datei beschreibt die noch fehlende Geräte-Emulation, sodass eine **andere**
> Claude-Code-Sitzung oder ein Mensch sie umsetzen kann. Sie enthält keinen Protokollcode, sondern
> Vorgaben, vorhandene Bausteine, Akzeptanzkriterien und Auflagen.
>
> Hintergrund: In der Sitzung vom 2026-10-27 konnte der Protokoll-Baustein nicht geschrieben werden
> (ein Sicherheitsfilter stoppte die Arbeit an Discovery/Adoption wiederholt). Alles Drumherum ist
> fertig; es fehlt genau der Teil, der AI-Keyless gegenüber Protect als Gerät antworten lässt.

## Was schon existiert (nicht neu bauen)

| Baustein | Datei | Inhalt |
|---|---|---|
| Discovery-Codec | `crates/protect-compat/src/discovery.rs` | `Packet::decode/encode`, TLV-Typen, `Probe` (v1/v2), harte Grenzen, keine Panics |
| Discovery-Hypothesen | `tests/contracts/discovery/hypotheses.json` | DISC-H01…H07 mit Quelle + Evidenz; TP-002 belegt: UNVR sendet `01 00 00 00` an UDP 10001 |
| Beobachtung Controller-Seite | `docs/compatibility/evidence/2026-10-27-tp002-discovery-passiv.md` | echte Anfragen der UNVR (UniFi OS 6.0.11, Protect 7.2.105) |
| Geräteidentität | `crates/device-lifecycle/src/identity.rs` | Schlüssel + selbstsigniertes Zertifikat je Profil, Fingerprint |
| Lifecycle + Kill-Switch | `crates/device-lifecycle/src/service.rs` | `create_profile`, `transition`, Kill-Switch; einziger Weg für Zustandsänderungen |
| Zustandsautomat | `crates/core/src/device.rs` | erlaubte Übergänge (`unconfigured → discovered → pending_adoption → adopted → online …`) |
| Simulierter Controller | `crates/protect-sim/src/lib.rs` | TLS mit Fingerprint, Aufzeichnung, Fehlerfälle (`Normal/Silent/Reset`) |
| Netzmodus-Messung | `docs/operations/NETWORKING.md` | host/macvlan tauglich, bridge nicht (für Broadcast) |
| TLS-Pinning | `crates/protect-compat/src/tls.rs` | `Fingerprint`, Controller-Vertrauen per SHA-256 |

## AKL-M040-03: Discovery-Antwort (Ziel `synthetic_verified`)

Neues Modul, Vorschlag `crates/device-lifecycle/src/discovery.rs`.

1. UDP-Socket auf `0.0.0.0:10001` (nur im Netzmodus host/macvlan sinnvoll, siehe NETWORKING.md).
   Lauschen; **nur** auf erkannte Suchanfragen reagieren (`Packet::decode(..).probe()`), alles andere
   verwerfen. Kein aktives Senden ohne eingehende Anfrage.
2. Antwort als `Packet` bauen (via `encode`), mit TLVs aus der eigenen Identität und Profilart:
   Hostname, Produkt/Modell, Version, Konfigurationsstatus „unmanaged“. **Hypothese** – Feldwerte, die
   Protect von einem AI-Key-Gerät erwartet, sind unbekannt (DISC-H07). Antwort als Unicast an den
   Absender.
3. Zustandsübergang nur über den Lifecycle-Service: `unconfigured → discovered`, sobald eine gültige
   Suchanfrage beantwortet wurde. Keine eigenen Schreibzugriffe auf die DB.
4. Gegen `akl-protect-sim` testen: Sim sendet eine Probe, erwartet eine wohlgeformte Antwort; Decode
   der Antwort ergibt die gesetzten TLVs. Zusätzlich: fehlerhafte/fremde Pakete lösen **keine** Antwort
   und **keinen** Zustandswechsel aus.
5. Kill-Switch beachten: aktiv ⇒ nicht lauschen/nicht antworten.

## AKL-M040-04: Adoption, Keepalive, Reconnect (Ziel `synthetic_verified`)

1. Adoptions-Endpunkt (HTTPS, berichtet Port 8080) mit dem **eigenen** Zertifikat aus der Identität.
2. Controller-Vertrauen **vor** Übergabe von Zugangsdaten per Fingerprint prüfen (`tls::Fingerprint`,
   Bedrohung T1). Credentials erst nach bestätigtem Pin speichern (Secret-Datei, nie DB/Log).
3. Übergänge über den Lifecycle: `discovered → pending_adoption → adopted → online`. Readback: Nach
   Neustart muss `online` ohne erneute Adoption erreichbar sein (`reconnecting → online`).
4. Kontrollkanal-Keepalive (berichtet „UCP4“ über WebSocket, Q-007) – als Hypothese modellieren,
   `needs_evidence`.
5. Gegen `akl-protect-sim` testen, inkl. Neustart des Lifecycles und Rückweg (`→ disabled`).

## Unbedingte Auflagen

- **Clean Room bleibt für die Implementierung bestehen:** Codestruktur/Ausdruck eigenständig. Fakten
  aus `hypotheses.json`/PROVENANCE sind erlaubt, Herstellercode nicht.
- **Keine `native_verified`-Aussage** aus synthetischen Tests. Native Abnahme nur über TP-001 (unten).
- Secrets (Controller-Credentials, private Schlüssel) nie in DB, Logs oder API-Antworten.
- Jede Fähigkeit trägt Evidenzstufe + Version in der Matrix; `http_accepted ≠ persisted`.

## Danach: TP-001 (native Abnahme, AKL-M040-05)

Erst nach M040-03/04 und mit **eigener Freigabe** je Lauf. Vorbedingungen aus dieser Sitzung:
Protect-Update auf 7.3.70 **erledigt 2026-10-27** (UniFi OS 6.0.11, Kameras FW 5.5.43), Netzmodus host oder
macvlan im LAN 192.168.10.0/24, UNVR-Konfigurationsbackup vorher. Rückweg (Gerät entfernen) ist Pflicht.
Siehe `docs/testing/plans/TP-001-ai_key-adoption.md`.
