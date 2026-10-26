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

## Backup / Restore (vorläufig, Automatisierung in AKL-M080-04)

```bash
docker compose -f deploy/compose/compose.yaml exec -T db pg_dump -U akl -Fc akl > akl-$(date +%F).dump
docker compose -f deploy/compose/compose.yaml exec -T db pg_restore -U akl -d akl --clean < akl-YYYY-MM-DD.dump
```

Secrets-Verzeichnis separat sichern. Wiederherstellungsprobe: **noch nicht durchgeführt**.

## Upgrade / Rollback

Migrationen laufen beim Start automatisch und sind additiv geplant. Vor jedem Upgrade Backup
erstellen. Rollback = vorheriges Image + Restore des Backups (Down-Migrationen sind nicht vorgesehen).
