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
- installer logic for settings, state dirs, systemd units, shell functions, Ollama startup, and librarian model pull

Non-shell launchers should call installed wrapper binaries directly, for example `/usr/local/bin/codex-memvid`, `/usr/local/bin/claude-memvid`, `/usr/local/bin/gemini-memvid`, or `/usr/local/bin/memvid-context-wrap -- your-agent-command`. Shell functions only affect interactive shells that source the installed rc block; absolute raw agent paths bypass Memvid startup recall.

The `.run` file is intentionally uncompressed. It is the fast local installer.

The `.run.tar.xz` file is the compressed transfer artifact. It is what you copy over the network, upload, archive, or share. Extract it first, then run the resulting `.run`.

Large installer artifacts belong in GitHub Releases, not git history. The repository ignores `/dist`; upload `memvid-bootstrap-x86_64-linux.run.tar.xz` as a release asset and keep generated `.run`, `.tar.xz`, model payloads, and other package blobs out of `main`.

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

On CachyOS, the default install is intended to be close to one-run: it installs Memvid binaries, CUDA/cuDNN runtime packages, CachyOS NVIDIA modules for installed kernels, Ollama with CUDA support, starts `ollama.service`, pulls `qwen3:8b`, writes Memvid settings, enables Memvid services, and installs shell launch wrappers.

Dry run:

```bash
./dist/memvid-bootstrap-x86_64-linux.run --dry-run
```

Useful options:

Default model dir is `/opt/models/nomic-embed-text-v1`; `--model-dir` overrides it for alternate local runtimes.

```bash
--no-deps
--no-services
--no-aliases
--no-ollama
--no-librarian-model
--user USER
--prefix /usr/local
--config-dir /etc/memvid
--state-dir /var/lib/memvid
--model-dir /opt/models/nomic-embed-text-v1
--source-dir /opt/memvid/source
--librarian-model qwen3:8b
--ollama-timeout 120
--cachyos-nvidia installed
--nvidia-flavor open
```

## CachyOS NVIDIA Support

On CachyOS and other pacman systems, the installer now attempts to install:

- `curl`, `tar`, `xz`, `coreutils`, `findutils`, `gawk`, `sed`, and `openssl`
- `cuda`
- `cudnn`
- `nvidia-utils`
- `libglvnd`
- `ollama`
- `ollama-cuda`
- matching CachyOS prebuilt NVIDIA module packages

If `ollama.service` exists, the installer enables and starts it, waits up to `--ollama-timeout` seconds for `http://127.0.0.1:11434`, then runs `ollama pull qwen3:8b`. Use `--no-ollama` to skip Ollama package/service/model setup, or `--no-librarian-model` to install/start Ollama without pulling the model.

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
- Debian/Ubuntu and Arch/CachyOS CUDA/runtime/Ollama packages are installed best-effort.
- CachyOS NVIDIA support depends on enabled CachyOS repositories matching the machine architecture tier.
- Other distributions receive explicit warnings because CUDA/cuDNN package names depend on enabled repositories.
- Alpine/musl is not supported by these prebuilt binaries.
- systemd services are installed only when systemd is present.
- `qwen3:8b` pull requires network access to Ollama's model registry unless the model already exists on the machine.

On non-systemd hosts, the binaries/config/model still install; run `ollama serve`, `memvid-ingestor`, and `memvid-embedder` under the host's service manager with `MEMVID_CONFIG=/etc/memvid/settings.toml`.
