#!/usr/bin/env bash
# Shell tests for packaging/install.sh: DESTDIR staging, dry-run, idempotency.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/memvid-installer.XXXXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

# ----- 1. Dry-run prints actions and creates nothing -----
DESTDIR="$TMP/dryrun_dest" \
PREFIX=/usr/local CONFIG_DIR=/etc/memvid STATE_DIR=/var/lib/memvid \
DRY_RUN=1 bash "$ROOT/packaging/install.sh" --dry-run > "$TMP/dryrun.log" 2>&1
grep -q 'DRY-RUN:' "$TMP/dryrun.log" || { sed -n '1,40p' "$TMP/dryrun.log"; fail "dry-run did not log actions"; }
[[ ! -d "$TMP/dryrun_dest" ]] || fail "dry-run should not create DESTDIR contents"
pass "dry-run leaves filesystem untouched"

# ----- 2. Real staged install populates DESTDIR -----
DEST="$TMP/staged"
DESTDIR="$DEST" PREFIX=/usr/local CONFIG_DIR=/etc/memvid STATE_DIR=/var/lib/memvid \
  bash "$ROOT/packaging/install.sh" > "$TMP/install.log" 2>&1
for tool in memvid-write memvid-context claude-memvid codex-memvid gemini-memvid; do
  [[ -x "$DEST/usr/local/bin/$tool" ]] || fail "missing installed tool: $tool"
done
[[ -f "$DEST/etc/memvid/settings.toml" ]] || fail "settings.toml not installed"
[[ -d "$DEST/var/lib/memvid/shards" ]] || fail "shards dir not created"
[[ -d "$DEST/var/lib/memvid/archive" ]] || fail "archive dir not created"
pass "staged install lays down tools, config, state dirs"

# ----- 3. Re-run is idempotent and preserves existing settings -----
echo "# user-edited marker" >> "$DEST/etc/memvid/settings.toml"
DESTDIR="$DEST" PREFIX=/usr/local CONFIG_DIR=/etc/memvid STATE_DIR=/var/lib/memvid \
  bash "$ROOT/packaging/install.sh" > "$TMP/install2.log" 2>&1
grep -q 'user-edited marker' "$DEST/etc/memvid/settings.toml" || fail "user edits clobbered on re-run"
pass "re-install preserves existing settings.toml"

echo "ALL TESTS PASSED"
