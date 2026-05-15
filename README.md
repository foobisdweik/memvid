# Memvid

Memvid is a tiny, file-based memory layer for coding agents. Each project gets a single sealed text shard on disk that the agent rewrites in full at meaningful milestones. There is no daemon, no database, no embeddings, and no query engine — just three bash tools and a documented file format.

## Why

A coding agent needs a durable record of project context across sessions: what was decided, what is in flight, what broke, what to avoid. Most "memory" systems give the agent a writable index it can silently corrupt over time. Memvid gives the agent one plain-text shard per project, makes it read-only after each write, keeps two prior versions on disk, and archives the rest indefinitely under `xz -9e`. The agent owns the shard's content. The format guarantees the agent cannot accidentally lose history.

## Install

The repo ships an install script that drops the three tools and their wrappers into a system prefix and creates the shard and archive directories.

```bash
sudo packaging/install.sh
```

Default install prefix is `/usr/local/bin`. Default state directories are `/var/lib/memvid/shards/` and `/var/lib/memvid/archive/`. Override via the environment variables documented below or in `/etc/memvid/settings.toml`.

## Architecture

Per project, Memvid keeps three live shards in the shards directory:

```
<shards>/<project>.mv2     current (read-only, 0444)
<shards>/<project>.mv2.1   one write ago
<shards>/<project>.mv2.2   two writes ago
```

On every successful write, the shard previously in `.2` is compressed with `xz -9e` and moved into the per-project archive directory; `.1` becomes `.2`, `current` becomes `.1`, and the freshly sealed shard becomes `current`. Renames are atomic; the new shard file is `chmod 0444` after seal.

Each shard's header carries `prev-sha256`, the SHA-256 of the entire prior `current` file. That gives a tamper-evident chain across the three live shards: any modification to `.1` or `.2` invalidates the next shard's prev hash. `--verify` checks both internal body hashes and the chain links.

The format itself is plain UTF-8: a small header of `key: value` lines, then `---BEGIN BODY---\n`, then exactly `body-bytes` bytes of body, then `\n---END BODY---\n`. Bodies are typically Markdown but the format does not constrain them. See [`MV2_SPEC.md`](MV2_SPEC.md) for the full specification.

## Tools

### memvid-write

```bash
memvid-write --project NAME [--agent NAME] [--force] < body.md
```

Reads the new shard body from stdin, seals it as MV2 v3, rotates `current` to `.1` and `.1` to `.2`, archives the evicted `.2`, and chmods the new `current` to `0444`. Refuses to write an empty body. Refuses to write a new body smaller than 25% of the prior body unless `--force` is passed (only enforced when the prior body is larger than 256 bytes). See [`docs/memvid-write.md`](docs/memvid-write.md).

### memvid-context

```bash
memvid-context [--project NAME] [--full|--verify|--history|--raw]
```

Reads from the current shard. Default mode prints the body. `--full` prints bodies of all three live shards. `--verify` recomputes hashes and chain links across `current`, `.1`, and `.2`. `--history` lists the archived `.mv2.xz` files for the project. `--raw` prints the full sealed file verbatim. See [`docs/memvid-context.md`](docs/memvid-context.md).

### claude-memvid / codex-memvid / gemini-memvid

Thin wrappers that locate the real `claude`, `codex`, or `gemini` CLI on `PATH`, fetch the current project shard's body via `memvid-context`, and inject it as startup context before the user's prompt. Arguments after `--` are treated as the prompt; arguments before `--` are passed through to the agent CLI.

```bash
claude-memvid -- "continue the migration"
codex-memvid --dangerously-bypass-approvals-and-sandbox -- "continue the migration"
gemini-memvid --model gemini-2.5-pro -- "review the current state"
```

## Agent contract

The agent is the sole writer for its project's shard. It is expected to:

- Treat the injected context as read-only recall.
- Rewrite the shard in full at meaningful milestones: confirmed fixes, finalized decisions, completed tasks, renames, hard blockers, session handoff.
- Prune at write time. The body is whatever the agent decides is worth remembering; old content that no longer matters should be dropped from the next write rather than accumulated.
- Use `--project global` only for explicit cross-project coordination. Ordinary workspace facts belong in the current project's shard.

There is no incremental queue. Each shard is a complete snapshot. The three-slot rotation and the archive provide the recovery path if a write turns out to have been a mistake.

## File format

A shard is plain UTF-8 text. Seven header lines (`MV2 SHARD v3` magic, project, agent, ISO-8601 UTC timestamp, `prev-sha256`, `body-sha256`, `body-bytes`), then `---BEGIN BODY---\n`, then exactly `body-bytes` bytes of body, then `\n---END BODY---\n`. Total file size is fully determined by the header values; verification is a constant-time structural check plus two SHA-256 computations.

The full spec, including hash chain semantics, rotation invariants, and archive naming, is in [`MV2_SPEC.md`](MV2_SPEC.md).

## Layout on disk

```
/var/lib/memvid/shards/<project>.mv2          current sealed shard (0444)
/var/lib/memvid/shards/<project>.mv2.1        one write ago (0444)
/var/lib/memvid/shards/<project>.mv2.2        two writes ago (0444)
/var/lib/memvid/archive/<project>/            xz -9e compressed evicted shards
/etc/memvid/settings.toml                     optional path overrides
```

Paths can be overridden per invocation with `MEMVID_SHARDS_DIR`, `MEMVID_ARCHIVE_DIR`, and `MEMVID_CONFIG`, or globally in the settings file:

```toml
[paths]
shards = "/var/lib/memvid/shards"
archive = "/var/lib/memvid/archive"
```

## License

Apache License 2.0. See [`LICENSE`](LICENSE).
