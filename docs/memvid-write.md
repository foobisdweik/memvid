# memvid-write

Seals a new project shard, rotates prior shards, and archives the evicted `.2`. Reads the full new shard body from stdin; the body is written verbatim into the sealed MV2 v3 file. The new `current` file is left mode `0444`.

## Usage

```bash
memvid-write --project NAME [--agent NAME] [--force] < body.md
```

## Flags

| Flag | Required | Description |
|------|----------|-------------|
| `--project NAME` | yes | Project name. Must match `[A-Za-z0-9._-]+`. Use `global` for cross-project shards. |
| `--agent NAME` | no | Agent identifier written into the `agent:` header line. Defaults to `$MEMVID_AGENT`, then to `agent`. |
| `--force` | no | Skip the shrinkage safety check. |
| `-h`, `--help` | no | Print usage and exit. |

On success, prints the path of the new `current` shard.

## Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMVID_AGENT` | `agent` | Default agent name when `--agent` is not given. |
| `MEMVID_SHARDS_DIR` | `/var/lib/memvid/shards` | Override the shards directory. |
| `MEMVID_ARCHIVE_DIR` | `/var/lib/memvid/archive` | Override the archive directory. The per-project subdirectory `<archive>/<project>/` is created on demand. |
| `MEMVID_CONFIG` | `/etc/memvid/settings.toml` | Settings file consulted for `[paths].shards` and `[paths].archive` when the corresponding env vars are unset. |

## Shrinkage safety check

Accidental truncation is the most common failure mode for an agent-owned shard: a model writes a one-line body when it meant to rewrite a hundred lines. To catch that, the writer compares the new body size against the prior `current`'s `body-bytes` header:

- If the prior body is `<= 256` bytes, the check is skipped — small shards are allowed to stay small.
- Otherwise, if the new body is smaller than 25% of the prior body, the write is refused with a nonzero exit and an explanatory message.
- `--force` bypasses the check. Use it when an intentional aggressive prune is really what you want.

The check only inspects sizes; it does not look at content.

## Rotation behavior

On a successful seal, in order:

1. The shard is built as a temp file in the shards directory, `fsync`'d (best effort), and `chmod 0444`'d.
2. If `.2` exists, it is `xz -9e`'d into `<archive>/<project>/<sanitized-ts>-<short-hash>.mv2.xz` and removed.
3. `.1` is renamed to `.2` if present.
4. `current` is renamed to `.1` if present.
5. The temp file is renamed to `current`.

All renames are atomic on local filesystems.

## Example

```bash
$ cat <<'EOF' | memvid-write --project memvid --agent claude
# Memvid project state

- decision (2026-05-15): switched to v3 sealed shard format
- next: wire codex/gemini wrappers into the install script
EOF
/var/lib/memvid/shards/memvid.mv2
```
