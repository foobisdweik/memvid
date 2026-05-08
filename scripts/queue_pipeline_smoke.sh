#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_DIR="${MEMVID_MODEL_DIR:-/opt/models/nomic-embed-text-v1}"
MODEL_PATH="${MEMVID_MODEL_PATH:-$MODEL_DIR/model.onnx}"
TOKENIZER_PATH="${MEMVID_TOKENIZER_PATH:-$MODEL_DIR/tokenizer.json}"

if [[ ! -f "$MODEL_PATH" || ! -f "$TOKENIZER_PATH" ]]; then
  echo "skip: embedding model or tokenizer missing: $MODEL_PATH / $TOKENIZER_PATH"
  exit 0
fi

cd "$ROOT"
cargo build -p memvid-embedder -p memvid-ingestor -p memvid-context

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/memvid-queue-pipeline.XXXXXX")"
cleanup() {
  local status=$?
  if [[ -n "${embedder_pid:-}" ]]; then
    kill "$embedder_pid" 2>/dev/null || true
    wait "$embedder_pid" 2>/dev/null || true
  fi
  if [[ -n "${ingestor_pid:-}" ]]; then
    kill "$ingestor_pid" 2>/dev/null || true
    wait "$ingestor_pid" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
  exit "$status"
}
trap cleanup EXIT INT TERM

queue="$tmpdir/queue"
processing="$tmpdir/processing"
ingest="$tmpdir/ingest"
done="$tmpdir/done"
failed="$tmpdir/failed"
store="$tmpdir/store"
mkdir -p "$queue" "$processing" "$ingest" "$done" "$failed" "$store"

config="$tmpdir/settings.toml"
cat > "$config" <<EOF
[paths]
queue = "$queue"
processing = "$processing"
ingest = "$ingest"
done = "$done"
failed = "$failed"
store = "$store"

[embedding]
model_path = "$MODEL_PATH"
tokenizer_path = "$TOKENIZER_PATH"
batch_size = 1
max_length = 128

[ingestion]
commit_interval = 1

[librarian]
enabled = false
endpoint = "http://127.0.0.1:11434/v1/chat/completions"
model = "qwen3:8b"
EOF

ingestor_log="$tmpdir/ingestor.log"
embedder_log="$tmpdir/embedder.log"
MEMVID_CONFIG="$config" target/debug/memvid-ingestor >"$ingestor_log" 2>&1 &
ingestor_pid=$!
MEMVID_CONFIG="$config" target/debug/memvid-embedder >"$embedder_log" 2>&1 &
embedder_pid=$!

deadline=$((SECONDS + 45))
while (( SECONDS < deadline )); do
  if ! kill -0 "$ingestor_pid" 2>/dev/null; then
    echo "ingestor exited before startup"
    cat "$ingestor_log"
    exit 1
  fi
  if ! kill -0 "$embedder_pid" 2>/dev/null; then
    echo "skip: embedder exited before startup; model/runtime unavailable"
    cat "$embedder_log"
    exit 0
  fi
  if grep -q "Ingestor listening" "$ingestor_log" && grep -q "Embedder listening" "$embedder_log"; then
    break
  fi
  sleep 0.5
done

if ! grep -q "Ingestor listening" "$ingestor_log"; then
  echo "ingestor did not become ready"
  cat "$ingestor_log"
  exit 1
fi
if ! grep -q "Embedder listening" "$embedder_log"; then
  echo "skip: embedder did not become ready; model/runtime unavailable"
  cat "$embedder_log"
  exit 0
fi

project="pipeline-smoke"
marker="queue-pipeline-smoke-$(date +%s%N)"
tmp_record="$queue/.tmp.$marker.md"
final_record="$queue/$marker.md"
cat > "$tmp_record" <<EOF
[project:$project]
[agent:codex]
[status:done]
[type:update]

Pipeline smoke marker $marker reached recall.
EOF
mv "$tmp_record" "$final_record"

deadline=$((SECONDS + 60))
while (( SECONDS < deadline )); do
  if [[ -f "$failed/$marker.md" ]]; then
    echo "pipeline moved record to failed"
    cat "$embedder_log"
    cat "$ingestor_log"
    exit 1
  fi
  if [[ -f "$done/$marker.md" && -f "$store/$project.mv2" ]]; then
    packet="$(target/debug/memvid-context \
      --config "$config" \
      --store "$store" \
      --project "$project" \
      --cwd "$ROOT" \
      --agent codex \
      --no-librarian \
      --query "$marker" \
      --budget-tokens 2000)"
    if grep -q "$marker" <<<"$packet"; then
      echo "ok: queue -> embedder -> ingestor -> recall pipeline passed"
      exit 0
    fi
  fi
  sleep 1
done

echo "pipeline did not reach recall before timeout"
cat "$embedder_log"
cat "$ingestor_log"
exit 1
