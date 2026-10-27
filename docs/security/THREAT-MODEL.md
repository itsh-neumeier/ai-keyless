# Bedrohungsmodell (STRIDE)

> Stand: 2026-10-26 · Gilt für AI-Keyless 0.1.x; bei jedem Meilenstein mit neuer Schnittstelle aktualisieren.

## 1. Assets

| Asset | Schutzbedarf |
|---|---|
| Geräteidentität (privater Schlüssel, Zertifikate, Management-Credentials) | sehr hoch – Kompromittierung erlaubt Imitation gegenüber Protect |
| Controller-Zugangsdaten / API-Keys | sehr hoch |
| Kamerabilder, Clips, Audio, Personen-/Kennzeichendaten | sehr hoch (DSGVO Art. 9 bei Biometrie) |
| Provider-API-Keys (kostenpflichtig) | hoch |
| Admin-Sitzungen | hoch |
| Konfiguration, Audit-Log, Jobhistorie | mittel |

## 2. Vertrauensgrenzen

1. Browser ↔ Admin-API (LAN, ggf. Reverse-Proxy)
2. AI-Keyless ↔ UniFi-Protect-Controller (LAN, TLS mit Pinning)
3. AI-Keyless ↔ Modell-Provider (lokal oder Internet)
4. API ↔ PostgreSQL (internes Docker-Netz)
5. Container ↔ Host (Netzwerkmodus host/macvlan für Discovery)

## 3. Bedrohungen und Maßnahmen

| ID | STRIDE | Bedrohung | Maßnahme | Status |
|---|---|---|---|---|
| T1 | S | Fremder Controller gibt sich als Protect aus | Zertifikat-Pinning (SHA-256), Controller-Allowlist, Credentials erst nach Pin-Prüfung | geplant M040 |
| T2 | S | Unbefugter Admin-Zugriff | Setup nur mit Einmal-Token (256 Bit, 60 min, nur ohne bestehendes Konto, als SHA-256 gespeichert, Ausgabe nur per CLI-stdout), Pflicht-Login für alle API-Endpunkte außer Health, Argon2id, Rollen Admin/Operator/Viewer, HttpOnly/SameSite=Strict-Cookie (Secure per `AKL_COOKIE_SECURE`), Drosselung nach 5 Fehlversuchen (auch beim Passwortwechsel), konstante Prüfzeit bei unbekannten Konten; Deaktivieren beendet Sitzungen sofort, letzter aktiver Admin geschützt; Passwortwechsel beendet andere Sitzungen | ✓ M020-01/02/04 (synthetisch + E2E) |
| T3 | T | CSRF auf schreibende Admin-Aktionen | SameSite=Strict + Sitzungs-CSRF-Token (`x-csrf-token`, konstantzeitiger Vergleich) + Origin-/`Sec-Fetch-Site`-Prüfung für alle schreibenden Anfragen | ✓ M020-02 |
| T4 | R | Nicht nachvollziehbare Änderungen | `audit_log` mit Actor/Korrelation (Einrichtung, An-/Abmeldung, Fehlversuche, Kontoanlage), Zustandsübergänge in `device_state_transitions` | ✓ Auth; Geräte M040 |
| T5 | I | Secrets in Logs/API/Browser | Secret-Dateien (`*_FILE`), Debug-Redaktion der DB-URL ✓, keine Secrets in API-Antworten, kein Browserstorage für Secrets, gitleaks in CI; Geräteschlüssel nur im Identitätsverzeichnis (0600/0700, atomar angelegt, `Debug` redigiert, Konsistenzprüfung Schlüssel↔Zertifikat↔Profilart) ✓ M040-01 | teilweise ✓ |
| T6 | I | Medien gelangen in Logs/Browser/externe APIs | In-Memory-Verarbeitung, minimale Retention, Opt-in pro Kamera und Provider, keine Fremdmedien in CI | geplant M060/M070 |
| T7 | I/E | SSRF über frei konfigurierbare Provider-/Controller-URLs | Provider: nur http(s), keine Zugangsdaten/Query in URLs, Link-Local/Metadaten/0.0.0.0/Multicast/Broadcast immer gesperrt (auch IPv4-gemappt), Loopback nur per Opt-in, private Netze konfigurierbar, optionale Host-Allowlist, DNS einmal auflösen und auf geprüfte Adressen pinnen, keine Redirects, keine System-Proxys, Größen-/Zeitlimits; nur Admins dürfen Provider anlegen/testen | ✓ Provider (M060-02); Controller-URLs M040 |
| T8 | D | Kosten-/Lastexplosion durch Ereignisflut | Dauerhaftes globales Startlimit (Standard 12/h, serialisiert per Advisory Lock, auch bei parallelen Workern getestet), Idempotenzschlüssel gegen Duplikate, Backoff, Max-Versuche, Provider-Zeitlimit, Payload ≤ 16 KiB ohne Medien; Kostenlimits für Cloud-Provider offen (Q-011) | ✓ Queue (M060-01/03) |
| T9 | D | Polling-Sturm durch UI | SSE mit Server-seitiger Änderungsprüfung ✓ | ✓ |
| T10 | E | Container-Ausbruch / Rechteausweitung | distroless, nonroot, `read_only`, `cap_drop: ALL`, `no-new-privileges` ✓ | ✓ (amd64 getestet) |
| T11 | E | Netzwerkmodus host vergrößert Angriffsfläche | Nur Geräteschnittstelle im LAN, Admin-Port getrennt bindbar, Bewertung in NETWORKING.md | geplant M080 |
| T12 | T | Manipulierte Callbacks/Ergebnisse an Protect | Idempotenz, Signatur-/Herkunftsprüfung je Vertrag, keine Erfolgsmeldung ohne Readback | geplant M070 |
| T13 | S | Clickjacking/Content-Injection in UI | `X-Frame-Options: DENY`, CSP `frame-ancestors 'none'`, `nosniff`, `Referrer-Policy: no-referrer` ✓ | ✓ |
| T14 | – | Rechtliches Risiko durch Geräteimitation/RE | ADR-002, nur autorisierte Systeme, keine Umgehung von Lizenzprüfungen | ✓ Prozess |

Bekannte Restrisiken Authentifizierung: Die Drosselung ist pro Benutzername und im Speicher
(ein Angreifer kann ein Konto kurzzeitig sperren; Neustart setzt Zähler zurück). Ohne TLS sind
Passwörter im LAN mitlesbar – Betrieb im LAN nur hinter TLS-Proxy (docs/operations).

Bekannte Abweichung: CSP erlaubt derzeit `'unsafe-inline'` für Skripte (Theme-Init-Snippet und
SvelteKit-Bootstrap). Backlog AKL-M020-07: Hash-basierte CSP.

## 4. Sicherer Rückweg

Jedes Geräteprofil kann jederzeit in `disabled` überführt werden (Zustandsautomat erlaubt dies aus
jedem Zustand). Ein globaler Kill-Switch (`AKL_DEVICE_EMULATION=off`) ist für M040 vorgesehen.
