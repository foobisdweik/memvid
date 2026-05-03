# Gemini Instructions

Follow `AGENTS.md`. Gemini-specific durable memory is disabled for project facts, architecture, conventions, and handoffs. Write those records to `/var/lib/memvid/queue/` with the atomic queue protocol.

Start sessions through `gemini-memvid` when available. Treat injected Memvid startup context as read-only recall and do not access `.mv2` stores directly.

Use `agent:gemini` in queued headers.
