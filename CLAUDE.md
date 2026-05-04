# Claude Instructions

Follow `AGENTS.md`. Claude-specific durable memory is disabled for project facts, architecture, conventions, and handoffs. Write those records to `/var/lib/memvid/queue/` with the atomic queue protocol.

Start sessions through a Memvid context wrapper when available. Treat injected Memvid startup context as read-only recall and do not access `.mv2` stores directly.

Use `agent:claude` in queued headers.

## Memvid Queue — Mandatory Checkpoints

Write a queue entry at each trigger below. No exceptions.

**Write immediately when:**
- [ ] User confirms a fix works on device or in tests
- [ ] User explicitly identifies something as a bug (not you)
- [ ] A decision is finalized — user accepted an approach or you committed code
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
