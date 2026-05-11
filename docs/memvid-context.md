# Memvid Startup Context

`memvid-context` emits a bounded Markdown startup packet from read-only Memvid source-of-truth stores. It is intended for launch wrappers, not for agents to mutate memory.

## Commands

```bash
memvid-context --project memvid --agent codex --budget-tokens 4000
memvid-context --project memvid --query "open risks service install"
memvid-context --project memvid --librarian --query "active bug next step"
memvid-context --project memvid --no-librarian --query "active bug next step"
memvid-queue-write --agent codex --project memvid --status done --type update <<'EOF'
Completed shard-routing fix and verified tests.
EOF
codex-memvid --dangerously-bypass-approvals-and-sandbox -- "continue the migration"
claude-memvid -- "continue the migration"
gemini-memvid --model gemini-2.5-pro -- "review the current state"
memvid-context-wrap -- your-agent-command
```

## CLI Launch Integration

Installed shell integration should make normal CLI launches route through Memvid automatically:

- `codex`, `claude`, and `gemini` shell functions call absolute wrapper paths.
- `codex-raw`, `claude-raw`, and `gemini-raw` bypass wrappers when direct launch is needed.
- Shell startup prepends the Memvid install prefix to `PATH` and exports `MEMVID_CONFIG`.
- Wrappers locate sibling `memvid-context` first, then fall back to `PATH`.
- Wrappers fail open: if startup recall fails, they launch the agent with a small fallback packet instead of blocking the CLI.

## Non-Shell Launchers

Launchers that do not evaluate shell startup files must call wrappers directly. Use installed wrapper paths as the command, then pass normal agent arguments unchanged:

```text
/usr/local/bin/codex-memvid
/usr/local/bin/claude-memvid
/usr/local/bin/gemini-memvid
/usr/local/bin/memvid-context-wrap -- your-agent-command
```

Desktop entries, IDE tasks, service managers, and other non-interactive launchers should point at `*-memvid` wrappers rather than raw agent binaries. Absolute calls to raw `codex`, `claude`, or `gemini` bypass Memvid startup recall because shell functions are not involved.

Use `memvid-context-wrap` for agents without a dedicated wrapper. It prepends Memvid startup context to the command after `--`:

```bash
/usr/local/bin/memvid-context-wrap -- /opt/tools/agent --flag value
```

## Compression

Startup recall uses a 7-day chrono-semantic horizon:

- 0-4 hours: near-full relevant snippets.
- 4-16 hours: rich recent context, still heavily favored in ranking.
- 16-48 hours: sharply reduced selection and much tighter compression.
- 2-4 days: compact task and project-state summaries only when still relevant.
- 4-7 days: canonical facts, risks, handoffs, and invariants.
- 7+ days: one-line facts or omission unless semantically critical.

Raw `.mv2` stores are never rewritten by compression. The packet is only a view over backend-owned stable project shards.

## Store Layout

Queue ingestion writes stable shards under `/var/lib/memvid/store`:

- `<project>.mv2` holds ordinary workspace memory for one project.
- `global.mv2` is reserved for explicit cross-project coordination.
- Queue records must include `[project:<name>]`; `[project:global]` is never inferred.

The injector opens stores read-only and falls back to a temporary snapshot copy if the live writer lock blocks direct reads. Startup context should be selected from the active project shard plus explicit global records. Other project shards are hidden unless `--include-other-projects` is set for debugging or migration.

Agents should treat injected context as read-only and write durable updates only through `/var/lib/memvid/queue`. Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state. Use `[project:global]` only for explicit cross-project coordination; normal workspace notes belong in the current project shard.

## Librarian Mode

`memvid-context` can call local OpenAI-compatible endpoint such as Ollama at `http://127.0.0.1:11434/v1/chat/completions`.

- Repo config enables librarian. `--librarian` is explicit override for ad hoc runs; `--no-librarian` forces heuristic baseline. If model call fails, startup falls back to heuristic recall.
- Heuristics still build bounded candidate pool first.
- Librarian sees only active project shard plus explicit global records.
- Recent typed notes in `/var/lib/memvid/librarian_queue` can steer recall selection. They are routing hints only, not durable facts.
- Valid librarian JSON can narrow final packet to selected record IDs and add short session brief.
- Timeouts, HTTP errors, malformed JSON, invalid IDs, over-selection, or empty selections fall back to heuristic recall.
- Packet header reports compact librarian diagnostics: status `enabled`, `disabled`, or `fallback`; candidate count; selected count; elapsed milliseconds; and `librarian_warning` when fallback occurs. `--include-store-errors` still controls detailed store warning blocks, but librarian fallback warning stays visible in header diagnostics.

Default local Qwen3/Ollama profile:

```toml
[librarian]
enabled = true
endpoint = "http://127.0.0.1:11434/v1/chat/completions"
model = "qwen3:8b"
timeout_ms = 30000
max_candidates = 6
max_selected = 6
max_tokens = 512
temperature = 0.0
top_p = 1.0
presence_penalty = 1.5
keep_alive = "-1"
```

Write recall-steering notes with `memvid-librarian-note`:

```bash
memvid-librarian-note --agent codex --project memvid --intent recall_focus <<'EOF'
Prioritize current wrapper diagnostics and unresolved installer dry-run risk.
EOF
```

Supported intents are `recall_question`, `recall_focus`, and `memory_correction`. To unload the warm librarian model without changing config, run `bash scripts/memvid-librarian-cold.sh`; next normal agent startup loads it again.
