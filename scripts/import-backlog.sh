#!/usr/bin/env bash
# Legt offene/blockierte Backlog-Einträge als GitHub-Issues an.
# NUR nach ausdrücklicher Freigabe durch den Projektverantwortlichen ausführen.
# Voraussetzung: gh CLI angemeldet, jq installiert. Dry-Run ist Standard.
set -euo pipefail
REPO=${REPO:-itsh-neumeier/ai-keyless}
APPLY=${APPLY:-0}
FILE="$(dirname "$0")/../docs/roadmap/backlog.json"

jq -c '.issues[] | select(.status != "done")' "$FILE" | while read -r issue; do
  id=$(jq -r .id <<<"$issue"); title=$(jq -r .title <<<"$issue"); ms=$(jq -r .ms <<<"$issue")
  body=$(jq -r --arg dod "$(jq -r .dod "$FILE")" '"**Meilenstein:** \(.ms)\n**Priorität:** \(.prio)\n**Abhängigkeiten:** \(if (.deps|length)>0 then (.deps|join(", ")) else "–" end)\n\n### Abnahmekriterien\n\(.accept|map("- [ ] "+.)|join("\n"))\n\n### Definition of Done\n\($dod)"' <<<"$issue")
  if [[ "$APPLY" == 1 ]]; then
    gh issue create --repo "$REPO" --title "[$id] $title" --body "$body" --label "$ms" || true
  else
    echo "DRY-RUN: [$id] $title"
  fi
done
