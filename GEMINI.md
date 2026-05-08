# Gemini Instructions

Follow `AGENTS.md`. Gemini-specific durable memory is disabled for project facts, architecture, conventions, decisions, handoffs, and task state. Do not use native Gemini memory tooling or caches for this repository. Write durable records only as atomic queue Markdown to `/var/lib/memvid/queue/`.

Start sessions through installed `gemini-memvid` wrapper or shell function when available. It loads `memvid-context` and librarian automatically at CLI startup. Treat injected startup context as read-only recall and do not access `.mv2` stores directly.

Use `agent:gemini` in queued headers.
Use `[project:global]` only for explicit cross-project coordination. Ordinary workspace facts belong to current project shard.

## Memvid Queue — Mandatory Checkpoints

Write a queue entry at each trigger below. No exceptions.

**Write immediately when:**
- [ ] User confirms a fix works on device or in tests
- [ ] User explicitly identifies something as a bug (not you)
- [ ] A decision is finalized — user accepted an approach or code was committed
- [ ] A file, function, command, or protocol is created or renamed
- [ ] A task the user assigned is complete
- [ ] A test produces a concrete, unexpected result that changes direction
- [ ] A hard blocker is hit: missing dependency, broken tool, auth failure, device rejection

**Write before stopping when:**
- [ ] Session is ending or handing off to another agent
- [ ] Context compaction is imminent

**Do NOT write for:**
- Speculation, hypotheses, or your own suspicions about the code
- Behavior you infer without a failing test or user report to back it
- Intermediate steps within a single task
- Explanations or plans that haven't been acted on

One entry per logical unit. Do not batch unrelated facts.
Prefer the helper when available:

```bash
memvid-queue-write --agent gemini --project <project> --status done --type update <<'EOF'
<concise durable update>
EOF
```
