# memvid-context

Reads from a project's sealed shards. Routine reads only touch the three live shards in the shards directory; the archive is consulted only via `--history`. The tool never writes.

## Usage

```bash
memvid-context [--project NAME] [--full|--verify|--history|--raw]
```

If `--project` is omitted, the project is taken from `MEMVID_PROJECT`, falling back to `basename "$PWD"`. Project names must match `[A-Za-z0-9._-]+`.

## Modes

| Mode | Behavior |
|------|----------|
| default | Prints the body of `current`. If no shard exists for the project, prints a short placeholder packet inviting the next agent to seal the first shard. |
| `--full` | Prints bodies of `current`, `.1`, and `.2` (whichever exist), each preceded by a `===== <slot> (<path>) =====` separator. |
| `--verify` | Recomputes `body-sha256` for each extant shard, checks the structural tail invariants (`body-bytes`, trailing `---END BODY---\n`), and verifies the hash chain: `current.prev-sha256 == sha256(.1 file)` and `.1.prev-sha256 == sha256(.2 file)`. `.2`'s chain link is not enforced. Exits nonzero on any failure; prints one `<slot>: OK` or `<slot>: FAIL <reason>` line per shard. |
| `--history` | Lists the archived `*.mv2.xz` files for the project as `<filename> <bytes>`, sorted. |
| `--raw` | Prints the full sealed `current` file verbatim, header and body framing included. |

## Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMVID_PROJECT` | `basename "$PWD"` | Default project name when `--project` is not given. |
| `MEMVID_SHARDS_DIR` | `/var/lib/memvid/shards` | Override the shards directory. Takes precedence over the settings file. |
| `MEMVID_ARCHIVE_DIR` | `/var/lib/memvid/archive` | Override the archive directory. Used only by `--history`. |
| `MEMVID_CONFIG` | `/etc/memvid/settings.toml` | Settings file consulted for `[paths].shards` and `[paths].archive` when the corresponding env vars are unset. |

## Example session

```bash
$ memvid-context --project memvid --verify
current: OK
.1: OK
.2: OK

$ memvid-context --project memvid | head -3
# Memvid project state
- branch: main
- last meaningful change: switched to v3 sealed shard format

$ memvid-context --project memvid --history
2026-05-14T18_03_12Z-1f2a3b4c.mv2.xz 612
2026-05-15T09_22_44Z-9d8c7b6a.mv2.xz 718
```
