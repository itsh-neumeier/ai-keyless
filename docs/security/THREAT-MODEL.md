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
| T2 | S | Unbefugter Admin-Zugriff | Setup nur mit Einmal-Token, danach Pflicht-Login, Rollen Admin/Operator/Viewer, sichere Cookies (HttpOnly, SameSite=Strict, Secure) | geplant M020 |
| T3 | T | CSRF auf schreibende Admin-Aktionen | SameSite + CSRF-Token + Origin-Prüfung | geplant M020 |
| T4 | R | Nicht nachvollziehbare Änderungen | `audit_log` mit Actor/Korrelation, Zustandsübergänge in `device_state_transitions` | Schema ✓, Nutzung M020/M040 |
| T5 | I | Secrets in Logs/API/Browser | Secret-Dateien (`*_FILE`), Debug-Redaktion der DB-URL ✓, keine Secrets in API-Antworten, kein Browserstorage für Secrets, gitleaks in CI | teilweise ✓ |
| T6 | I | Medien gelangen in Logs/Browser/externe APIs | In-Memory-Verarbeitung, minimale Retention, Opt-in pro Kamera und Provider, keine Fremdmedien in CI | geplant M060/M070 |
| T7 | I/E | SSRF über frei konfigurierbare Provider-/Controller-URLs | Allowlist, Auflösung + Prüfung der Ziel-IP (Link-Local/Metadaten blocken), Redirects nicht automatisch folgen, nur http(s) | geplant M060 |
| T8 | D | Kosten-/Lastexplosion durch Ereignisflut | Start-Limit 12 Jobs/h, Queue-Grenzen, Timeouts, Backoff, Kostenlimits | geplant M060 |
| T9 | D | Polling-Sturm durch UI | SSE mit Server-seitiger Änderungsprüfung ✓ | ✓ |
| T10 | E | Container-Ausbruch / Rechteausweitung | distroless, nonroot, `read_only`, `cap_drop: ALL`, `no-new-privileges` ✓ | ✓ (amd64 getestet) |
| T11 | E | Netzwerkmodus host vergrößert Angriffsfläche | Nur Geräteschnittstelle im LAN, Admin-Port getrennt bindbar, Bewertung in NETWORKING.md | geplant M080 |
| T12 | T | Manipulierte Callbacks/Ergebnisse an Protect | Idempotenz, Signatur-/Herkunftsprüfung je Vertrag, keine Erfolgsmeldung ohne Readback | geplant M070 |
| T13 | S | Clickjacking/Content-Injection in UI | `X-Frame-Options: DENY`, CSP `frame-ancestors 'none'`, `nosniff`, `Referrer-Policy: no-referrer` ✓ | ✓ |
| T14 | – | Rechtliches Risiko durch Geräteimitation/RE | ADR-002, nur autorisierte Systeme, keine Umgehung von Lizenzprüfungen | ✓ Prozess |

Bekannte Abweichung: CSP erlaubt derzeit `'unsafe-inline'` für Skripte (Theme-Init-Snippet und
SvelteKit-Bootstrap). Backlog AKL-M020-07: Hash-basierte CSP.

## 4. Sicherer Rückweg

Jedes Geräteprofil kann jederzeit in `disabled` überführt werden (Zustandsautomat erlaubt dies aus
jedem Zustand). Ein globaler Kill-Switch (`AKL_DEVICE_EMULATION=off`) ist für M040 vorgesehen.
