# Memvid Project State

## Purpose

Memvid provides durable per-project memory for coding agents through sealed Markdown shards rotated on disk. There is no daemon, no database, no embedding pipeline, no librarian, and no queue. Agents own their project's shard and rewrite it in full at meaningful milestones.

## Layout

- `deploy/bin/memvid-write` — seal a new shard, rotate (`current → .1 → .2`), `xz -9e` archive the evicted `.2`.
- `deploy/bin/memvid-context` — read the current shard, the full rotation, the archive history, or verify the hash chain.
- `deploy/bin/claude-memvid`, `codex-memvid`, `gemini-memvid` — wrappers that inject the current shard at agent CLI launch.
- `config/settings.toml` — `shards` and `archive` paths only.
- `packaging/install.sh` — single-shot installer.
- `MV2_SPEC.md` — sealed shard format (v3).
- `tests/` — shell tests for sealing, rotation, archive, verification.

## Runtime protocol

Each project has one current shard plus two rotated backups on disk. The writer:

1. Reads the full new body from stdin.
2. Refuses empty bodies or shrinkage to less than 25% of prior body (`--force` overrides; only enforced when prior body > 256 bytes).
3. Builds a sealed v3 file with magic line, header, body framing, and SHA-256 hashes (`body-sha256`, `prev-sha256` chaining to the prior `current` file).
4. Atomically renames into place after `chmod 0444`.
5. Rotates the prior shards down (`current → .1 → .2`), `xz -9e` archives the evicted `.2`, and removes it from the live rotation.

Default state paths:

- Shards: `/var/lib/memvid/shards/<project>.mv2` (plus `.mv2.1`, `.mv2.2`)
- Archive: `/var/lib/memvid/archive/<project>/<sanitized-ts>-<short>.mv2.xz`

## Commands

- `make test` — run shell test suite (sealing, rotation, archive, verify).
- `make lint` — shellcheck on `deploy/bin/*` and `packaging/install.sh`.
- `make install` — invoke `packaging/install.sh`.

## Agent contract

Native agent memory is disabled for this project. Agents read the shard via `memvid-context` at session start and write a full new shard via `memvid-write` at milestones. See `AGENTS.md` for the write checkpoints and pruning rules.

## Format

`.mv2` is a sealed plain-text container. See `MV2_SPEC.md`. Files are `chmod 0444` and tamper-evident through a SHA-256 hash chain across the rotation.
