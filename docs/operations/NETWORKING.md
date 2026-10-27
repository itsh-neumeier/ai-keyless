# Netzwerkmodi (AKL-M080-01)

Frage: Welcher Docker-Netzmodus lässt ein LAN-Gerät (später: die Protect-Konsole) AI-Keyless per
UDP-Broadcast finden und per TCP erreichen?

## Synthetische Messung (`synthetic_verified`, 2026-10-27)

Werkzeug `akl-netprobe` (im Image unter `/usr/local/bin/akl-netprobe`), Ablauf `scripts/netlab.sh`:
eigene Linux-Bridge `br-akllab` (10.77.0.1/24) als LAN-Switch, „LAN-Gerät“ 10.77.0.50 als Container
per veth an der Bridge, macvlan an einer zweiten veth (wie eine Netzwerkkarte am Switch).
Protokollneutral (eigene Kennung `AKL-NETPROBE`), UDP 47001 / TCP 47002, kein Verkehr außerhalb der Bridge.

Gemessen auf dem Lab-PC (Docker Desktop 29.9.2, WSL2-Kernel 6.18.54.1, Image mit Commit dieses PRs),
zwei Läufe mit identischem Ergebnis; außerdem in jeder CI-Ausführung (Job „container“, Ergebnis
in der Job-Zusammenfassung):

| Modus | Broadcast 10.77.0.255 | Broadcast 255.255.255.255 | TCP aus dem LAN | Absender laut Container |
|---|---|---|---|---|
| host | ✓ | ✓ | ✓ (Host-IP 10.77.0.1) | echte LAN-Adresse 10.77.0.50 |
| bridge (`-p 47001/udp -p 47002/tcp`) | ✗ | ✗ | ✓ (veröffentlichter Port auf 10.77.0.1) | echte LAN-Adresse 10.77.0.50 |
| macvlan (eigene IP 10.77.0.130) | ✓ | ✓ | ✓ (eigene IP) | echte LAN-Adresse 10.77.0.50 |

Ausführen: Linux `sudo scripts/netlab.sh`; Windows/Docker Desktop `scripts\lab.ps1 netlab`.

## Bewertung

| Modus | Admin-UI | LAN-Discovery/Adoption (ab M040) | Bewertung |
|---|---|---|---|
| bridge (Standard-Compose) | ✓ (getestet 2026-10-26) | ✗ Broadcasts erreichen den Container nicht (gemessen); Unicast-TCP über veröffentlichte Ports geht | für Discovery **ungeeignet** (`synthetic_verified`) |
| host | ✓ | Broadcast + TCP ✓ (gemessen) | **Empfehlung für Linux-Server/NAS** (`synthetic_verified`); Port-Konflikte mit dem Host prüfen |
| macvlan (eigene LAN-IP) | ✓ | Broadcast + TCP ✓ (gemessen) | **Empfehlung, wenn eine eigene IP nötig ist** (z. B. AI Port, Port 443) (`synthetic_verified`); Host↔Container braucht ein Zusatzinterface (Linux-Eigenschaft, hier nicht gemessen) |

Ob die Protect-Konsole tatsächlich per Broadcast sucht und welche Ports sie nutzt, ist offen
(Q-006) – die Messung belegt nur, was der jeweilige Modus durchlässt.

## Einschränkung Docker Desktop (Windows/macOS)

Unter Docker Desktop laufen Container in einer VM. „host“ und macvlan beziehen sich auf das Netz
**dieser VM**, nicht auf die Netzwerkkarte des PCs; die Messung oben fand vollständig in der VM statt.
Ob Broadcasts aus dem echten LAN die VM erreichen, ist **nicht gemessen** (`needs_evidence`) –
erwartet wird nein (NAT). Für TP-001 vom Windows-Lab-PC aus ist daher eine native Messung nötig
(Issue #2, Schritt 3, eigene Freigabe) bzw. AI-Keyless auf einem Linux-Host im LAN.
