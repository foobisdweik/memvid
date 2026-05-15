#!/usr/bin/env bash
# Memvid installer: copy bash tools into PREFIX/bin, write settings, create state dirs.
# Idempotent. Honors DESTDIR for packaging staging.
set -euo pipefail

DESTDIR="${DESTDIR:-}"
PREFIX="${PREFIX:-/usr/local}"
CONFIG_DIR="${CONFIG_DIR:-/etc/memvid}"
STATE_DIR="${STATE_DIR:-/var/lib/memvid}"
RUN_USER="${MEMVID_USER:-${SUDO_USER:-$(id -un)}}"
DRY_RUN="${DRY_RUN:-0}"

usage() {
  cat <<'EOF'
Usage:
  packaging/install.sh [--dry-run] [--prefix PATH] [--config-dir PATH] [--state-dir PATH] [--user USER]

Installs memvid bash tools, settings file, and state directories.

Environment overrides:
  DESTDIR      Staging root (prepended to all paths).
  PREFIX       Install prefix for bin/. Default: /usr/local.
  CONFIG_DIR   Settings directory. Default: /etc/memvid.
  STATE_DIR    State directory (shards, archive). Default: /var/lib/memvid.
  MEMVID_USER  Owner for state dirs. Default: $SUDO_USER or current user.
  DRY_RUN=1    Print actions without performing them.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)    DRY_RUN=1; shift ;;
    --prefix)     PREFIX="${2:?--prefix requires a value}"; shift 2 ;;
    --config-dir) CONFIG_DIR="${2:?--config-dir requires a value}"; shift 2 ;;
    --state-dir)  STATE_DIR="${2:?--state-dir requires a value}"; shift 2 ;;
    --user)       RUN_USER="${2:?--user requires a value}"; shift 2 ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "install.sh: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
bin_src="$repo_root/deploy/bin"
config_src="$repo_root/config/settings.toml"

bin_dst="${DESTDIR}${PREFIX}/bin"
config_dst="${DESTDIR}${CONFIG_DIR}"
state_dst="${DESTDIR}${STATE_DIR}"
shards_dst="$state_dst/shards"
archive_dst="$state_dst/archive"

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$*"
  else
    "$@"
  fi
}

is_root=0
if [[ "$(id -u)" -eq 0 ]]; then
  is_root=1
fi

if [[ "$DRY_RUN" -ne 1 && -z "$DESTDIR" && "$is_root" -ne 1 && "${PREFIX#/usr}" != "$PREFIX" ]]; then
  echo "install.sh: writing to $PREFIX requires root; rerun with sudo or set PREFIX=\$HOME/.local or stage with DESTDIR=" >&2
  exit 1
fi

run mkdir -p "$bin_dst" "$config_dst" "$shards_dst" "$archive_dst"

for tool in memvid-write memvid-context claude-memvid codex-memvid gemini-memvid; do
  src="$bin_src/$tool"
  if [[ ! -f "$src" ]]; then
    echo "install.sh: missing source: $src" >&2
    exit 1
  fi
  run install -m 0755 "$src" "$bin_dst/$tool"
done

if [[ ! -f "$config_dst/settings.toml" ]]; then
  run install -m 0644 "$config_src" "$config_dst/settings.toml"
else
  printf 'install.sh: keeping existing %s\n' "$config_dst/settings.toml"
fi

if [[ "$is_root" -eq 1 && "$DRY_RUN" -ne 1 ]]; then
  if id -u "$RUN_USER" >/dev/null 2>&1; then
    chown -R "$RUN_USER":"$RUN_USER" "$state_dst"
  fi
fi

run chmod 0755 "$shards_dst" "$archive_dst"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "DRY-RUN: install complete (no changes made)."
else
  cat <<EOF
install.sh: done.
  bin:     $bin_dst
  config:  $config_dst/settings.toml
  shards:  $shards_dst
  archive: $archive_dst
EOF
fi
