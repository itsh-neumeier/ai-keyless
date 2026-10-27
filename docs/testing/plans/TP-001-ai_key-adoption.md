# TP-001: AI-Key-Profil – Discovery und Adoption (Entwurf)

| Feld | Inhalt |
|---|---|
| Testplan-ID | TP-001 |
| Backlog-Issue | AKL-M040-05 |
| Ziel-Fähigkeiten | `discovery`, `adoption`, `keepalive` (Profil `ai_key`) |
| Erwartete Evidenzstufe danach | `native_verified` oder `unsupported` für die getestete Version |
| Status | **Entwurf – blockiert** (AKL-M040-01…04 nicht implementiert; Protect-Update auf 7.3.70 ausstehend) |
| Zielumgebung | UNVR 192.168.10.181, Produktivsystem mit 8 × G5 Turret Ultra (FW 5.5.31). Stand TP-000: UniFi OS 5.1.33, Protect 7.2.105; **vor TP-001 Update auf Protect 7.3.70**, exakte Versionen danach erneut ablesen |
| Freigabe | — |

## Hypothesen (aus Berichten Dritter, nicht unsere Evidenz)

- Protect findet ein AI-Key-Gerät per UDP-Discovery im selben L2-Segment (Q-006).
- Die Adoption läuft über HTTPS (berichtet: Port 8080) mit Zertifikats-Pinning; danach rotiert
  Protect Zugangsdaten (berichtet für 7.3.56). Ob die Zielversion 7.2.105 AI-Key-Geräte anbietet, ist unbekannt.
- Ein Kontrollkanal (berichtet: „UCP4“ über WebSocket) hält das Gerät online (Q-007).

## Mindestumgebung

Eigene Konsole mit exakt dokumentierter Protect-Version, AI-Keyless im Netzwerkmodus `host` oder
`macvlan` (Ergebnis aus AKL-M080-01), keine Produktivkameras nötig.

**Entscheidung 2026-10-27:** kein separates Test-VLAN – TP-001 läuft im Produktivnetz 192.168.10.0/24
(Discovery braucht dasselbe L2-Segment wie die UNVR). Auflagen: keine Änderung an Kameras oder
Aufnahmeeinstellungen; Rückweg (Schritt 6) ist Pflicht; der Projektverantwortliche erstellt vorher ein
Konfigurationsbackup der UNVR.

## Geplante Schritte (nach Implementierung von AKL-M040-01…04)

| # | Aktion | Erwartete Beobachtung |
|---|---|---|
| 1 | AI-Keyless starten, Profil `ai_key` aktivieren (Zustand `unconfigured → discovered` nur bei Discovery-Anfrage) | Übergang im `device_state_transitions`-Log mit Korrelation |
| 2 | In Protect nach neuen Geräten suchen | Gerät erscheint (oder nicht → `unsupported`/`needs_evidence` mit Paketmitschnitt) |
| 3 | Adoption in Protect starten | `pending_adoption → adopted`; Zertifikat-Fingerprint des Controllers vor Credential-Übergabe geprüft |
| 4 | 30 min warten | `online`, keine Fehler in Protect |
| 5 | AI-Keyless neu starten | `reconnecting → online` ohne erneute Adoption |
| 6 | Rückweg: Gerät in Protect entfernen, Profil `disabled` | Protect ohne Restartefakte |

## Readback

Protect-UI nach vollständigem Neuladen (Gerät online), Integration-API-Abfrage der Geräteliste,
Konsolen-Neustart und erneute Prüfung.
