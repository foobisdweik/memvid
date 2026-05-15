#!/usr/bin/env bash
# Shell tests for the agent launch wrappers: shard injection on launch.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/deploy/bin"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/memvid-wrappers.XXXXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

setup_case() {
  local name="$1"
  CASE_DIR="$TMP/$name"
  CASE_BIN="$CASE_DIR/bin"
  OUT_DIR="$CASE_DIR/out"
  mkdir -p "$CASE_BIN" "$OUT_DIR"

  # Symlink the wrappers and memvid-context into the case bin
  ln -s "$BIN/claude-memvid" "$CASE_BIN/claude-memvid"
  ln -s "$BIN/codex-memvid"  "$CASE_BIN/codex-memvid"
  ln -s "$BIN/gemini-memvid" "$CASE_BIN/gemini-memvid"
  ln -s "$BIN/memvid-context" "$CASE_BIN/memvid-context"
  ln -s "$BIN/memvid-write" "$CASE_BIN/memvid-write"

  # Fake real agent CLIs that just dump argv into a file
  local fake
  for fake in claude codex gemini; do
    cat > "$CASE_BIN/$fake" <<SCRIPT
#!/usr/bin/env bash
printf '%s\n' "\$@" > "$OUT_DIR/${fake}.out"
SCRIPT
    chmod +x "$CASE_BIN/$fake"
  done

  export MEMVID_SHARDS_DIR="$CASE_DIR/shards"
  export MEMVID_ARCHIVE_DIR="$CASE_DIR/archive"
  unset MEMVID_PROJECT
}

assert_contains() {
  local file="$1" needle="$2" label="$3"
  grep -F -- "$needle" "$file" >/dev/null || {
    sed -n '1,40p' "$file" >&2
    fail "$label: missing [$needle]"
  }
}

# ----- 1. Wrapper invokes real agent and injects shard body -----
setup_case "claude-basic"
shard_marker="MEMVID_TEST_MARKER_$(date +%s%N)"
printf '# Test Shard\n\nproject: demo\nmarker: %s\nbody filler to meet the minimum size\n' "$shard_marker" | \
  "$CASE_BIN/memvid-write" --project demo >/dev/null
cd "$TMP" && mkdir -p demo && cd demo
PATH="$CASE_BIN:$PATH" "$CASE_BIN/claude-memvid" >/dev/null
assert_contains "$OUT_DIR/claude.out" "$shard_marker" "claude-memvid injects shard body"
pass "claude-memvid invokes real claude with injected context"
cd "$ROOT"

# ----- 2. With user prompt after -- -----
setup_case "claude-prompt"
cd "$TMP" && mkdir -p demo2 && cd demo2
PATH="$CASE_BIN:$PATH" "$CASE_BIN/claude-memvid" -- "hello world" >/dev/null
assert_contains "$OUT_DIR/claude.out" "hello world" "user prompt forwarded"
assert_contains "$OUT_DIR/claude.out" "User request:" "user prompt section header"
pass "user prompt forwarded after --"
cd "$ROOT"

# ----- 3. Codex and Gemini wrappers behave the same -----
for agent in codex gemini; do
  setup_case "$agent-basic"
  cd "$TMP" && mkdir -p demo3 && cd demo3
  PATH="$CASE_BIN:$PATH" "$CASE_BIN/$agent-memvid" -- "test prompt for $agent" >/dev/null
  assert_contains "$OUT_DIR/$agent.out" "test prompt for $agent" "$agent prompt forwarded"
  pass "$agent-memvid wraps correctly"
  cd "$ROOT"
done

# ----- 4. Wrapper falls back gracefully when memvid-context fails -----
setup_case "fallback"
# point context at a directory with bad perms so it fails
export MEMVID_SHARDS_DIR=/no/such/path/that/cannot/be/created
cd "$TMP" && mkdir -p demo4 && cd demo4 || true
PATH="$CASE_BIN:$PATH" "$CASE_BIN/claude-memvid" >/dev/null 2>/dev/null || true
[[ -f "$OUT_DIR/claude.out" ]] || fail "wrapper should still exec agent on context failure"
pass "wrapper falls back on context failure"
cd "$ROOT"

echo "ALL TESTS PASSED"
