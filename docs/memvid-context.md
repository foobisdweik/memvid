# Memvid Startup Context

`memvid-context` emits a bounded Markdown startup packet from read-only Memvid source-of-truth stores. It is intended for launch wrappers, not for agents to mutate memory.

## Commands

```bash
memvid-context --project memvid --agent codex --budget-tokens 4000
memvid-context --project memvid --query "open risks service install"
codex-memvid --dangerously-bypass-approvals-and-sandbox -- "continue the migration"
claude-memvid -- "continue the migration"
gemini-memvid --model gemini-2.5-pro -- "review the current state"
memvid-context-wrap -- your-agent-command
```

## Compression

Startup recall uses a 7-day chrono-semantic horizon:

- 0-6 hours: near-full relevant snippets.
- 6-24 hours: trimmed prose, decisions and commands preserved.
- 1-2 days: task/thread summaries.
- 2-4 days: compact project-state bullets.
- 4-7 days: canonical facts, risks, handoffs, and invariants.
- 7+ days: one-line facts or omission unless semantically critical.

Raw `.mv2` stores are never rewritten by compression. The packet is only a view over backend-owned daily stores.

## Rotation

Stores rotate by day under `/var/lib/memvid/store/YYYY-MM-DD.mv2`. The injector scans recent stores newest-first, opens them read-only, and falls back to a temporary snapshot copy if the live writer lock blocks direct reads.

Agents should treat injected context as read-only and write durable updates only through `/var/lib/memvid/queue`.
