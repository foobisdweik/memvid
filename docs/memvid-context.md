# Memvid Startup Context

`memvid-context` emits a bounded Markdown startup packet from read-only Memvid source-of-truth stores. It is intended for launch wrappers, not for agents to mutate memory.

## Commands

```bash
memvid-context --project memvid --agent codex --budget-tokens 4000
memvid-context --project memvid --query "open risks service install"
memvid-queue-write --agent codex --project memvid --status done --type update <<'EOF'
Completed shard-routing fix and verified tests.
EOF
codex-memvid --dangerously-bypass-approvals-and-sandbox -- "continue the migration"
claude-memvid -- "continue the migration"
gemini-memvid --model gemini-2.5-pro -- "review the current state"
memvid-context-wrap -- your-agent-command
```

## Compression

Startup recall uses a 7-day chrono-semantic horizon:

- 0-4 hours: near-full relevant snippets.
- 4-16 hours: rich recent context, still heavily favored in ranking.
- 16-48 hours: sharply reduced selection and much tighter compression.
- 2-4 days: compact task and project-state summaries only when still relevant.
- 4-7 days: canonical facts, risks, handoffs, and invariants.
- 7+ days: one-line facts or omission unless semantically critical.

Raw `.mv2` stores are never rewritten by compression. The packet is only a view over backend-owned project shards.

## Store Layout

Queue ingestion writes stable shards under `/var/lib/memvid/store`:

- `<project>.mv2` holds ordinary workspace memory for one project.
- `global.mv2` is reserved for explicit cross-project coordination.
- Queue records must include `[project:<name>]`; `[project:global]` is never inferred.

The injector opens stores read-only and falls back to a temporary snapshot copy if the live writer lock blocks direct reads. Startup context should be selected from the active project shard plus explicit global records. Other project shards are hidden unless `--include-other-projects` is set for debugging or migration.

Agents should treat injected context as read-only and write durable updates only through `/var/lib/memvid/queue`. Use `[project:global]` only for explicit cross-project coordination; normal workspace notes belong in the current project shard.
