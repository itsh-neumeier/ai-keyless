# Betrieb

> Status: AI-Keyless 0.1.0 ist ein **Entwicklungsstand** ohne Protect-Funktionen. Nicht produktiv einsetzen.

## Docker Compose (lokal/NAS)

```bash
cp deploy/compose/.env.example deploy/compose/.env
scripts/init-secrets.sh                      # erzeugt deploy/compose/secrets/{db_password,database_url}
scripts/docker-build.sh                      # Image ai-keyless:dev bauen
docker compose -f deploy/compose/compose.yaml --env-file deploy/compose/.env up -d
# UI: http://127.0.0.1:8090
```

- Admin-Port ist standardmäßig **nur an 127.0.0.1** gebunden. Für LAN-Zugriff `AKL_ADMIN_BIND`
  bewusst setzen (z. B. LAN-IP des NAS) und per Firewall einschränken. Ohne TLS laufen Passwort und
  Sitzungscookie im Klartext durchs LAN → für LAN-Zugriff einen TLS-Reverse-Proxy vorschalten und
  `AKL_COOKIE_SECURE=true` setzen.

## Ersteinrichtung

```bash
docker compose -f deploy/compose/compose.yaml exec api ai-keyless-api setup-token
```

Gibt ein Einrichtungs-Token aus (einmalig, 60 Minuten gültig, nur solange noch kein Konto
existiert; ein neuer Aufruf macht ältere Tokens ungültig). Das Token wird **nur** auf stdout
ausgegeben, nicht ins Log geschrieben. In der UI Token, Benutzername und Passwort (≥ 12 Zeichen)
eingeben → erstes Administrator-Konto. Weitere Konten (Administrator, Operator, Betrachter) unter
*Einstellungen → Benutzer*.

Sitzungen: HttpOnly-Cookie `akl_session` (SameSite=Strict), 12 h Inaktivität, max. 7 Tage.
Anmeldung wird nach 5 Fehlversuchen pro Benutzername exponentiell gesperrt (30 s … 15 min,
Speicher, Neustart setzt zurück). Alle Anmelde- und Kontoereignisse landen in `audit_log`.
- Secrets liegen als Dateien außerhalb von Git (`.gitignore`), werden als Compose-Secrets gemountet.
- Container: nonroot, read-only Root-FS, `cap_drop: ALL`, `no-new-privileges`, Healthcheck
  (`ai-keyless-api healthcheck`).

## Lab auf dem PC (UNVR-Tests, GPU)

Siehe `LAB-SETUP.md` und Overlay `deploy/compose/compose.lab.yaml` (Ollama mit NVIDIA-GPU).

## KI-Provider (Modelle)

Unter *KI-Modelle* legen Administratoren Provider an (Typ Ollama oder OpenAI-kompatibel), testen
die Verbindung (Modellliste, keine Inferenz) und führen einen Bildtest mit einem eigenen
synthetischen Testbild aus (löst eine Inferenz aus). Ergebnisse werden gespeichert.

- **Erreichbarkeit:** Ollama auf dem Docker-Host ist aus dem Container z. B. über die Bridge-
  Gateway-Adresse (`http://172.17.0.1:11434`) oder die LAN-IP des Hosts erreichbar; Ollama dafür
  mit `OLLAMA_HOST=0.0.0.0:11434` starten und per Firewall auf das LAN beschränken.
- **SSRF-Richtlinie:** nur `http`/`https`, keine Zugangsdaten/Query in der URL; immer gesperrt:
  Link-Local (u. a. `169.254.169.254`), `0.0.0.0`, Multicast, Broadcast. Loopback nur mit
  `AKL_PROVIDER_ALLOW_LOOPBACK=true`, private Netze per `AKL_PROVIDER_ALLOW_PRIVATE` (Standard:
  erlaubt), optional Host-Allowlist `AKL_PROVIDER_ALLOWED_HOSTS`. Keine Weiterleitungen,
  keine System-Proxys, Antworten max. 2 MiB, Zeitlimit `AKL_PROVIDER_TIMEOUT_SECS`.
- **API-Keys:** nie in der UI oder DB. Datei in das Secret-Verzeichnis legen (Standard
  `/run/secrets`, `AKL_SECRETS_DIR`; in Compose als Secret mounten) und im Provider nur den
  Dateinamen angeben.
- Gemessene Modelle: `docs/compatibility/PROVIDERS.md`.

## Verarbeitungs-Queue

Jobs liegen persistent in PostgreSQL (`jobs`). Ein Worker im API-Prozess holt fällige Jobs
(Priorität, dann Alter), hält dabei eine Lease (Provider-Zeitlimit + 60 s) und beachtet ein
**globales, rollierendes Stundenlimit** (`AKL_JOBS_PER_HOUR`, Standard 12 – anfängliche
Sicherheitsvorgabe, keine Leistungsgrenze; `0` hält die Verarbeitung an). Das Limit zählt
Starts in `job_starts` und überlebt Neustarts.

| Zustand | Bedeutung |
|---|---|
| Wartend / Wiederholung geplant | fällig ab `next_attempt_at` (Backoff 30 s · 2^n, max. 1 h) |
| Läuft | Worker hält die Lease |
| Abgeschlossen / Fehlgeschlagen | Ende; Fehlgeschlagen = Dead Letter nach `max_attempts` oder nicht wiederholbarem Fehler |
| Unklar | Abbruch während eines Callbacks – **nie** automatisch wiederholt |
| Abgebrochen | durch Operator |

Abgelaufene Leases (Absturz/Neustart) werden minütlich erkannt: vor dem Callback → erneuter
Versuch, im Callback → *Unklar*. Operatoren können wartende/fehlgeschlagene/unklare Jobs
abbrechen und fehlgeschlagene/unklare nach Prüfung neu einreihen (UI *Verarbeitung*).

## Portainer

Stack aus `deploy/compose/compose.yaml` anlegen, Umgebungsvariablen aus `.env.example` setzen,
Secrets als Dateien auf dem Host bereitstellen (`AKL_SECRETS_DIR`). **Status: `planned`, nicht getestet** (AKL-M080-02).

## Health und Diagnose

| Endpunkt | Bedeutung |
|---|---|
| `GET /healthz` | Prozess lebt (Liveness) |
| `GET /readyz` | 200 wenn DB erreichbar und migriert (oder nicht konfiguriert), sonst 503 |
| `GET /api/v1/system/status` | Zustand API/DB/Geräteprofile |

Logs: `docker compose logs api` (JSON, mit `correlation_id` aus `x-request-id`). Log-Level über `AKL_RUST_LOG`.

## Backup / Restore

```bash
scripts/backup.sh /pfad/zu/backups          # → akl-<UTC-Zeit>.dump + .devices.tar + .json (Schema-Version, SHA-256)
scripts/restore.sh /pfad/zu/backups/akl-20261027T001500Z.dump
```

- **Sicherung:** `pg_dump` im Custom-Format aus dem laufenden DB-Container (konsistenter
  Snapshot), Metadaten mit Schema-Version und SHA-256. Dateien mit Modus 600, Verzeichnis 700.
  Enthält Passwort- und Token-**Hashes**, Audit-Log und Konfiguration → wie ein Geheimnis behandeln.
- **Geräteidentitäten** (Volume `akl-devices`) als `akl-<zeit>.devices.tar` mit eigener SHA-256
  (`devices_sha256`); Rechte und Eigentümer bleiben erhalten. Enthält **private Schlüssel** → die
  Sicherung nur verschlüsselt aufbewahren. Ohne sie erzeugt AI-Keyless neue Identitäten und Protect
  muss die Geräte neu adoptieren.
- **Nicht enthalten:** das Secret-Verzeichnis (`deploy/compose/secrets`) – separat sichern.
- **Wiederherstellung:** prüft beide Prüfsummen (manipulierte/defekte Dateien werden abgelehnt),
  stoppt die API, ersetzt Datenbank und Identitätsverzeichnis, startet die API und prüft Readiness und
  Schema-Version. Ältere Sicherungen ohne `devices_sha256` lassen die Identitäten unverändert. Nur auf
  gleiche oder neuere AI-Keyless-Versionen wiederherstellen.
- Andere Projekt-/Dateinamen: `AKL_COMPOSE_PROJECT`, `AKL_COMPOSE_FILES`, `AKL_ADMIN_PORT`.
- **Wiederherstellungsprobe:** `scripts/restore-test.sh` (auch in der CI): Daten und Geräteprofil
  anlegen → sichern → Volumes löschen → manipulierte Sicherungen (Dump bzw. Identitäten) müssen
  scheitern → wiederherstellen → Anmeldung mit den alten Konten, gleicher Fingerprint, Rechte 700/600
  für `nonroot`. Ergebnis 2026-10-27: erfolgreich (lokal, Docker 29.9.2).

## Geräteidentität (`AKL_DEVICE_DIR`)

Im Image `/var/lib/ai-keyless/devices` (Compose-Volume `akl-devices`, Eigentümer `nonroot`, Modus 0700).
Je Profil ein Unterverzeichnis mit `identity.json` (Profilart, SHA-256-Fingerprint), `cert.pem` und
`key.pem` (0600). Schlüssel stehen nie in der Datenbank, in Logs oder API-Antworten. Eine beschädigte
oder vertauschte Identität wird beim Laden erkannt und **nicht** still ersetzt.

## Geräteprofile und Kill-Switch

Profile (`ai_key`, `ai_port`) legt ein Admin über `POST /api/v1/devices` an; dabei entsteht zuerst die
Identität, dann das Profil (Zustand `unconfigured`). Zustände ändert nur der Lifecycle-Service; jeder
Übergang wird mit Akteur, Grund und Korrelations-ID protokolliert (`/api/v1/devices/{id}/transitions`).
Per API möglich sind nur der sichere Rückweg (`disable`) und dessen Aufhebung (`enable`).

**Kill-Switch (Notaus):** `PUT /api/v1/devices/kill-switch {"active": true}` deaktiviert sofort alle
Profile und blockiert Anlage und Reaktivierung, bis er wieder ausgeschaltet wird. Mit
`AKL_DEVICE_KILL_SWITCH=true` ist er erzwungen und per API nicht abschaltbar (z. B. bei Verdacht auf
Fehlverhalten gegenüber Protect).

## Geräte-Emulation (`AKL_DEVICE_EMULATION`, standardmäßig aus)

> Nur gegen den Simulator verifiziert (`synthetic_verified`). Gegen echte Protect-Installationen
> ungetestet – Einsatz dort nur im Rahmen von TP-001 mit Freigabe.

Mit `AKL_DEVICE_EMULATION=true` startet AI-Keyless für das aktive `ai_key`-Profil:

| Dienst | Standard-Bind | Variable | Voraussetzung |
|---|---|---|---|
| Discovery-Antwort (UDP) | `0.0.0.0:10001` | `AKL_DISCOVERY_BIND` | – |
| Adoptions-Endpunkt (TLS, eigenes Gerätezertifikat) | `0.0.0.0:8080` | `AKL_ADOPTION_BIND` | Controller konfiguriert |
| Kontrollkanal zum Controller (gepinnt) | – | – | Controller konfiguriert, Profil adoptiert |

Controller-Vertrauen: `AKL_CONTROLLER_IP`, `AKL_CONTROLLER_PORT`, `AKL_CONTROLLER_FINGERPRINT`
(SHA-256 des Controller-Zertifikats, vorher vom Admin geprüft). Nur alle drei gemeinsam; fehlen sie,
läuft nur Discovery und **kein** Endpunkt nimmt Zugangsdaten an. Das Token des Controllers liegt nach
der Adoption als `controller.token` (0600) neben der Identität und ist damit Teil des
Identitäts-Backups. Das Netz muss host oder macvlan sein (`NETWORKING.md`); im Bridge-Netz erreicht die
Discovery den Container nicht. Kill-Switch oder deaktiviertes Profil beenden alle drei Dienste.

`GET /api/v1/devices` zeigt den Emulationsstatus (`emulation`) und je Profil die gespeicherte Bindung
(`controller`: IP, Port, Fingerprint – nie das Token). **Gerät entfernen:** Profil deaktivieren
(`POST /api/v1/devices/{id}/disable`), dann `POST /api/v1/devices/{id}/forget-controller` (nur Admin,
nur bei `disabled`, auditiert); anschließend das Gerät auch in Protect entfernen.

## Upgrade / Rollback

Migrationen laufen beim Start automatisch und sind additiv geplant. Vor jedem Upgrade Backup
erstellen. Rollback = vorheriges Image + Restore des Backups (Down-Migrationen sind nicht vorgesehen).
