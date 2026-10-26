#!/usr/bin/env python3
"""Erzeugt docs/roadmap/BACKLOG.md aus docs/roadmap/backlog.json."""
import json
import pathlib

root = pathlib.Path(__file__).resolve().parent.parent
data = json.loads((root / "docs/roadmap/backlog.json").read_text())
icons = {"done": "✅ erledigt", "open": "⬜ offen", "blocked": "⛔ blockiert", "in_progress": "🔄 in Arbeit"}
out = [
    "# Backlog",
    "",
    "> **Generiert** aus `backlog.json` via `scripts/gen-backlog-md.py` – nicht von Hand bearbeiten.",
    f"> Stand: {data['generated']}. Nach Anlage der GitHub-Issues (`scripts/import-backlog.sh`) sind diese führend.",
    "",
    f"**Definition of Done (jedes Issue):** {data['dod']}.",
    "",
    "Prioritäten: P0 kritisch · P1 hoch · P2 mittel · P3 später.",
    "",
]
for ms in data["milestones"]:
    items = [i for i in data["issues"] if i["ms"] == ms["id"]]
    if not items:
        continue
    out += [f"## {ms['id']} – {ms['title']}", "", "| ID | Titel | Prio | Status | Abhängigkeiten | Abnahmekriterien |", "|---|---|---|---|---|---|"]
    for i in items:
        deps = ", ".join(i["deps"]) or "–"
        acc = "<br>".join(i["accept"])
        out.append(f"| {i['id']} | {i['title']} | {i['prio']} | {icons[i['status']]} | {deps} | {acc} |")
    out.append("")
(root / "docs/roadmap/BACKLOG.md").write_text("\n".join(out))
print("BACKLOG.md aktualisiert")
