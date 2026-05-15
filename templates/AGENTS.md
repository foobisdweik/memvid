<!--
SETUP: After copying this file into your project, replace `<project>` with
your project name everywhere it appears (one find-and-replace). The chosen
name is the durable identifier under `/var/lib/memvid/shards/` and must
match `[A-Za-z0-9._-]+`.
-->

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
memvid-write --project <project> --agent <your-agent-id> <<'EOF'
<full new shard body, in Markdown, rewritten from scratch>
EOF
```

The writer:
- Refuses an empty body.
- Refuses a new body smaller than 25% of the prior body unless you pass `--force` (truncation guard; only enforced when prior body > 256 bytes).
- Atomically rotates the prior shard into `.1`, evicts `.2` to the archive, seals the new shard at `chmod 0444`.

Use `--project global` only for explicit cross-project coordination. Ordinary workspace facts belong to the `<project>` shard.

## How to read

```bash
memvid-context --project <project>          # body of current shard
memvid-context --project <project> --full   # bodies of current, .1, .2
memvid-context --project <project> --verify # recompute hashes, exit nonzero on mismatch
memvid-context --project <project> --history # list archived shards
memvid-context --project <project> --raw    # full sealed file verbatim
```

Wrappers (`claude-memvid`, `codex-memvid`, `gemini-memvid`) auto-inject the current shard at agent launch. They default the project to the basename of the working directory; set `MEMVID_PROJECT=<project>` in the environment if your repo lives at a path whose basename does not match `<project>`.

## Format

`.mv2` is a plain-text sealed shard. Files are `chmod 0444`. Each shard's header records the SHA-256 of the prior shard's full file (`prev-sha256`), forming a tamper-evident chain across the rotation. See the memvid repo's `MV2_SPEC.md` for the full v3 spec.
