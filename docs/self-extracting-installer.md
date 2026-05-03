# Self-Extracting Memvid Installer

Build a single Linux `.run` installer from the current workspace:

```bash
packaging/build-self-extracting-installer.sh
```

Default output:

```text
dist/memvid-bootstrap-x86_64-linux.run
```

The bundle contains:

- `memvid-context`
- `memvid-embedder`
- `memvid-ingestor`
- `memvid-migrator`
- agent wrappers: `codex-memvid`, `claude-memvid`, `gemini-memvid`, `memvid-context-wrap`
- AGENTS/CLAUDE/GEMINI protocol docs
- Nomic ONNX model and tokenizer
- ONNX Runtime provider libraries
- rebuildable source snapshot installed to `/opt/memvid/source`
- installer logic for settings, state dirs, systemd units, and Bash aliases

The self-extracting payload uses `xz -9e` compression. `xz` is mainstream on Linux and gives a stronger ratio than gzip without depending on niche decompression tooling.

Install:

```bash
sudo ./dist/memvid-bootstrap-x86_64-linux.run
```

Dry run:

```bash
sudo ./dist/memvid-bootstrap-x86_64-linux.run --dry-run
```

Useful options:

```bash
--no-deps
--no-services
--no-aliases
--user USER
--prefix /usr/local
--config-dir /etc/memvid
--state-dir /var/lib/memvid
--model-dir /opt/models/nomic-embed-text-v1
--source-dir /opt/memvid/source
--cachyos-nvidia installed
--nvidia-flavor open
```

## CachyOS NVIDIA Support

On CachyOS and other pacman systems, the installer now attempts to install:

- `cuda`
- `cudnn`
- `nvidia-utils`
- `libglvnd`
- matching CachyOS prebuilt NVIDIA module packages

Default CachyOS behavior is conservative:

```bash
sudo ./dist/memvid-bootstrap-x86_64-linux.run \
  --cachyos-nvidia installed \
  --nvidia-flavor open
```

That installs `linux-cachyos*-nvidia-open` packages only for CachyOS kernel variants already installed on the host.

To target every currently available CachyOS stable/RC kernel variant exposed by the enabled CachyOS repositories:

```bash
sudo ./dist/memvid-bootstrap-x86_64-linux.run \
  --cachyos-nvidia all \
  --nvidia-flavor open
```

The installer filters candidates through `pacman -Si`, so unavailable variants are skipped instead of failing the install. Candidate kernel families include default, LTO, BORE, BMQ, Deckify, EEVDF, LTS, Hardened, RC, Server, and RT-BORE variants when the repository publishes them with NVIDIA module packages.

Use `--nvidia-flavor auto` to prefer open modules and fall back to closed modules when an open package is not available. Use `--cachyos-nvidia skip` if the system already has a working NVIDIA stack or uses DKMS/custom kernel modules.

## Portability Limits

The installer is intentionally robust, but not magic:

- It targets x86_64 glibc Linux.
- CUDA service startup still requires a working NVIDIA driver.
- Debian/Ubuntu and Arch/CachyOS CUDA runtime packages are installed best-effort.
- CachyOS NVIDIA support depends on enabled CachyOS repositories matching the machine architecture tier.
- Other distributions receive explicit warnings because CUDA/cuDNN package names depend on enabled repositories.
- Alpine/musl is not supported by these prebuilt binaries.
- systemd services are installed only when systemd is present.

On non-systemd hosts, the binaries/config/model still install; run `memvid-ingestor` and `memvid-embedder` under the host's service manager with `MEMVID_CONFIG=/etc/memvid/settings.toml`.
