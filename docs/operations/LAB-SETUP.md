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
| Git for Windows | Standardinstallation (Git Bash wird nicht benötigt) | `git --version` |
| Claude Code | Claude-Desktop-App oder Claude Code CLI | — |
| Coral USB | später (AKL-M100-02); unter Windows nur über `usbipd-win` in WSL2 durchreichbar | — |

Zeilenenden: Das Repo erzwingt per `.gitattributes` LF für `*.sh` und CRLF für `*.ps1`. Wurde vor
dieser Regel geklont, einmal neu auschecken: `git rm --cached -r . ; git reset --hard` (im Repo).

## 2. Repository und AI-Keyless starten

Alles in **PowerShell** (`scripts/lab.ps1`, keine Git Bash nötig):

```powershell
cd D:\GitHub
git clone https://github.com/itsh-neumeier/ai-keyless.git   # bzw. im vorhandenen Repo: git checkout main; git pull
cd ai-keyless
powershell -ExecutionPolicy Bypass -File scripts\lab.ps1 up      # .env + Secrets anlegen, Image bauen, Stack starten
powershell -ExecutionPolicy Bypass -File scripts\lab.ps1 token   # einmaliges Setup-Token
```

Danach im Browser http://127.0.0.1:8090 öffnen, Token eingeben, Admin-Konto anlegen.

Weitere Befehle: `status`, `logs`, `down` (Daten bleiben in den Volumes). Optionen: `-NoBuild`
(vorhandenes Image nutzen), `-NoGpu` (ohne Ollama-Overlay). Bereits vorhandene Secrets in
`deploy\compose\secrets\` werden nie überschrieben.

Der erste Build dauert einige Minuten (Rust-Release-Build im Container).

## 3. Modell auf der T600 bereitstellen

```powershell
powershell -ExecutionPolicy Bypass -File scripts\lab.ps1 model   # Standard: granite3.2-vision; anderes: -Model <name>
```

In AI-Keyless → *KI-Modelle*: Provider „Ollama T600“, Typ Ollama, URL `http://ollama:11434`,
Standardmodell `granite3.2-vision:latest` → *Verbindung testen* → *Bildtest*. Ergebnis und Latenz
in `docs/compatibility/PROVIDERS.md` eintragen (Zeile „NVIDIA T600“).

**VRAM der T600 (gemessen 2026-10-27):** Von 4 GB sind neben dem Windows-Desktop nur ~2,7 GiB frei.
`granite3.2-vision` setzt im Modelfile `num_ctx 16384`; damit landen nur 18 von 41 Schichten auf der GPU.
Gemessen: erster Bildtest nach dem Laden 27 s (einmal >120 s), danach 458 ms. `OLLAMA_CONTEXT_LENGTH` greift dagegen nicht. Kleineren Kontext über eine Modellvariante:

```powershell
docker compose -f deploy/compose/compose.yaml -f deploy/compose/compose.lab.yaml exec ollama sh -c "printf 'FROM granite3.2-vision\nPARAMETER num_ctx 4096\n' > /tmp/Modelfile && ollama create granite3.2-vision-4k -f /tmp/Modelfile"
```

Danach *Verbindung testen*, im Auswahlfeld `granite3.2-vision-4k:latest` wählen und *Bildtest*. Ob 4096 Token
für reale Kamerabilder reichen, ist offen (`needs_evidence`).

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
