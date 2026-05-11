#!/usr/bin/env bash
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
INSTALL_DEPS=1
INSTALL_SERVICES=1
INSTALL_ALIASES=1
INSTALL_OLLAMA=1
PULL_LIBRARIAN_MODEL=1
INSTALL_RUST=1
PREFIX=/usr/local
CONFIG_DIR=/etc/memvid
STATE_DIR=/var/lib/memvid
MODEL_DIR=/opt/models/nomic-embed-text-v1
SOURCE_DIR=/opt/memvid/source
CACHYOS_NVIDIA_SCOPE=installed
CACHYOS_NVIDIA_FLAVOR=open
LIBRARIAN_MODEL=qwen3:8b
OLLAMA_TIMEOUT_SECONDS=120
RUST_TOOLCHAIN=1.90.0
RUST_NIGHTLY_TOOLCHAIN=nightly
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
  --no-aliases           Do not update root/user shell functions and wrappers.
  --no-ollama            Do not install/enable/start Ollama.
  --no-librarian-model   Do not pull the configured librarian model with Ollama.
  --no-rust              Do not install Rust build toolchains.
  --user USER            Service/data owner user. Defaults to omen when present.
  --prefix PATH          Install prefix for binaries/libs/share. Default: /usr/local.
  --config-dir PATH      Config directory. Default: /etc/memvid.
  --state-dir PATH       State directory. Default: /var/lib/memvid.
  --model-dir PATH       Model directory. Default: /opt/models/nomic-embed-text-v1.
  --source-dir PATH      Source snapshot extract directory. Default: /opt/memvid/source.
  --librarian-model NAME Ollama model for librarian recall. Default: qwen3:8b.
  --ollama-timeout SEC   Seconds to wait for Ollama readiness. Default: 120.
  --rust-toolchain NAME  Rust release toolchain to install. Default: 1.90.0.
  --rust-nightly NAME    Rust nightly toolchain to install. Default: nightly.
  --cachyos-nvidia SCOPE CachyOS NVIDIA modules: installed, all, or skip. Default: installed.
  --nvidia-flavor FLAVOR NVIDIA kernel module flavor: open, closed, or auto. Default: open.
  -h, --help             Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --no-deps) INSTALL_DEPS=0 ;;
    --no-services) INSTALL_SERVICES=0 ;;
    --no-aliases) INSTALL_ALIASES=0 ;;
    --no-ollama) INSTALL_OLLAMA=0; PULL_LIBRARIAN_MODEL=0 ;;
    --no-librarian-model) PULL_LIBRARIAN_MODEL=0 ;;
    --no-rust) INSTALL_RUST=0 ;;
    --user) RUN_USER="${2:?--user requires a value}"; shift ;;
    --prefix) PREFIX="${2:?--prefix requires a value}"; shift ;;
    --config-dir) CONFIG_DIR="${2:?--config-dir requires a value}"; shift ;;
    --state-dir) STATE_DIR="${2:?--state-dir requires a value}"; shift ;;
    --model-dir) MODEL_DIR="${2:?--model-dir requires a value}"; shift ;;
    --source-dir) SOURCE_DIR="${2:?--source-dir requires a value}"; shift ;;
    --librarian-model) LIBRARIAN_MODEL="${2:?--librarian-model requires a value}"; shift ;;
    --ollama-timeout) OLLAMA_TIMEOUT_SECONDS="${2:?--ollama-timeout requires a value}"; shift ;;
    --rust-toolchain) RUST_TOOLCHAIN="${2:?--rust-toolchain requires a value}"; shift ;;
    --rust-nightly) RUST_NIGHTLY_TOOLCHAIN="${2:?--rust-nightly requires a value}"; shift ;;
    --cachyos-nvidia) CACHYOS_NVIDIA_SCOPE="${2:?--cachyos-nvidia requires a value}"; shift ;;
    --nvidia-flavor) CACHYOS_NVIDIA_FLAVOR="${2:?--nvidia-flavor requires a value}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 2 ;;
  esac
  shift
done

if [[ "$(id -u)" -ne 0 && "$DRY_RUN" -ne 1 ]]; then
  echo "This installer needs root privileges. Re-run with sudo or as root." >&2
  exit 1
elif [[ "$(id -u)" -ne 0 ]]; then
  echo "WARN: dry-run running without root; privileged actions will only be printed." >&2
fi

case "$CACHYOS_NVIDIA_SCOPE" in
  installed|all|skip) ;;
  *) echo "--cachyos-nvidia must be installed, all, or skip" >&2; exit 2 ;;
esac

case "$CACHYOS_NVIDIA_FLAVOR" in
  open|closed|auto) ;;
  *) echo "--nvidia-flavor must be open, closed, or auto" >&2; exit 2 ;;
esac

case "$OLLAMA_TIMEOUT_SECONDS" in
  ''|*[!0-9]*) echo "--ollama-timeout must be a non-negative integer" >&2; exit 2 ;;
esac

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

run_as_user() {
  local user="$1"
  shift
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run] as %q' "$user"
    printf ' %q' "$@"
    printf '\n'
  elif [[ "$user" == "root" ]]; then
    "$@"
  elif have runuser; then
    runuser -u "$user" -- "$@"
  elif have sudo; then
    sudo -u "$user" "$@"
  else
    warn "Cannot run command as $user; missing runuser and sudo."
    return 1
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

pkg_installed_pacman() {
  pacman -Qq "$1" >/dev/null 2>&1
}

pkg_available_pacman() {
  pacman -Si "$1" >/dev/null 2>&1
}

is_cachyos() {
  [[ "${MEMVID_FORCE_CACHYOS:-0}" == "1" ]] && return 0
  [[ -r /etc/os-release ]] || return 1
  local os_id="" os_id_like=""
  # shellcheck disable=SC1091
  . /etc/os-release
  os_id="${ID:-}"
  os_id_like="${ID_LIKE:-}"
  [[ "$os_id" == "cachyos" || " $os_id_like " == *" cachyos "* ]]
}

pacman_install_available() {
  local pkg needed=()
  for pkg in "$@"; do
    if pkg_installed_pacman "$pkg"; then
      continue
    fi
    if pkg_available_pacman "$pkg"; then
      needed+=("$pkg")
    else
      warn "Pacman package not available in enabled repositories: $pkg"
    fi
  done

  if [[ "${#needed[@]}" -gt 0 ]]; then
    msg "Installing pacman packages: ${needed[*]}"
    run pacman -Syu --needed --noconfirm "${needed[@]}"
  fi
}

cachyos_kernel_variants_all() {
  cat <<'EOF'
linux-cachyos
linux-cachyos-lto
linux-cachyos-bore
linux-cachyos-bore-lto
linux-cachyos-bmq
linux-cachyos-bmq-lto
linux-cachyos-deckify
linux-cachyos-deckify-lto
linux-cachyos-eevdf
linux-cachyos-eevdf-lto
linux-cachyos-lts
linux-cachyos-lts-lto
linux-cachyos-hardened
linux-cachyos-hardened-lto
linux-cachyos-rc
linux-cachyos-rc-lto
linux-cachyos-server
linux-cachyos-server-lto
linux-cachyos-rt-bore
linux-cachyos-rt-bore-lto
EOF
}

cachyos_installed_kernel_variants() {
  local pkg
  pacman -Qq | while read -r pkg; do
    case "$pkg" in
      linux-cachyos|linux-cachyos-*)
        case "$pkg" in
          *-headers|*-nvidia|*-nvidia-open|*-zfs|*-dbg|*-r8125)
            continue
            ;;
        esac
        printf '%s\n' "$pkg"
        ;;
    esac
  done
}

cachyos_module_package_for_kernel() {
  local kernel="$1"
  case "$CACHYOS_NVIDIA_FLAVOR" in
    open)
      pkg_available_pacman "$kernel-nvidia-open" && printf '%s\n' "$kernel-nvidia-open"
      ;;
    closed)
      pkg_available_pacman "$kernel-nvidia" && printf '%s\n' "$kernel-nvidia"
      ;;
    auto)
      if pkg_available_pacman "$kernel-nvidia-open"; then
        printf '%s\n' "$kernel-nvidia-open"
      elif pkg_available_pacman "$kernel-nvidia"; then
        printf '%s\n' "$kernel-nvidia"
      fi
      ;;
  esac
}

install_cachyos_nvidia_modules() {
  [[ "$CACHYOS_NVIDIA_SCOPE" != "skip" ]] || return 0

  local kernels=() kernel module modules=()
  if [[ "$CACHYOS_NVIDIA_SCOPE" == "all" ]]; then
    mapfile -t kernels < <(cachyos_kernel_variants_all)
  else
    mapfile -t kernels < <(cachyos_installed_kernel_variants)
  fi

  if [[ "${#kernels[@]}" -eq 0 ]]; then
    warn "No CachyOS kernel packages detected for NVIDIA module matching."
    return 0
  fi

  for kernel in "${kernels[@]}"; do
    if [[ "$CACHYOS_NVIDIA_SCOPE" == "all" ]] && ! pkg_available_pacman "$kernel"; then
      continue
    fi
    module="$(cachyos_module_package_for_kernel "$kernel" || true)"
    if [[ -n "$module" ]]; then
      modules+=("$module")
    else
      warn "No $CACHYOS_NVIDIA_FLAVOR NVIDIA module package found for CachyOS kernel: $kernel"
    fi
  done

  if [[ "${#modules[@]}" -gt 0 ]]; then
    msg "Installing CachyOS NVIDIA module packages for $CACHYOS_NVIDIA_SCOPE kernels: ${modules[*]}"
    pacman_install_available "${modules[@]}"
  else
    warn "No CachyOS NVIDIA module packages selected."
  fi
}

install_pacman_deps() {
  msg "Installing Arch/CachyOS CUDA runtime packages"
  local pkgs=(ca-certificates curl tar xz coreutils findutils gawk sed openssl cuda cudnn nvidia-utils libglvnd)
  if [[ "$INSTALL_RUST" -eq 1 ]]; then
    pkgs+=(rustup base-devel git clang cmake pkgconf lld mold)
  fi
  if [[ "$INSTALL_OLLAMA" -eq 1 ]]; then
    pkgs+=(ollama ollama-cuda)
  fi
  pacman_install_available "${pkgs[@]}"

  if is_cachyos; then
    install_cachyos_nvidia_modules
  else
    warn "Arch-like system detected but not CachyOS; install a matching NVIDIA module or DKMS package for the active kernel if provider loading fails."
  fi
}

bootstrap_rust() {
  [[ "$INSTALL_RUST" -eq 1 ]] || return 0

  msg "Bootstrapping Rust build toolchains"
  if ! have rustup; then
    warn "rustup command not found; Rust toolchain install skipped."
    return 0
  fi
  if ! id "$RUN_USER" >/dev/null 2>&1; then
    warn "User '$RUN_USER' does not exist; Rust toolchain install skipped."
    return 0
  fi

  local host_target="x86_64-unknown-linux-gnu"
  local source_targets=(
    aarch64-apple-darwin
    x86_64-apple-darwin
    x86_64-pc-windows-msvc
    x86_64-unknown-linux-gnu
  )
  local stable_args=(rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --component rustfmt --component clippy)
  local target
  for target in "${source_targets[@]}"; do
    stable_args+=(--target "$target")
  done
  if ! run_as_user "$RUN_USER" "${stable_args[@]}"; then
    warn "Rust toolchain install failed for $RUST_TOOLCHAIN; rebuild from source may need manual rustup repair."
  fi
  if ! run_as_user "$RUN_USER" rustup toolchain install "$RUST_NIGHTLY_TOOLCHAIN" --profile minimal --component rustfmt --component clippy --target "$host_target"; then
    warn "Rust nightly install failed for $RUST_NIGHTLY_TOOLCHAIN; nightly-only rebuild tasks may need manual rustup repair."
  fi
}

install_deps() {
  [[ "$INSTALL_DEPS" -eq 1 ]] || return 0
  msg "Checking runtime dependencies"

  local missing_tools=()
  for tool in tar xz sed awk find chmod chown install; do
    have "$tool" || missing_tools+=("$tool")
  done
  if [[ "${#missing_tools[@]}" -gt 0 ]]; then
    warn "Missing basic tools: ${missing_tools[*]}"
  fi

  if [[ "${MEMVID_FORCE_PACMAN:-0}" == "1" ]] && have pacman; then
    install_pacman_deps
  elif [[ -f /etc/debian_version ]] && have apt-get; then
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
    install_pacman_deps
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
librarian_queue = "$STATE_DIR/librarian_queue"
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

[librarian]
enabled = true
endpoint = "http://127.0.0.1:11434/v1/chat/completions"
model = "$LIBRARIAN_MODEL"
timeout_ms = 30000
max_candidates = 6
max_selected = 6
max_tokens = 512
temperature = 0.0
top_p = 1.0
presence_penalty = 1.5
keep_alive = "-1"
EOF
  chmod 0644 "$settings"
}

install_files() {
  msg "Installing Memvid files"
  run install -d -o root -g root -m 0755 "$PREFIX/bin" "$PREFIX/lib/memvid" "$PREFIX/share/memvid" "$PREFIX/share/memvid/docs" "$MODEL_DIR" "$SOURCE_DIR"
  run install -o root -g root -m 0755 "$SELF_DIR/bin/"* "$PREFIX/bin/"
  run install -o root -g root -m 0644 "$SELF_DIR/lib/"*.so "$PREFIX/lib/memvid/"
  run install -o root -g root -m 0644 "$SELF_DIR/docs/AGENTS.md" "$SELF_DIR/docs/CLAUDE.md" "$SELF_DIR/docs/GEMINI.md" "$PREFIX/share/memvid/"
  run install -o root -g root -m 0644 "$SELF_DIR/docs/memvid-context.md" "$SELF_DIR/docs/memvid-librarian.md" "$PREFIX/share/memvid/docs/"
  run install -o root -g root -m 0644 "$SELF_DIR/model/model.onnx" "$MODEL_DIR/model.onnx"
  run install -o root -g root -m 0644 "$SELF_DIR/model/tokenizer.json" "$MODEL_DIR/tokenizer.json"
  run install -o root -g root -m 0644 "$SELF_DIR/source/memvid-source.tar" "$PREFIX/share/memvid/memvid-source.tar"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] extract source snapshot to $SOURCE_DIR"
  else
    tar -xf "$SELF_DIR/source/memvid-source.tar" -C "$SOURCE_DIR"
  fi
  write_settings
}

ollama_base_url() {
  printf '%s\n' "http://127.0.0.1:11434"
}

wait_for_ollama() {
  local base="$1"
  local waited=0
  if ! have curl; then
    warn "curl not found; cannot poll Ollama readiness."
    return 1
  fi
  while (( waited <= OLLAMA_TIMEOUT_SECONDS )); do
    if curl -fsS "$base/api/tags" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done
  return 1
}

ollama_model_present() {
  local model="$1"
  ollama list 2>/dev/null | awk -v model="$model" 'NR > 1 && $1 == model { found=1 } END { exit found ? 0 : 1 }'
}

bootstrap_ollama() {
  [[ "$INSTALL_OLLAMA" -eq 1 || "$PULL_LIBRARIAN_MODEL" -eq 1 ]] || return 0

  msg "Bootstrapping Ollama librarian runtime"
  if ! have ollama; then
    warn "ollama command not found; install/pull skipped."
    return 0
  fi

  local base
  base="$(ollama_base_url)"
  if have systemctl && [[ -d /run/systemd/system || -d /etc/systemd/system ]]; then
    if systemctl list-unit-files ollama.service >/dev/null 2>&1; then
      if ! run systemctl enable --now ollama.service; then
        warn "Failed to enable/start ollama.service; try: systemctl status ollama.service"
      fi
    else
      warn "ollama.service not found; start Ollama manually with: ollama serve"
    fi
  else
    warn "systemd not detected; start Ollama manually with: ollama serve"
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] wait for Ollama at $base"
    [[ "$PULL_LIBRARIAN_MODEL" -eq 0 ]] || echo "[dry-run] ollama pull $LIBRARIAN_MODEL"
    return 0
  fi

  if ! wait_for_ollama "$base"; then
    warn "Ollama did not become ready at $base within ${OLLAMA_TIMEOUT_SECONDS}s; librarian model pull skipped."
    return 0
  fi

  if [[ "$PULL_LIBRARIAN_MODEL" -eq 1 ]]; then
    if ollama_model_present "$LIBRARIAN_MODEL"; then
      msg "Ollama model already present: $LIBRARIAN_MODEL"
    else
      msg "Pulling Ollama librarian model: $LIBRARIAN_MODEL"
      ollama pull "$LIBRARIAN_MODEL"
    fi
  fi
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
  for dir in queue librarian_queue processing ingest done failed store legacy_archives; do
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

append_shell_block() {
  local rc="$1"
  [[ "$INSTALL_ALIASES" -eq 1 ]] || return 0
  [[ -n "$rc" ]] || return 0
  msg "Adding shell integration to $rc"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] update shell integration in $rc"
    return 0
  fi
  mkdir -p "$(dirname "$rc")"
  local tmp
  tmp="$(mktemp)"
  if [[ -f "$rc" ]]; then
    sed \
      -e '/^# Memvid startup context injection\./,/^# End Memvid startup context injection\./d' \
      "$rc" > "$tmp"
  fi
  cat >> "$tmp" <<EOF

# Memvid startup context injection.
# Default agent launches go through hardened shell functions that call absolute wrappers and prepend read-only startup context.
case ":\$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *) export PATH="$PREFIX/bin:\$PATH" ;;
esac
export MEMVID_CONFIG="$CONFIG_DIR/settings.toml"
codex() { command "$PREFIX/bin/codex-memvid" "\$@"; }
claude() { command "$PREFIX/bin/claude-memvid" "\$@"; }
gemini() { command "$PREFIX/bin/gemini-memvid" "\$@"; }
codex-raw() { command codex "\$@"; }
claude-raw() { command claude "\$@"; }
gemini-raw() { command gemini "\$@"; }
memctx() { command "$PREFIX/bin/memvid-context" "\$@"; }
memq() { command "$PREFIX/bin/memvid-queue-write" "\$@"; }
memlib() { command "$PREFIX/bin/memvid-librarian-note" "\$@"; }
# End Memvid startup context injection.
EOF
  install -m 0644 "$tmp" "$rc"
  rm -f "$tmp"
}

install_aliases() {
  [[ "$INSTALL_ALIASES" -eq 1 ]] || return 0
  append_shell_block /root/.bashrc
  append_shell_block /root/.zshrc
  if id "$RUN_USER" >/dev/null 2>&1; then
    local home
    home="$(getent passwd "$RUN_USER" | cut -d: -f6)"
    if [[ -n "$home" ]]; then
      append_shell_block "$home/.bashrc"
      append_shell_block "$home/.zshrc"
      [[ "$DRY_RUN" -eq 1 ]] || chown "$RUN_USER:$RUN_GROUP" "$home/.bashrc" "$home/.zshrc"
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
    "$PREFIX/bin/memvid-queue-write"
    "$PREFIX/bin/memvid-librarian-note"
    "$CONFIG_DIR/settings.toml"
    "$MODEL_DIR/model.onnx"
    "$MODEL_DIR/tokenizer.json"
    "$PREFIX/share/memvid/memvid-source.tar"
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
  bootstrap_rust
  bootstrap_ollama
  install_services
  install_aliases
  verify_install
  msg "Memvid install complete"
}

main "$@"
