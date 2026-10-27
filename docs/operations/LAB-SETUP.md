# Lab-Setup auf dem PC des Projektverantwortlichen

> Ziel: AI-Keyless läuft **im LAN der UNVR** (192.168.10.181), nutzt die NVIDIA T600 über Ollama,
> und eine **Claude-Code-Sitzung auf demselben PC** führt die nativen Tests nach Testplan aus.
> Die Cloud-Sitzung (Entwicklung) erreicht das Heimnetz nicht; Live-Tests laufen nur hier.

## 1. Voraussetzungen

| Komponente | Linux | Windows |
|---|---|---|
| Docker | Docker Engine + Compose-Plugin | Docker Desktop (WSL2-Backend) |
| NVIDIA T600 | aktueller NVIDIA-Treiber + [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html) | aktueller NVIDIA-Treiber für Windows; GPU-Support in Docker Desktop (WSL2) ist dann enthalten |
| Coral USB | USB-Durchreichung an den Container (später, AKL-M100-02) | in WSL2 nur über `usbipd-win`; später |
| Git, Claude Code | `git`, Claude Code CLI oder Claude-Desktop-App | Git for Windows, Claude-Desktop-App |

GPU prüfen: `docker run --rm --gpus all nvidia/cuda:12.6.3-base-ubuntu24.04 nvidia-smi` muss die T600 zeigen.

## 2. AI-Keyless starten

```bash
git clone https://github.com/itsh-neumeier/ai-keyless.git && cd ai-keyless
cp deploy/compose/.env.example deploy/compose/.env
scripts/init-secrets.sh                 # Windows: in WSL2/Git-Bash ausführen
scripts/docker-build.sh                 # Image ai-keyless:dev bauen (oder später GHCR-Release)
docker compose -f deploy/compose/compose.yaml -f deploy/compose/compose.lab.yaml \
  --env-file deploy/compose/.env up -d
docker compose -f deploy/compose/compose.yaml exec api ai-keyless-api setup-token
```

Dann http://127.0.0.1:8090 öffnen, Token eingeben, Admin anlegen.

## 3. Modell auf der T600 bereitstellen

```bash
docker compose -f deploy/compose/compose.yaml -f deploy/compose/compose.lab.yaml exec ollama ollama pull granite3.2-vision
```

In AI-Keyless → *KI-Modelle*: Provider „Ollama T600“, Typ Ollama, URL `http://ollama:11434`,
Standardmodell `granite3.2-vision:latest` → *Verbindung testen* → *Bildtest*. Ergebnis und Latenz
in `docs/compatibility/PROVIDERS.md` eintragen (Zeile „NVIDIA T600“).

`granite3.2-vision` (2B, ~2,4 GB) passt in 4 GB VRAM. Größere Modelle (z. B. 7B-Vision) passen nicht
vollständig in die T600 und laufen teilweise auf der CPU – messen statt annehmen (AKL-M080-06).

## 4. Claude-Code-Sitzung auf dem PC

Im Projektordner `claude remote-control` starten (oder die Claude-Desktop-App mit diesem Ordner
öffnen). Die Sitzung erscheint in der Claude-Code-App. Als erste Nachricht:

> Lies `docs/ai-memory/PC-SESSION-BRIEF.md` und führe Testplan TP-000 aus, sobald ich ihn freigebe.

## 5. Netzwerkmodus (später, für Discovery/Adoption)

Für die Adoption muss die UNVR AI-Keyless direkt im LAN erreichen (UDP-Discovery im selben Segment,
eingehende Verbindung). Der Standard-Stack (Bridge, Admin-UI nur auf 127.0.0.1) reicht dafür **nicht**;
das Geräte-Profil wird mit AKL-M040 im Modus `host` (Linux) bzw. mit eigener LAN-IP (macvlan) betrieben.
Unter Windows/WSL2 ist `host` eingeschränkt – Ergebnis von AKL-M080-01 abwarten.

## 6. Sicherheit

- Admin-UI bleibt auf `127.0.0.1` gebunden, solange kein TLS-Proxy davor ist.
- Keine Ports der UNVR ins Internet öffnen; keine Tunnel nötig.
- Live-Tests nur nach freigegebenem Testplan; Rückweg steht in jedem Plan.
