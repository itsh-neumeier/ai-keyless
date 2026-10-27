# Briefing für die Claude-Code-Sitzung auf dem PC des Projektverantwortlichen

Du arbeitest im Repo **AI-Keyless** auf dem PC im LAN der UniFi-Konsole (UNVR, 192.168.10.181).
Diese Sitzung existiert, weil die Cloud-Entwicklungssitzung das Heimnetz nicht erreicht.

## Pflichtlektüre

`CLAUDE.md`, `AGENTS.md`, `docs/ai-memory/PROJECT_STATE.md`, `docs/legal/RISK-ACCEPTANCE.md`,
`docs/testing/NATIVE-TESTPLAN-TEMPLATE.md`, `docs/operations/LAB-SETUP.md`.

## Aufgabe dieser Sitzung

1. Lab nach `docs/operations/LAB-SETUP.md` aufsetzen (mit dem Projektverantwortlichen).
2. **TP-000** (`docs/testing/plans/TP-000-unvr-recon.md`) ausführen – erst nach ausdrücklicher Freigabe im Chat.
3. Ergebnisse redigiert unter `docs/compatibility/evidence/` ablegen, `PROVIDERS.md` (T600-Messung) und
   `OPEN_QUESTIONS.md` (Q-003: exakte Protect-Version) aktualisieren, auf einem Branch committen und PR öffnen.
4. Spätere Live-Tests (TP-001 ff.) nur nach jeweils eigener Freigabe.

## Unverrückbare Regeln

- Keine Änderungen an der UNVR/Protect ohne freigegebenen Testplan; kein Login durch dich.
- Keine Netz-Scans, keine Paketmitschnitte ohne Freigabe.
- Keine Kamerabilder, Zugangsdaten, Seriennummern oder MAC-Adressen ins Repo.
- Ehrlichkeit: Ergebnisse so dokumentieren, wie sie sind (auch „funktioniert nicht“). `native_verified`
  nur mit vollständigem Nachweis inkl. Readback.
- Kein Code aus dem unlizenzierten Referenzprojekt (ADR-002).
