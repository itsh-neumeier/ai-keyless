# Nachweise (Evidence)

Hier liegen ausschließlich Nachweise für `native_verified`-Aussagen, je Datei ein durchgeführter
Testplan (`docs/testing/NATIVE-TESTPLAN-TEMPLATE.md`).

Dateiname: `JJJJ-MM-TT-<thema>.md`, z. B. `2026-11-03-ai_key-adoption-7.3.60.md`.

Pflichtinhalt: Testplan-ID, Umgebung (exakte Versionen), Schritte mit Beobachtungen,
Readback-Methode, redigierte Logs, Ergebnis je Fähigkeit, offene Risiken.

Ein Profileintrag mit `native_verified` muss im Feld `note` auf seine Datei hier verweisen
(`evidence/<datei>.md`); andernfalls lehnt `crates/protect-compat` die Profile beim Laden ab.

**Stand 2026-10-27: keine `native_verified`-Nachweise vorhanden.** Umgebungsdokumentation:
[`2026-10-27-tp000-umgebung.md`](2026-10-27-tp000-umgebung.md) (TP-000, UNVR mit Protect 7.2.105).
