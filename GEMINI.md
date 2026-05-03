# Gemini Instructions

Follow `AGENTS.md`. Gemini-specific durable memory is disabled for project facts, architecture, conventions, and handoffs. Write those records to `/var/lib/memvid/queue/` with the atomic queue protocol.

Use `agent:gemini` in queued headers.
