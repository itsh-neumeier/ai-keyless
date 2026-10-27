# Lab-Setup auf dem PC des Projektverantwortlichen (Windows)

> Ziel: AI-Keyless läuft **im LAN der UNVR** (192.168.10.181), nutzt die NVIDIA T600 über Ollama,
> und eine **Claude-Code-Sitzung auf demselben PC** führt die nativen Tests nach Testplan aus.
> Die Cloud-Sitzung (Entwicklung) erreicht das Heimnetz nicht; Live-Tests laufen nur hier.
>
> Arbeitsverzeichnis: **`D:\GitHub\ai-keyless`**

## 1. Voraussetzungen (Windows)

| Komponente | Einrichtung | Prüfung |
|---|---|---|
| Docker Desktop | WSL2-Backend aktiv (Einstellungen → General → „Use the WSL 2 based engine“) | `docker version` in PowerShell |
| NVIDIA T600 | aktueller NVIDIA-Treiber für Windows (bringt WSL2-GPU-Support mit) | `docker run --rm --gpus all nvidia/cuda:12.6.3-base-ubuntu24.04 nvidia-smi` zeigt die T600 |
| Git for Windows | inkl. **Git Bash** (für die `scripts/*.sh`) | `git --version` |
| Claude Code | Claude-Desktop-App oder Claude Code CLI | — |
| Coral USB | später (AKL-M100-02); unter Windows nur über `usbipd-win` in WSL2 durchreichbar | — |

Zeilenenden: Das Repo erzwingt per `.gitattributes` LF für Skripte. Wurde vor dieser Änderung
geklont, einmal neu auschecken: `git rm --cached -r . && git reset --hard` (in Git Bash, im Repo).

## 2. Repository und AI-Keyless starten

In **Git Bash**:

```bash
cd /d/GitHub
git clone https://github.com/itsh-neumeier/ai-keyless.git   # bzw. vorhandenes Repo: git pull
cd ai-keyless
cp deploy/compose/.env.example deploy/compose/.env
scripts/init-secrets.sh                 # erzeugt deploy/compose/secrets/ (nicht in Git)
scripts/docker-build.sh                 # Image ai-keyless:dev bauen (einige Minuten)
docker compose -f deploy/compose/compose.yaml -f deploy/compose/compose.lab.yaml \
  --env-file deploy/compose/.env up -d
docker compose -f deploy/compose/compose.yaml exec api ai-keyless-api setup-token
```

Danach im Browser http://127.0.0.1:8090 öffnen, Token eingeben, Admin-Konto anlegen.

PowerShell-Hinweis: Die `docker compose …`-Befehle funktionieren auch in PowerShell; die Zeilenfortsetzung
ist dort `` ` `` statt `\`. Die `scripts/*.sh` nur in Git Bash ausführen.

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

In PowerShell:

```powershell
cd D:\GitHub\ai-keyless
claude remote-control
```

(oder die Claude-Desktop-App mit dem Ordner `D:\GitHub\ai-keyless` öffnen). Die Sitzung erscheint in der
Claude-Code-App. Als erste Nachricht:

> Lies `docs/ai-memory/PC-SESSION-BRIEF.md` und führe Testplan TP-000 aus (freigegeben).

## 5. Netzwerk unter Windows – wichtig für die spätere Adoption

Für Discovery/Adoption muss die UNVR AI-Keyless **direkt im LAN** erreichen (UDP-Broadcast im selben
Segment, eingehende Verbindung). Docker Desktop betreibt Container in einer WSL2-VM hinter NAT:

| Variante | Admin-UI + Ollama | Discovery/Adoption | Bewertung |
|---|---|---|---|
| Docker Desktop (Standard, dieses Lab) | ✓ | ✗ (NAT, kein LAN-Broadcast) | reicht für TP-000 und Modelltests |
| Docker Desktop „host networking“ | ✓ | voraussichtlich ✗ (Host = WSL2-VM, nicht das LAN) | `needs_evidence` |
| WSL2 mit `networkingMode=mirrored` + Docker Engine in WSL | ✓ | unklar (Broadcast-Weiterleitung) | `needs_evidence` (AKL-M080-01) |
| Linux-VM (Hyper-V, externer vSwitch) bzw. NAS mit eigener LAN-IP | ✓ | voraussichtlich ✓ | empfohlen für das Geräte-Profil |

Für TP-000 genügt das Standard-Lab. Vor TP-001 wird anhand von AKL-M080-01 entschieden, wo das Geräte-Profil läuft.

## 6. Sicherheit

- Admin-UI bleibt auf `127.0.0.1` gebunden, solange kein TLS-Proxy davor ist.
- Keine Ports der UNVR ins Internet öffnen; keine Tunnel nötig.
- `deploy/compose/secrets/` und `deploy/compose/.env` niemals committen (stehen in `.gitignore`).
- Live-Tests nur nach freigegebenem Testplan; Rückweg steht in jedem Plan.
