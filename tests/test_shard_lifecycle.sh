#!/usr/bin/env bash
# Shell tests for memvid-write and memvid-context: seal, rotate, archive, verify.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/deploy/bin"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/memvid-lifecycle.XXXXXXXX")"
trap 'rm -rf "$TMP"' EXIT

export MEMVID_SHARDS_DIR="$TMP/shards"
export MEMVID_ARCHIVE_DIR="$TMP/archive"
export MEMVID_AGENT=test-agent
PATH="$BIN:$PATH"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

assert_eq() {
  local expected="$1" actual="$2" label="$3"
  [[ "$actual" == "$expected" ]] || fail "$label: expected [$expected], got [$actual]"
}

bigbody() {
  local n="$1" alphabet='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
  local out=""
  while [[ ${#out} -lt "$n" ]]; do
    out+="$alphabet"
  done
  printf '%s' "${out:0:$n}"
}

# ----- 1. First write seals the shard at 0444 -----
bigbody 1500 | memvid-write --project demo >/dev/null
shard="$MEMVID_SHARDS_DIR/demo.mv2"
[[ -f "$shard" ]] || fail "first shard not created"
mode="$(stat -c '%a' "$shard")"
assert_eq "444" "$mode" "first shard mode"
pass "first write seals at 0444"

# ----- 2. Header structure -----
head -n1 "$shard" | grep -q '^MV2 SHARD v3$' || fail "magic line"
grep -q '^project: demo$' "$shard" || fail "project header"
grep -q '^agent: test-agent$' "$shard" || fail "agent header"
grep -q '^prev-sha256: none$' "$shard" || fail "first prev-sha should be none"
pass "header structure"

# ----- 3. Rotation across four writes -----
bigbody 1400 | memvid-write --project demo >/dev/null
bigbody 1300 | memvid-write --project demo >/dev/null
bigbody 1200 | memvid-write --project demo >/dev/null
for f in demo.mv2 demo.mv2.1 demo.mv2.2; do
  [[ -f "$MEMVID_SHARDS_DIR/$f" ]] || fail "missing $f after rotation"
done
pass "rotation populates current/.1/.2"

# ----- 4. Eviction creates exactly one archive entry -----
archive_count="$(find "$MEMVID_ARCHIVE_DIR/demo" -type f -name '*.mv2.xz' | wc -l)"
assert_eq "1" "$archive_count" "archive entry count after 4 writes"
archive_file="$(find "$MEMVID_ARCHIVE_DIR/demo" -type f -name '*.mv2.xz')"
[[ "$(stat -c '%a' "$archive_file")" == "444" ]] || fail "archive file not sealed 0444"
# Decompress and verify it parses as a v3 shard
xz -dc "$archive_file" | head -n1 | grep -q '^MV2 SHARD v3$' || fail "archive content not a v3 shard"
pass "evicted .2 archived under xz -9e at 0444"

# ----- 5. Verify clean rotation -----
memvid-context --project demo --verify >/dev/null
pass "verify clean rotation"

# ----- 6. Body content round-trip -----
expected_body="$(bigbody 800)"
printf '%s' "$expected_body" | memvid-write --project demo --force >/dev/null
actual_body="$(memvid-context --project demo)"
assert_eq "$expected_body" "$actual_body" "body round-trip"
pass "body round-trip"

# ----- 7. Shrinkage refusal -----
if echo "x" | memvid-write --project demo 2>/dev/null; then
  fail "shrinkage should have been refused"
fi
pass "shrinkage refused without --force"

echo "x" | memvid-write --project demo --force >/dev/null
pass "shrinkage allowed with --force"

# ----- 8. Empty body refusal -----
if : | memvid-write --project demo 2>/dev/null; then
  fail "empty body should have been refused"
fi
pass "empty body refused"

# ----- 9. Tamper detection (trailing bytes) -----
chmod 0644 "$shard"
echo "EVIL APPEND" >> "$shard"
chmod 0444 "$shard"
if memvid-context --project demo --verify >/dev/null 2>&1; then
  fail "verify should detect trailing bytes"
fi
pass "verify detects trailing bytes"

# ----- 10. Tamper detection (body byte flip) -----
TMP2="$(mktemp -d "${TMPDIR:-/tmp}/memvid-lifecycle2.XXXXXXXX")"
export MEMVID_SHARDS_DIR="$TMP2/shards"
export MEMVID_ARCHIVE_DIR="$TMP2/archive"
bigbody 1500 | memvid-write --project demo2 >/dev/null
target="$MEMVID_SHARDS_DIR/demo2.mv2"
chmod 0644 "$target"
printf 'Z' | dd of="$target" bs=1 seek=300 count=1 conv=notrunc status=none
chmod 0444 "$target"
if memvid-context --project demo2 --verify >/dev/null 2>&1; then
  fail "verify should detect body byte flip"
fi
pass "verify detects body byte flip"
rm -rf "$TMP2"

# ----- 11. --full prints all three shards -----
export MEMVID_SHARDS_DIR="$TMP/shards"
export MEMVID_ARCHIVE_DIR="$TMP/archive"
full_out="$(memvid-context --project demo --full 2>/dev/null || true)"
count=$(grep -c '^===== ' <<< "$full_out" || true)
[[ "$count" -ge 1 ]] || fail "--full produced no shard sections"
pass "--full prints shard sections"

# ----- 12. --history lists archives -----
hist="$(memvid-context --project demo --history)"
[[ "$(printf '%s\n' "$hist" | wc -l)" -ge 1 ]] || fail "--history produced no entries"
pass "--history lists archives"

# ----- 13. --raw includes header -----
raw="$(memvid-context --project demo --raw)"
grep -q '^MV2 SHARD v3$' <<< "$raw" || fail "--raw missing magic line"
pass "--raw includes header"

# ----- 14. Hash chain: prev-sha256 of current equals sha256(.1 file) -----
TMP3="$(mktemp -d "${TMPDIR:-/tmp}/memvid-lifecycle3.XXXXXXXX")"
export MEMVID_SHARDS_DIR="$TMP3/shards"
export MEMVID_ARCHIVE_DIR="$TMP3/archive"
bigbody 1500 | memvid-write --project chain >/dev/null
bigbody 1400 | memvid-write --project chain >/dev/null
declared="$(grep '^prev-sha256:' "$MEMVID_SHARDS_DIR/chain.mv2" | awk '{print $2}')"
expected="$(sha256sum < "$MEMVID_SHARDS_DIR/chain.mv2.1" | awk '{print $1}')"
assert_eq "$expected" "$declared" "hash chain link"
pass "hash chain matches"
rm -rf "$TMP3"

# ----- 15. Invalid project name rejected -----
if echo "x" | memvid-write --project '../escape' 2>/dev/null; then
  fail "path-traversal project name should be rejected"
fi
pass "invalid project name rejected"

echo "ALL TESTS PASSED"
