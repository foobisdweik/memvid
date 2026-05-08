#!/usr/bin/env bash
set -euo pipefail

settings_path="${MEMVID_CONFIG:-/etc/memvid/settings.toml}"

if [[ ! -f "$settings_path" ]]; then
  echo "Memvid settings not found: $settings_path" >&2
  exit 1
fi

model="$(
  awk -F'=' '
    /^\[librarian\]/ { in_librarian=1; next }
    /^\[/ { in_librarian=0 }
    in_librarian && $1 ~ /^[[:space:]]*model[[:space:]]*$/ {
      gsub(/"/, "", $2)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2)
      print $2
      exit
    }
  ' "$settings_path"
)"

if [[ -z "$model" ]]; then
  echo "No librarian model configured in $settings_path" >&2
  exit 1
fi

if command -v ollama >/dev/null 2>&1; then
  exec ollama stop "$model"
fi

endpoint="$(
  awk -F'=' '
    /^\[librarian\]/ { in_librarian=1; next }
    /^\[/ { in_librarian=0 }
    in_librarian && $1 ~ /^[[:space:]]*endpoint[[:space:]]*$/ {
      gsub(/"/, "", $2)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2)
      print $2
      exit
    }
  ' "$settings_path"
)"
base="${endpoint%%/v1/chat/completions}"
if [[ -z "$base" || "$base" == "$endpoint" ]]; then
  base="http://127.0.0.1:11434"
fi

json_model="${model//\\/\\\\}"
json_model="${json_model//\"/\\\"}"
curl -fsS "$base/api/generate" \
  -H 'Content-Type: application/json' \
  -d "{\"model\":\"$json_model\",\"keep_alive\":0}"
