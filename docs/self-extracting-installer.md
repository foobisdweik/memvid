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
```

## Portability Limits

The installer is intentionally robust, but not magic:

- It targets x86_64 glibc Linux.
- CUDA service startup still requires a working NVIDIA driver.
- Debian/Ubuntu CUDA runtime packages are installed best-effort.
- Other distributions receive explicit warnings because CUDA/cuDNN package names depend on enabled repositories.
- Alpine/musl is not supported by these prebuilt binaries.
- systemd services are installed only when systemd is present.

On non-systemd hosts, the binaries/config/model still install; run `memvid-ingestor` and `memvid-embedder` under the host's service manager with `MEMVID_CONFIG=/etc/memvid/settings.toml`.
