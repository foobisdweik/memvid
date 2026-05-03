#!/usr/bin/env bash
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
INSTALL_DEPS=1
INSTALL_SERVICES=1
INSTALL_ALIASES=1
PREFIX=/usr/local
CONFIG_DIR=/etc/memvid
STATE_DIR=/var/lib/memvid
MODEL_DIR=/opt/models/nomic-embed-text-v1
SOURCE_DIR=/opt/memvid/source
RUN_USER="${SUDO_USER:-}"
RUN_GROUP=""

if [[ -z "$RUN_USER" || "$RUN_USER" == "root" ]]; then
  if id omen >/dev/null 2>&1; then
    RUN_USER=omen
  else
    RUN_USER="$(id -un)"
  fi
fi

usage() {
  cat <<'EOF'
Memvid self-extracting installer

Options:
  --dry-run              Print actions without changing the system.
  --no-deps              Do not attempt package-manager dependency installation.
  --no-services          Do not install/enable/start systemd services.
  --no-aliases           Do not update root/user Bash aliases.
  --user USER            Service/data owner user. Defaults to omen when present.
  --prefix PATH          Install prefix for binaries/libs/share. Default: /usr/local.
  --config-dir PATH      Config directory. Default: /etc/memvid.
  --state-dir PATH       State directory. Default: /var/lib/memvid.
  --model-dir PATH       Model directory. Default: /opt/models/nomic-embed-text-v1.
  --source-dir PATH      Source snapshot extract directory. Default: /opt/memvid/source.
  -h, --help             Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --no-deps) INSTALL_DEPS=0 ;;
    --no-services) INSTALL_SERVICES=0 ;;
    --no-aliases) INSTALL_ALIASES=0 ;;
    --user) RUN_USER="${2:?--user requires a value}"; shift ;;
    --prefix) PREFIX="${2:?--prefix requires a value}"; shift ;;
    --config-dir) CONFIG_DIR="${2:?--config-dir requires a value}"; shift ;;
    --state-dir) STATE_DIR="${2:?--state-dir requires a value}"; shift ;;
    --model-dir) MODEL_DIR="${2:?--model-dir requires a value}"; shift ;;
    --source-dir) SOURCE_DIR="${2:?--source-dir requires a value}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
  esac
  shift
done

if [[ "$(id -u)" -ne 0 ]]; then
  echo "This installer needs root privileges. Re-run with sudo or as root." >&2
  exit 1
fi

if id "$RUN_USER" >/dev/null 2>&1; then
  RUN_GROUP="$(id -gn "$RUN_USER")"
else
  RUN_GROUP="$RUN_USER"
fi

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run]'
    printf ' %q' "$@"
    printf '\n'
  else
    "$@"
  fi
}

msg() {
  printf '==> %s\n' "$*"
}

warn() {
  printf 'WARN: %s\n' "$*" >&2
}

have() {
  command -v "$1" >/dev/null 2>&1
}

pkg_installed_deb() {
  dpkg -s "$1" >/dev/null 2>&1
}

install_deps() {
  [[ "$INSTALL_DEPS" -eq 1 ]] || return 0
  msg "Checking runtime dependencies"

  local missing_tools=()
  for tool in tar sed awk find chmod chown install; do
    have "$tool" || missing_tools+=("$tool")
  done
  if [[ "${#missing_tools[@]}" -gt 0 ]]; then
    warn "Missing basic tools: ${missing_tools[*]}"
  fi

  if [[ -f /etc/debian_version ]] && have apt-get; then
    local pkgs=(ca-certificates libcublas12 libcublaslt12 libcudart12 libcufft11 nvidia-cudnn)
    local needed=()
    for pkg in "${pkgs[@]}"; do
      pkg_installed_deb "$pkg" || needed+=("$pkg")
    done
    if [[ "${#needed[@]}" -gt 0 ]]; then
      msg "Installing Debian/Ubuntu CUDA runtime packages: ${needed[*]}"
      warn "NVIDIA CUDA/cuDNN packages may require accepting NVIDIA license terms through your distribution packages."
      run env DEBIAN_FRONTEND=noninteractive apt-get update
      run env DEBIAN_FRONTEND=noninteractive apt-get install -y "${needed[@]}"
    fi
  elif have dnf; then
    warn "DNF detected. CUDA/cuDNN package names vary by repository; install CUDA 12 runtime and cuDNN 9 if provider loading fails."
  elif have pacman; then
    warn "Pacman detected. Install NVIDIA driver, CUDA runtime, and cuDNN if provider loading fails."
  elif have zypper; then
    warn "Zypper detected. Install NVIDIA driver, CUDA 12 runtime, and cuDNN 9 if provider loading fails."
  elif have apk; then
    warn "Alpine/musl is not supported by these glibc-built binaries. Use a glibc distribution or rebuild from source."
  else
    warn "No supported package manager detected. Dependency installation skipped."
  fi
}

write_settings() {
  local settings="$CONFIG_DIR/settings.toml"
  run install -d -o root -g root -m 0755 "$CONFIG_DIR"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] write $settings"
    return 0
  fi
  cat > "$settings" <<EOF
[paths]
queue = "$STATE_DIR/queue"
processing = "$STATE_DIR/processing"
ingest = "$STATE_DIR/ingest"
done = "$STATE_DIR/done"
failed = "$STATE_DIR/failed"
store = "$STATE_DIR/store"

[embedding]
model_path = "$MODEL_DIR/model.onnx"
tokenizer_path = "$MODEL_DIR/tokenizer.json"
batch_size = 32
max_length = 512

[ingestion]
commit_interval = 32
EOF
  chmod 0644 "$settings"
}

install_files() {
  msg "Installing Memvid files"
  run install -d -o root -g root -m 0755 "$PREFIX/bin" "$PREFIX/lib/memvid" "$PREFIX/share/memvid" "$PREFIX/share/memvid/docs" "$MODEL_DIR" "$SOURCE_DIR"
  run install -o root -g root -m 0755 "$SELF_DIR/bin/"* "$PREFIX/bin/"
  run install -o root -g root -m 0644 "$SELF_DIR/lib/"*.so "$PREFIX/lib/memvid/"
  run install -o root -g root -m 0644 "$SELF_DIR/docs/AGENTS.md" "$SELF_DIR/docs/CLAUDE.md" "$SELF_DIR/docs/GEMINI.md" "$PREFIX/share/memvid/"
  run install -o root -g root -m 0644 "$SELF_DIR/docs/memvid-context.md" "$PREFIX/share/memvid/docs/"
  run install -o root -g root -m 0644 "$SELF_DIR/model/model.onnx" "$MODEL_DIR/model.onnx"
  run install -o root -g root -m 0644 "$SELF_DIR/model/tokenizer.json" "$MODEL_DIR/tokenizer.json"
  run install -o root -g root -m 0644 "$SELF_DIR/source/memvid-source.tar.gz" "$PREFIX/share/memvid/memvid-source.tar.gz"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] extract source snapshot to $SOURCE_DIR"
  else
    tar -xzf "$SELF_DIR/source/memvid-source.tar.gz" -C "$SOURCE_DIR"
  fi
  write_settings
}

create_state_dirs() {
  msg "Creating state directories"
  local owner="$RUN_USER"
  if ! id "$owner" >/dev/null 2>&1; then
    warn "User '$owner' does not exist; using root ownership for state dirs and disabling service install."
    owner=root
    RUN_GROUP=root
    INSTALL_SERVICES=0
  fi
  for dir in queue processing ingest done failed store legacy_archives; do
    run install -d -o "$owner" -g "$RUN_GROUP" -m 0755 "$STATE_DIR/$dir"
  done
}

install_services() {
  [[ "$INSTALL_SERVICES" -eq 1 ]] || return 0
  if ! have systemctl || [[ ! -d /run/systemd/system && ! -d /etc/systemd/system ]]; then
    warn "systemd not detected; service install skipped."
    return 0
  fi

  msg "Installing systemd services"
  run install -d -o root -g root -m 0755 /etc/systemd/system

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] write /etc/systemd/system/memvid-ingestor.service"
    echo "[dry-run] write /etc/systemd/system/memvid-embedder.service"
  else
    cat > /etc/systemd/system/memvid-ingestor.service <<EOF
[Unit]
Description=Memvid queue ingestor
After=local-fs.target

[Service]
Type=simple
User=$RUN_USER
Group=$RUN_GROUP
Environment=MEMVID_CONFIG=$CONFIG_DIR/settings.toml
WorkingDirectory=$STATE_DIR
ExecStart=$PREFIX/bin/memvid-ingestor
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

    cat > /etc/systemd/system/memvid-embedder.service <<EOF
[Unit]
Description=Memvid CUDA queue embedder
After=local-fs.target memvid-ingestor.service
Wants=memvid-ingestor.service

[Service]
Type=simple
User=$RUN_USER
Group=$RUN_GROUP
Environment=MEMVID_CONFIG=$CONFIG_DIR/settings.toml
Environment=LD_LIBRARY_PATH=$PREFIX/lib/memvid:/usr/lib/x86_64-linux-gnu
WorkingDirectory=$STATE_DIR
ExecStart=$PREFIX/bin/memvid-embedder
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    chmod 0644 /etc/systemd/system/memvid-ingestor.service /etc/systemd/system/memvid-embedder.service
  fi

  run systemctl daemon-reload
  run systemctl enable memvid-ingestor.service memvid-embedder.service
  run systemctl restart memvid-ingestor.service memvid-embedder.service
}

append_alias_block() {
  local rc="$1"
  [[ "$INSTALL_ALIASES" -eq 1 ]] || return 0
  [[ -n "$rc" ]] || return 0
  if [[ -f "$rc" ]] && grep -q "Memvid startup context injection" "$rc"; then
    return 0
  fi
  msg "Adding aliases to $rc"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] append aliases to $rc"
    return 0
  fi
  mkdir -p "$(dirname "$rc")"
  cat >> "$rc" <<'EOF'

# Memvid startup context injection.
# Default agent launches go through wrappers that prepend read-only source-of-truth context.
alias codex='codex-memvid'
alias claude='claude-memvid'
alias gemini='gemini-memvid'
alias codex-raw='command codex'
alias claude-raw='command claude'
alias gemini-raw='command gemini'
alias memctx='memvid-context'
EOF
}

install_aliases() {
  [[ "$INSTALL_ALIASES" -eq 1 ]] || return 0
  append_alias_block /root/.bashrc
  if id "$RUN_USER" >/dev/null 2>&1; then
    local home
    home="$(getent passwd "$RUN_USER" | cut -d: -f6)"
    if [[ -n "$home" ]]; then
      append_alias_block "$home/.bashrc"
      [[ "$DRY_RUN" -eq 1 ]] || chown "$RUN_USER:$RUN_GROUP" "$home/.bashrc"
    fi
  fi
}

verify_install() {
  msg "Verifying install"
  local required=(
    "$PREFIX/bin/memvid-context"
    "$PREFIX/bin/memvid-embedder"
    "$PREFIX/bin/memvid-ingestor"
    "$PREFIX/bin/memvid-migrator"
    "$PREFIX/bin/codex-memvid"
    "$PREFIX/bin/claude-memvid"
    "$PREFIX/bin/gemini-memvid"
    "$CONFIG_DIR/settings.toml"
    "$MODEL_DIR/model.onnx"
    "$MODEL_DIR/tokenizer.json"
    "$PREFIX/share/memvid/memvid-source.tar.gz"
  )
  for path in "${required[@]}"; do
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "[dry-run] verify $path"
    elif [[ ! -e "$path" ]]; then
      echo "Missing expected install path: $path" >&2
      exit 1
    fi
  done

  if [[ "$DRY_RUN" -eq 0 ]]; then
    "$PREFIX/bin/memvid-context" --project global --budget-tokens 300 >/dev/null || warn "memvid-context returned non-zero; store may be empty or unreadable until services ingest records."
    if have systemctl && systemctl list-unit-files memvid-ingestor.service >/dev/null 2>&1; then
      systemctl --no-pager --full status memvid-ingestor.service memvid-embedder.service || true
    fi
  fi
}

main() {
  msg "Installing Memvid protocol stack"
  install_deps
  install_files
  create_state_dirs
  install_services
  install_aliases
  verify_install
  msg "Memvid install complete"
}

main "$@"
