#!/usr/bin/env bash
# Erzeugt lokale Secret-Dateien für den Compose-Stack (nicht in Git!).
set -euo pipefail
DIR=${1:-deploy/compose/secrets}
mkdir -p "$DIR"
chmod 700 "$DIR"
if [[ -e "$DIR/db_password" ]]; then
  echo "Secrets existieren bereits in $DIR – nichts überschrieben." >&2
  exit 0
fi
umask 077
PW=$(head -c 32 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 32)
printf '%s' "$PW" > "$DIR/db_password"
printf 'postgres://akl:%s@db:5432/akl' "$PW" > "$DIR/database_url"
# Der API-Container läuft als nonroot (UID 65532) und muss die Datei lesen können.
chmod 644 "$DIR/database_url" "$DIR/db_password"
echo "Secrets erzeugt in $DIR"
