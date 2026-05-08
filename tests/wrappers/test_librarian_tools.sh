#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASH_BIN="$(command -v bash)"
TMP="${TMPDIR:-/tmp}/memvid-librarian-tools-$$"
mkdir -p "$TMP/bin" "$TMP/state/librarian_queue" "$TMP/config"
trap 'rm -rf "$TMP"' EXIT

cat > "$TMP/config/settings.toml" <<EOF
[paths]
queue = "$TMP/state/queue"
librarian_queue = "$TMP/state/librarian_queue"
processing = "$TMP/state/processing"
ingest = "$TMP/state/ingest"
done = "$TMP/state/done"
failed = "$TMP/state/failed"
store = "$TMP/state/store"

[embedding]
model_path = "/opt/models/model.onnx"
tokenizer_path = "/opt/models/tokenizer.json"
batch_size = 32
max_length = 512

[ingestion]
commit_interval = 32

[librarian]
enabled = true
endpoint = "http://127.0.0.1:11434/v1/chat/completions"
model = "qwen3:8b"
keep_alive = "-1"
EOF

printf 'Focus wrapper diagnostics.\n' | \
  MEMVID_CONFIG="$TMP/config/settings.toml" \
  bash "$ROOT/deploy/bin/memvid-librarian-note" \
    --agent codex \
    --project memvid \
    --intent recall_focus

record_count="$(find "$TMP/state/librarian_queue" -maxdepth 1 -type f -name '*.md' | wc -l)"
[[ "$record_count" -eq 1 ]]
! find "$TMP/state/librarian_queue" -maxdepth 1 -name '.tmp.*' | grep -q .
record="$(find "$TMP/state/librarian_queue" -maxdepth 1 -type f -name '*.md' | head -n1)"
grep -q '^\[agent:codex\]$' "$record"
grep -q '^\[project:memvid\]$' "$record"
grep -q '^\[intent:recall_focus\]$' "$record"
grep -q '^Focus wrapper diagnostics\.$' "$record"

if printf '\n' | MEMVID_CONFIG="$TMP/config/settings.toml" bash "$ROOT/deploy/bin/memvid-librarian-note" --agent codex --project memvid --intent recall_focus 2>/dev/null; then
  echo "empty librarian note unexpectedly succeeded" >&2
  exit 1
fi

cat > "$TMP/bin/ollama" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" > "$MEMVID_FAKE_OLLAMA_LOG"
EOF
chmod 0755 "$TMP/bin/ollama"

MEMVID_FAKE_OLLAMA_LOG="$TMP/ollama.log" \
MEMVID_CONFIG="$TMP/config/settings.toml" \
PATH="$TMP/bin:$PATH" \
bash "$ROOT/scripts/memvid-librarian-cold.sh"

grep -q '^stop qwen3:8b$' "$TMP/ollama.log"

mkdir -p "$TMP/fallback-bin"
ln -s /usr/bin/awk "$TMP/fallback-bin/awk"
cat > "$TMP/fallback-bin/curl" <<'EOF'
#!/bin/sh
set -eu
printf '%s\n' "$*" > "$MEMVID_FAKE_CURL_LOG"
EOF
chmod 0755 "$TMP/fallback-bin/curl"

MEMVID_FAKE_CURL_LOG="$TMP/curl.log" \
MEMVID_CONFIG="$TMP/config/settings.toml" \
PATH="$TMP/fallback-bin" \
"$BASH_BIN" "$ROOT/scripts/memvid-librarian-cold.sh"

grep -q 'http://127.0.0.1:11434/api/generate' "$TMP/curl.log"
grep -q '"model":"qwen3:8b","keep_alive":0' "$TMP/curl.log"
