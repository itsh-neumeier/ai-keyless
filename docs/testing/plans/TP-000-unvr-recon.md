# TP-000: UNVR-Bestandsaufnahme (nur lesend)

| Feld | Inhalt |
|---|---|
| Testplan-ID | TP-000 |
| Ziel | Umgebung für TP-001 exakt dokumentieren, **ohne** etwas an der UNVR zu verändern |
| Erwartete Evidenzstufe danach | keine Fähigkeitsänderung (reine Umgebungsdokumentation) |
| Status | **durchgeführt 2026-10-27** → `docs/compatibility/evidence/2026-10-27-tp000-umgebung.md` |
| Freigabe (Name, Datum) | Projektverantwortlicher, 2026-10-27 (Chat: „Ja darfst alles machen“) |

## Umgebung

| Element | Wert (auszufüllen) |
|---|---|
| Konsole | UNVR, 192.168.10.181, Firmware: … |
| UniFi-Protect-Version | exakt aus *Protect → Einstellungen → System* ablesen: … |
| Kameras (Modell, Firmware) | Liste aus der Protect-Oberfläche: … |
| Netz | Subnetz/VLAN des PCs und der UNVR: … |
| PC | Windows, Repo unter `D:\GitHub\ai-keyless`; Docker-Desktop-Version, `nvidia-smi`-Ausgabe (T600), Coral sichtbar (ja/nein) |
| AI-Keyless | Commit-SHA, Image-Digest |

## Erlaubte Aktionen (ausschließlich lesend)

1. Protect-Version, Konsolen-Firmware und Kameraliste **manuell in der Protect-Oberfläche ablesen** (Projektverantwortlicher).
2. Vom PC: Erreichbarkeit prüfen – PowerShell `Test-NetConnection 192.168.10.181 -Port 443` bzw. Git Bash `curl -sk -o /dev/null -w '%{http_code}' https://192.168.10.181/` (nur Erreichbarkeit/HTTP-Statuscode, kein Login).
3. PC-Umgebung erfassen: OS, Docker-Version, `docker run --rm --gpus all nvidia/cuda:12.6.3-base-ubuntu24.04 nvidia-smi`, USB-Geräteliste (Coral).
4. AI-Keyless-Lab nach `docs/operations/LAB-SETUP.md` starten; Bildtest mit `granite3.2-vision` auf der T600 ausführen und messen.

## Ausdrücklich nicht erlaubt

- Anmeldung an der UNVR durch Claude, Änderungen an Einstellungen, Geräte suchen/adoptieren, Kameras umkonfigurieren.
- Paketmitschnitte oder Scans im Netz.
- Speichern von Kamerabildern, Zugangsdaten oder Seriennummern im Repository.

## Ergebnis

Ablage: `docs/compatibility/evidence/<Datum>-tp000-umgebung.md` (redigiert: keine Seriennummern, keine MAC-Adressen,
keine Zugangsdaten). Danach TP-001 mit der exakten Protect-Version ergänzen.
