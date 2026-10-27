# AI-Keyless UI – Designsystem

> Stand: 2026-10-26 · Meilenstein AKL-M020 (App-Shell bereits in M010 umgesetzt)
> Implementierung: `apps/web/src/app.css` (Tokens), `apps/web/src/lib/components/*`

## 1. Leitbild und Abgrenzung

Das Bediengefühl orientiert sich bewusst an der **Informationsarchitektur** moderner
NVR-/Protect-Weboberflächen: kompakte, einklappbare linke Navigation, schmale Kopfzeile,
ruhige dunkle Flächen, feine Trenner, dezente Statusfarben, tabellarische Listen, Karten/Kacheln.

**Rechte-/Referenzprüfung:** Es werden **keine** Hersteller-Assets, -Logos, -Icons, -Schriften,
-CSS oder -Design-Dateien verwendet oder nachgebaut. Farben, Abstände, Icons und Logo sind eigene
Entwürfe. Öffentliche Produktabbildungen dürfen nur als visuelle Orientierung für Layoutmuster
dienen (keine Pixel-/Maßübernahme). Die UI weist sich durchgängig als
**„Inoffiziell · unabhängig“** aus (Kopfzeile, Übersicht, System → „Independent / Unofficial“).
Name/Logo „AI-Keyless“ verhindern Verwechslung mit einem Herstellerprodukt.

## 2. Navigationsbaum

```text
Hauptbereich                      Admin
├─ Übersicht        (M010 ✓)      ├─ System         (M010 ✓, Über/Unofficial)
├─ Geräte           (Profile + Kompatibilitätsmatrix ✓; Lifecycle M040)        └─ Einstellungen  (M020, Theme/Sprache ✓)
├─ Kameras          (M050)
├─ Erkennungen      (M070)
├─ Suche            (M090)
├─ KI-Modelle       (Provider, Tests ✓ M060)
├─ Verarbeitung     (Queue, Limit, Aktionen ✓ M060)
└─ Alarmierung      (M100)
```

Nicht implementierte Bereiche zeigen einen ehrlichen Platzhalter („Noch nicht verfügbar“,
Meilenstein) und einen grauen Punkt in der Navigation – **niemals Beispieldaten**.

## 3. Layout / Wireframes

```text
┌────────┬───────────────────────────────────────────────────────────┐
│ ▲ Logo │ Seitentitel                    Inoffiziell · unabhängig ● │  Kopfzeile 48 px
├────────┼───────────────────────────────────────────────────────────┤
│ ▣ Über │  Untertitel                                               │
│ ▭ Gerä │  ┌─ Karte ───────┐ ┌─ Karte ───────┐ ┌─ Karte ───────┐    │  Kartenraster
│ ◫ Kame │  │ Label · Chip  │ │               │ │               │    │  1/2/3 Spalten
│ …      │  └───────────────┘ └───────────────┘ └───────────────┘    │
│        │  ┌─ Tabelle ─────────────────────────────────────────┐    │
│ ────── │  │ Kopf | Kopf | Kopf                                │    │
│ System │  │ Zeile mit Statuschips                             │    │
│ Einst. │  └───────────────────────────────────────────────────┘    │
│   ‹    │                                                           │
└────────┴───────────────────────────────────────────────────────────┘
 208 px / 56 px eingeklappt;  < 768 px: Off-Canvas-Drawer mit Overlay
```

Geplante Muster (M020+): Drawer für Detailansichten (rechts, 420 px), Dialoge für risikoreiche
Aktionen mit expliziter Bestätigung, Toasts (unten rechts), Kontextmenüs in Tabellenzeilen,
Kamera-/Ereigniskacheln 16:9 mit Status-Overlay, Tabs in Detailseiten, Suchfeld in der Kopfzeile.

## 4. Tokens

| Token | Dark | Light | Verwendung |
|---|---|---|---|
| `--akl-bg` | `#0f1013` | `#f4f5f7` | Seitenhintergrund |
| `--akl-surface` | `#17191d` | `#ffffff` | Karten, Navigation |
| `--akl-surface-2` | `#1d2025` | `#f0f1f4` | Eingabefelder, Skeletons |
| `--akl-border` | `#262a31` | `#e1e3e8` | Trenner |
| `--akl-text` | `#e7e9ee` | `#1b1d22` | Primärtext |
| `--akl-text-muted` | `#aab0bb` | `#4b505b` | Sekundärtext |
| `--akl-text-faint` | `#8d939e` | `#5f6571` | Hinweise, Tabellenköpfe |
| `--akl-accent` | `#5b9cff` | `#2459c7` | Aktiv, Links, primäre Buttons |
| `--akl-on-accent` | `#0b1426` | `#ffffff` | Text auf `accent` (Buttons) |
| `--akl-focus` | `#7fb2ff` | `#2f6fe4` | Fokusring |
| `--akl-ok` / `--akl-warn` / `--akl-danger` | `#3ccf7a` / `#f0b252` / `#ff6b6b` | `#137a3f` / `#8a5a0c` / `#b8302f` | Status (auch auf `*-soft`) |

Alle Text-Tokens erreichen ≥ 4,5:1 auf `bg`, `surface`, `surface-2`, `surface-hover` und – bei Status/Akzent –
auf der jeweiligen `*-soft`-Fläche (berechnet 2026-10-27, AKL-M020-06).

Tailwind-Utilities: `bg-surface`, `text-muted`, `border-line`, `bg-ok-soft text-ok` usw.
(`@theme inline` in `app.css`). **Keine Hex-Werte in Komponenten.**

- **Typografie:** Systemschrift (Inter falls installiert, sonst Segoe UI/System-UI), Basis 14 px,
  Tabellen 13 px, Labels 12 px Großbuchstaben mit Laufweite; Zahlen `tabular-nums` + Mono.
- **Abstände:** 4-px-Raster; Karten `p-4`, Tabellenzellen `px-4 py-2.5`, Raster-Gap 16 px.
- **Radien:** Karten 12 px, Buttons/Felder 6–8 px, Chips voll gerundet.
- **Schatten:** nur `shadow-card` (dezent).

## 5. Komponenten

| Komponente | Datei | Status |
|---|---|---|
| App-Shell (Sidebar, Topbar, Skip-Link) | `routes/+layout.svelte`, `Sidebar.svelte`, `Topbar.svelte` | ✓ |
| Icon (eigene 24×24-Linienicons) | `Icon.svelte`, `icons.ts` | ✓ |
| StatusChip (ok/warn/danger/neutral/accent) | `StatusChip.svelte`, `tone.ts` | ✓ |
| Platzhalter für nicht implementierte Bereiche | `Placeholder.svelte` | ✓ |
| Skeleton (Ladezustand) | inline `animate-pulse` | ✓ (Komponente M020) |
| Button (primary/secondary/ghost, busy) | `Button.svelte` | ✓ |
| TextField (Label, Hinweis, `aria-describedby`) | `TextField.svelte` | ✓ |
| FormError (`role=alert`) | `FormError.svelte` | ✓ |
| Einrichtungs-/Anmeldemaske | `AuthScreen.svelte` | ✓ |
| ConfirmButton (zweistufige Bestätigung) | `ConfirmButton.svelte` | ✓ (bis Dialog) |
| Benutzerverwaltung | `UsersSection.svelte` | ✓ |
| Select, Switch, Tabs, Drawer, Dialog, Toast, Menu, DataTable, CameraTile | – | M020-03 |

## 6. Zustandsdesign

| Zustand | Darstellung |
|---|---|
| Laden | Skeleton-Balken, keine Platzhalterwerte |
| Unbekannt / API offline | Chip „Unbekannt“ bzw. „API nicht erreichbar“ (rot), Werte „—“ |
| Nicht eingerichtet | neutraler Chip |
| Evidenz | eigener Chip pro Fähigkeit (`Geplant`, `Synthetisch verifiziert`, `Nativ verifiziert`, …) mit Tooltip |
| Fehler | Chip rot + Fehlercode (M020: Fehlercode-Katalog) |

Regel: Wenn die Live-Verbindung fehlt, werden zuvor empfangene Gerätezustände **nicht** als aktuell angezeigt.

## 7. Theme, i18n, Zugänglichkeit

- Dark (Standard/Hauptansicht), Light, System; Auswahl unter Einstellungen; Initialisierung vor dem ersten Paint (kein Aufblitzen).
- i18n: `de` (primär), `en` vollständig; Test erzwingt Schlüsselgleichheit. Neue Sprache = neues Wörterbuch.
- Tastatur: Skip-Link, sichtbarer Fokusring (`:focus-visible`), `aria-current` in der Navigation,
  `aria-expanded` am Einklapp-Button, Tabellen mit `scope`, `prefers-reduced-motion` respektiert.
- Kontrast und WCAG 2.1 A/AA: automatisches Audit mit axe (`@axe-core/playwright`) in der E2E-Suite – alle
  Seiten und der Anmeldebildschirm, jeweils Dark und Light; **jeder** Verstoß lässt die CI scheitern
  (AKL-M020-06). Ergebnis 2026-10-27: 0 Verstöße. Manuelle Prüfung (Screenreader, Tastaturpfade) ergänzt das,
  ersetzt sie aber nicht.
- Responsiv: keine horizontale Seiten-Scrollbar (E2E-Test bei 390 px für alle Seiten); breite Tabellen scrollen in ihrem Container. Der Container ist immer `relative overflow-x-auto`, sonst entkommen absolut positionierte Elemente (z. B. `sr-only`-Beschriftungen) dem Clipping und verbreitern die Seite.

## 8. Demo-/Mock-Modus

Noch nicht vorhanden. Geplant (AKL-M020-05): separater Build-Modus `AKL_DEMO=1` mit dauerhaft
sichtbarem Banner „Demo-Daten“; im Normalbetrieb technisch ausgeschlossen.

## 9. Medien

Rohvideo wird nicht durch die Admin-UI geleitet. Vorschaubilder nur bei Berechtigung, ohne
Logging von Medieninhalten (M070).
