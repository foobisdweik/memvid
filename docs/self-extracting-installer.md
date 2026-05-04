# Self-Extracting Memvid Installer

Build a single Linux `.run` installer from the current workspace:

```bash
packaging/build-self-extracting-installer.sh
```

Default output:

```text
dist/memvid-bootstrap-x86_64-linux.run
dist/memvid-bootstrap-x86_64-linux.run.tar.xz
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

The `.run` file is intentionally uncompressed. It is the fast local installer.

The `.run.tar.xz` file is the compressed transfer artifact. It is what you copy over the network, upload, archive, or share. Extract it first, then run the resulting `.run`.

This layout is intentionally simple:

```text
memvid-bootstrap-x86_64-linux.run          # fast local install
memvid-bootstrap-x86_64-linux.run.tar.xz   # compressed network/storage form
```

The transfer artifact uses multithreaded `xz -9e -T0` compression by default. `xz` is mainstream on Linux and gives a stronger ratio than gzip without depending on niche decompression tooling.

`xz -T0` may choose fewer threads than the machine's logical CPU count when the input stream has only a few large blocks. The builder defaults to `XZ_BLOCK_SIZE=64MiB` so the compressor has enough blocks to feed more threads. Override compression knobs when building:

```bash
MEMVID_XZ_THREADS=12 MEMVID_XZ_BLOCK_SIZE=32MiB packaging/build-self-extracting-installer.sh
```

The builder also honors `XZ_THREADS`, `XZ_BLOCK_SIZE`, and `XZ_MEMLIMIT`. Recommended shell defaults:

```bash
export XZ_THREADS=0
export MEMVID_XZ_THREADS=0
export XZ_BLOCK_SIZE=64MiB
export XZ_MEMLIMIT=75%
```

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
