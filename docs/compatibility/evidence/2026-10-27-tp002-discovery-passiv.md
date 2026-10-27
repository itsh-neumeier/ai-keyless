# TP-002: Discovery – passiver Empfang, Ergebnis 2026-10-27

> Beobachtung der **Controller-Seite**. Keine Fähigkeitsaussage für AI-Keyless (es hat nicht geantwortet).
> Redigiert: keine MAC-Adressen, Seriennummern oder Gerätenamen; Rohlogs nur lokal auf dem Lab-PC.

| Feld | Inhalt |
|---|---|
| Testplan | `docs/testing/plans/TP-002-discovery-passive.md` (freigegeben 2026-10-27) |
| Konsole | UNVR 192.168.10.181, UniFi OS 6.0.11, Protect 7.2.105 |
| Empfänger | Lab-PC 192.168.10.160/24, `scripts/discovery-listen.ps1` (UDP 10001, sendet nichts) |
| Gesendet durch AI-Keyless/Lab-PC | nichts |

## Läufe

| Lauf | Zeitraum | Ergebnis | Gültig? |
|---|---|---|---|
| 1 | 11:26:59–11:37:00 | 0 Datagramme | **nein** – Netzprofil „Öffentlich“ mit Eingangsregel „Windows PowerShell – Public – Block“; Loopback-Test zeigte, dass nur die Firewall blockierte |
| 2a | 11:38:06–11:38:17 (danach von Claude Code wegen Speichermangel beendet) | 51 Datagramme | ja (Netzprofil vorher auf „Privat“ gestellt, PowerShell für private Netze erlaubt) |
| 2b | 11:41:09–11:46:09 | 0 Datagramme | ja |

## Beobachtungen Lauf 2a

| Absender | Anzahl | Inhalt | Muster |
|---|---|---|---|
| **UNVR 192.168.10.181** (ein Quellport) | 3 | jeweils `01 00 00 00` (v1, Kommando 0, Länge 0) | 11:38:10, :11, :12 – 1-s-Takt |
| ein weiteres Gerät im Netz 192.168.231.0/24 (ein Quellport) | 12 / 12 | `01 00 00 00` und `02 08 00 00` paarweise | 1-s-Takt über den ganzen Lauf |
| Gateway-Schnittstellen (18 Adressen `x.x.x.1` verschiedener eigener Netze) | je 1 | `01 00 00 00` | einmalig um 11:38:10 |
| weitere Einzelabsender (3 Adressen) | 1–3 | `01 00 00 00` | – |

Alle Datagramme waren genau 4 Byte lang; keines enthielt TLVs.

## Ergebnis je Hypothese (`tests/contracts/discovery/hypotheses.json`)

| ID | Ergebnis |
|---|---|
| DISC-H01 (UDP 10001) | **beobachtet** – die UNVR (UniFi OS 6.0.11 / Protect 7.2.105) sendet Suchanfragen an UDP 10001 |
| DISC-H02 (v1 `01 00 00 00`) | **beobachtet** von der UNVR |
| DISC-H03 (v2 `02 08 00 00`) | im Netz beobachtet, **nicht** von der UNVR |
| DISC-H04/H05 (Antwortformat) | nicht prüfbar (keine Antworten aufgezeichnet, keine gesendet) |
| DISC-H07 (was Protect als AI-Key-Antwort erwartet) | offen |

## Offene Punkte / Einschränkungen

- **Auslöser unbekannt:** Die Suche kam in einem Schub (11:38:10–12), 11:41–11:46 kam nichts. Ob die
  Ansicht „Gerät hinzufügen“, UniFi Network oder ein Netzereignis (z. B. die Profiländerung des PCs gegen
  11:37) den Schub auslöste, ist nicht belegt. → Gezielter Folgelauf mit protokolliertem Auslöser.
- **Zieladresse nicht erfasst** (Broadcast/Unicast/Multicast): Das Skript speichert nur den Absender.
  Die Anfragen der Gateway-Schnittstellen aus fremden Subnetzen sprechen für Broadcast/Weiterleitung;
  belegt ist das nicht.
- Welcher Dienst auf der UNVR sendet (UniFi OS oder Protect), ist nicht unterscheidbar.
- Lauf 2a dauerte nur 11 s.
