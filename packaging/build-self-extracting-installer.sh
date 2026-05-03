#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/memvid-sfx.XXXXXX")"
PAYLOAD="$WORK/payload"
OUT="${1:-$DIST/memvid-bootstrap-x86_64-linux.run}"

cleanup() {
  rm -rf "$WORK"
}
trap cleanup EXIT

msg() {
  printf '==> %s\n' "$*"
}

need_file() {
  if [[ ! -f "$1" ]]; then
    echo "Missing required file: $1" >&2
    exit 1
  fi
}

msg "Building release binaries"
if command -v cargo >/dev/null 2>&1; then
  cargo build --release --workspace
elif [[ -x /home/omen/.cargo/bin/cargo ]]; then
  env HOME=/home/omen USER=omen PATH=/home/omen/.cargo/bin:$PATH cargo build --release --workspace
else
  echo "cargo not found" >&2
  exit 1
fi

mkdir -p "$PAYLOAD/bin" "$PAYLOAD/lib" "$PAYLOAD/docs" "$PAYLOAD/model" "$PAYLOAD/source"

for bin in memvid-context memvid-embedder memvid-ingestor memvid-migrator; do
  need_file "$ROOT/target/release/$bin"
  install -m 0755 "$ROOT/target/release/$bin" "$PAYLOAD/bin/$bin"
done

for wrapper in memvid-context-wrap codex-memvid claude-memvid gemini-memvid; do
  need_file "$ROOT/deploy/bin/$wrapper"
  install -m 0755 "$ROOT/deploy/bin/$wrapper" "$PAYLOAD/bin/$wrapper"
done

for lib in libonnxruntime_providers_cuda.so libonnxruntime_providers_shared.so libonnxruntime_providers_tensorrt.so; do
  need_file "$ROOT/target/release/$lib"
  cp -L "$ROOT/target/release/$lib" "$PAYLOAD/lib/$lib"
done

for doc in AGENTS.md CLAUDE.md GEMINI.md; do
  need_file "$ROOT/$doc"
  install -m 0644 "$ROOT/$doc" "$PAYLOAD/docs/$doc"
done
need_file "$ROOT/docs/memvid-context.md"
install -m 0644 "$ROOT/docs/memvid-context.md" "$PAYLOAD/docs/memvid-context.md"
install -m 0755 "$ROOT/packaging/install-payload.sh" "$PAYLOAD/install.sh"

MODEL_ONNX="${MEMVID_MODEL_ONNX:-/opt/models/nomic-embed-text-v1/model.onnx}"
TOKENIZER_JSON="${MEMVID_TOKENIZER_JSON:-/opt/models/nomic-embed-text-v1/tokenizer.json}"
need_file "$MODEL_ONNX"
need_file "$TOKENIZER_JSON"
msg "Bundling model: $MODEL_ONNX"
cp -L "$MODEL_ONNX" "$PAYLOAD/model/model.onnx"
cp -L "$TOKENIZER_JSON" "$PAYLOAD/model/tokenizer.json"

msg "Bundling source snapshot"
tar -C "$ROOT" \
  --exclude='./.git' \
  --exclude='./target' \
  --exclude='./dist' \
  --exclude='./models' \
  --exclude='./.idea' \
  --exclude='*.mv2' \
  --exclude='*.mv2-*' \
  --exclude='*.mv2.*' \
  -czf "$PAYLOAD/source/memvid-source.tar.gz" .

cat > "$PAYLOAD/MANIFEST.txt" <<EOF
memvid self-extracting payload
created_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
git_commit=$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)
model=$MODEL_ONNX
tokenizer=$TOKENIZER_JSON
source_snapshot=source/memvid-source.tar.gz
EOF

ARCHIVE="$WORK/payload.tar.xz"
msg "Creating xz payload archive with -9e compression"
if ! command -v xz >/dev/null 2>&1; then
  echo "xz not found; install xz-utils/xz before building the self-extracting installer" >&2
  exit 1
fi
XZ_OPT="-9e" tar -C "$WORK" -cJf "$ARCHIVE" payload

mkdir -p "$(dirname "$OUT")"
cat > "$OUT" <<'STUB'
#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Memvid self-extracting installer

Usage:
  ./memvid-bootstrap-x86_64-linux.run [installer options]

Common options:
  --dry-run
  --no-deps
  --no-services
  --no-aliases
  --user USER
  --cachyos-nvidia installed|all|skip
  --nvidia-flavor open|closed|auto
  --help
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

tmp="${TMPDIR:-/tmp}/memvid-bootstrap.$$"
mkdir -p "$tmp"
cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT HUP INT TERM

line=$(awk '/^__MEMVID_ARCHIVE_BELOW__$/ { print NR + 1; exit 0; }' "$0")
if [ -z "$line" ]; then
  echo "Archive marker not found" >&2
  exit 1
fi

if ! command -v xz >/dev/null 2>&1; then
  echo "xz is required to extract this installer. Install xz/xz-utils and re-run." >&2
  exit 1
fi

tail -n +"$line" "$0" > "$tmp/payload.tar.xz"
tar -xJf "$tmp/payload.tar.xz" -C "$tmp"
exec "$tmp/payload/install.sh" "$@"

__MEMVID_ARCHIVE_BELOW__
STUB
cat "$ARCHIVE" >> "$OUT"
chmod 0755 "$OUT"

msg "Created $OUT"
du -h "$OUT"
