# Repository Guidelines

```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
```

## Memory model

Memvid stores one durable shard per project. Three live shards rotate on disk (`current`, `.1`, `.2`); the shard evicted from `.2` is `xz -9e` compressed and kept forever in the per-project archive. Single agent owns a project at a time — no concurrent writers.

Native agent memory (Claude memory tooling, learned profiles, cross-session caches) is disabled for project facts, architecture, decisions, handoffs, and task state in this repository. The project shard is the only durable recall surface.

The startup context injected by `memvid-context` is read-only. Do not parse rotated shards directly — read via `memvid-context --full` if you need them.

## Write contract

Write the **full** project shard from scratch on each meaningful milestone. Replace, do not append. The writer rotates the prior shard into `.1` automatically; you do not manage rotation.

Trigger a write when:
- A task you were given is complete.
- A decision is finalized (you committed code, or the user accepted an approach).
- A file, function, command, or protocol is created or renamed.
- A hard blocker is hit (missing dependency, broken tool, auth failure).
- Session is ending or context compaction is imminent.

Do **not** trigger a write for: speculation, intermediate steps within a task, hypotheses without evidence, plans you have not acted on.

## What the shard should contain

A fresh agent should be able to read the current shard and pick up cold. Include:

- Current architecture sketch (in one paragraph or short section).
- Active decisions, with the reasoning that makes them load-bearing.
- Recent handoffs, curated — drop intermediate steps, keep specifics that the next session needs.
- Open risks and blockers.
- Important paths, commands, and invariants that future-you cannot reconstruct from `git log` alone.

Pruning is your responsibility at write time. Drop stale handoffs first; never drop open risks or current architecture. Preserve **specifics** (file paths, error strings, exact commands, timestamps) — they are the part future-you cannot reconstruct from summary.

## How to write

```bash
memvid-write --project memvid --agent <your-agent-id> <<'EOF'
<full new shard body, in Markdown, rewritten from scratch>
EOF
```

The writer:
- Refuses an empty body.
- Refuses a new body smaller than 25% of the prior body unless you pass `--force` (truncation guard; only enforced when prior body > 256 bytes).
- Atomically rotates the prior shard into `.1`, evicts `.2` to the archive, seals the new shard at `chmod 0444`.

Use `--project global` only for explicit cross-project coordination. Ordinary workspace facts belong to the current project shard.

## How to read

```bash
memvid-context                       # body of current shard (project = $PWD basename)
memvid-context --project memvid      # explicit project
memvid-context --full                # bodies of current, .1, .2
memvid-context --verify              # recompute hashes, exit nonzero on mismatch
memvid-context --history             # list archived shards for the project
memvid-context --raw                 # full sealed file verbatim (header + body)
```

Wrappers (`claude-memvid`, `codex-memvid`, `gemini-memvid`) auto-inject the current shard at agent launch. Prefer those over invoking the underlying CLI directly.

## Format

`.mv2` is a plain-text sealed shard. See `MV2_SPEC.md` for the full v3 spec. Files are `chmod 0444`. Each shard's header records the SHA-256 of the prior shard's full file (`prev-sha256`), forming a tamper-evident chain across the rotation.

## Repo layout

- `deploy/bin/` — `memvid-write`, `memvid-context`, three agent wrappers.
- `packaging/install.sh` — single-shot installer.
- `config/settings.toml` — `shards` and `archive` paths.
- `tests/` — shell tests for sealing, rotation, archive, verification.
- `MV2_SPEC.md` — file format spec.
- `docs/` — per-tool references.

## Coding style (shell)

- `#!/usr/bin/env bash` + `set -euo pipefail`.
- Quote variables (`"$var"`). Use `[[ ]]` for tests.
- Prefer POSIX utilities (`awk`, `sed`, `grep`, `sha256sum`, `xz`); no Python or Rust dependencies for the runtime tools.
- Functions in `snake_case`. Constants in `SCREAMING_SNAKE_CASE`. Local vars in `snake_case`.

## Build and test

```bash
make test         # run shell test suite
make lint         # shellcheck
make install      # local install via packaging/install.sh
```

## Commit and PR style

Concise imperative subjects (`fix: ...`, `docs: ...`, scoped where useful). Mention user-visible behavior changes. Pull requests note tests run and any installer or layout changes.

## Security

Do not commit local shards, archives, credentials, or generated `.mv2` files. Service paths default under `/var/lib/memvid`.
