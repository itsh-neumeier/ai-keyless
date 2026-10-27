# Vorlage: Nativer Testplan (Protect-Live-Test)

> Native Tests verändern ein echtes UniFi-Protect-System. Sie laufen **nie** in der CI und
> **nie** ohne schriftliche Freigabe des Projektverantwortlichen (Feld „Freigabe“ unten).
> Kopieren nach `docs/testing/plans/TP-<nr>-<thema>.md`.

## Kopf

| Feld | Inhalt |
|---|---|
| Testplan-ID | TP-xxx |
| Backlog-Issue | AKL-Mxxx-yy |
| Ziel-Fähigkeit(en) | z. B. `adoption` (Profil `ai_key`) |
| Erwartete Evidenzstufe danach | `native_verified` oder `unsupported` |
| Status | Entwurf / freigegeben / durchgeführt / abgebrochen |
| Freigabe (Name, Datum) | — |

## 1. Umgebung (vollständig, sonst kein Test)

| Element | Wert |
|---|---|
| Konsole (Modell, Firmware) | |
| UniFi-Protect-Version | exakt, z. B. 7.3.60 |
| Kameras (Modell, Firmware, Anzahl) | nur eigene Testkameras |
| Netz | Test-VLAN/Subnetz, Firewall-Regeln |
| AI-Keyless | Commit-SHA, Image-Digest, Netzwerkmodus (host/macvlan/bridge) |
| Modell-Provider | Server + Version, Modell |

## 2. Vorbedingungen und sicherer Rückweg

- Backup der Konsole (Protect-Backup) erstellt und Wiederherstellung bekannt.
- AI-Keyless-Backup (`scripts/backup.sh`).
- Rückweg schriftlich: Gerät in Protect entfernen/„vergessen“, AI-Keyless-Profil auf `disabled`,
  Container stoppen, ggf. Protect-Backup einspielen.
- Abbruchkriterien festgelegt (z. B. Protect-Fehlermeldungen, Kameras offline, unerwartete
  Konfigurationsänderungen).

## 3. Datenschutz

- Keine fremden Personen im Bild; Testszene bewusst inszeniert oder abgedeckt.
- Keine Bilder/Clips an Cloud-Provider ohne gesonderte Freigabe.
- Logs vor Ablage redigieren (IP-Adressen nach Bedarf, keine Tokens, keine Medien).

## 4. Schritte

| # | Aktion | Erwartete Beobachtung | Tatsächliche Beobachtung | OK? |
|---|---|---|---|---|
| 1 | | | | |

## 5. Readback (Pflicht für `native_verified`)

Wie wurde **unabhängig von AI-Keyless** geprüft, dass Protect das Ergebnis übernommen hat?
(z. B. Protect-UI nach vollständigem Neuladen, Integration-API-Abfrage, Neustart der Konsole und
erneute Prüfung). HTTP 200 allein ist kein Nachweis.

## 6. Ergebnis

- Ergebnis je Fähigkeit mit Evidenzstufe und **exakter** Protect-Version.
- Abweichungen, unerwartete Effekte, offene Risiken.
- Nachweisdatei: `docs/compatibility/evidence/<JJJJ-MM-TT>-<thema>.md`.
- Folgeänderungen: `crates/protect-compat/profiles/*.json` (Verweis auf die Nachweisdatei im
  Feld `note`, sonst lehnt die Validierung `native_verified` ab), `MATRIX.md`.
