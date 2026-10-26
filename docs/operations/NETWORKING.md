# Netzwerkmodi (Bewertung offen – AKL-M080-01)

| Modus | Admin-UI | LAN-Discovery/Adoption (ab M040) | Bewertung |
|---|---|---|---|
| bridge (Standard-Compose) | ✓ (getestet 2026-10-26) | voraussichtlich ✗: Broadcast/Multicast erreicht den Container nicht, Container-IP ≠ beworbene IP | `needs_evidence` |
| host | ✓ | wahrscheinlich ✓ (Referenzbericht nutzt host auf NAS) | `needs_evidence`; Port-Konflikte mit Host prüfen |
| macvlan (eigene LAN-IP) | ✓ | wahrscheinlich ✓, eigene IP pro Profil (AI Port benötigt laut Bericht eigene IP + 443) | `needs_evidence`; Host↔Container-Kommunikation braucht Zusatzinterface |

Geplanter Messtest: synthetischer Discovery-Sender im selben L2-Segment, Prüfung von Empfang,
Antwortadresse und Erreichbarkeit der beworbenen Adresse pro Modus. Keine pauschale Annahme,
dass ein Docker-Bridge-Netz funktioniert.
