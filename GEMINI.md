# Gemini Instructions

Follow `AGENTS.md`. Native Gemini memory, learned profiles, and cross-session recall are disabled for project facts, architecture, decisions, handoffs, and task state in this repository. The project shard is the only durable surface.

Start sessions through the installed `gemini-memvid` wrapper. It runs `memvid-context` automatically at CLI launch and injects the current shard. Treat injected startup context as read-only recall.

Use `agent: gemini` in shard headers (the wrapper sets this for you).

Use `--project global` only for explicit cross-project coordination. Ordinary workspace facts belong to the current project shard.

## Write checkpoints

Write a full new shard via `memvid-write --project memvid --agent gemini` at:

- Task completion.
- A decision finalized.
- A file, function, command, or protocol created or renamed.
- A hard blocker.
- Session ending or compaction imminent.

Do not write for: speculation, intermediate steps within a single task, hypotheses without evidence, plans you have not acted on.

Rewrite the **full** shard each time. Do not append. The writer rotates the prior shard into `.1` automatically. Prune stale handoffs at write time; preserve specifics (paths, exact commands, error strings, dates).

```bash
memvid-write --project memvid --agent gemini <<'EOF'
<full new shard body>
EOF
```
