# Memvid Librarian Framework

Memvid librarian is a planned local LLM layer for startup recall selection. It should rank and summarize bounded candidate records, not discover raw memory directly.

## Responsibility Split

- `memvid-context` owns deterministic safety gates: active project shard, explicit `global.mv2`, age limits, byte limits, and fallback behavior.
- Librarian owns judgment inside that bounded pool: redundancy removal, stale-record rejection, relevance ranking, and session brief synthesis.
- Agents receive final Markdown context only. They do not query the librarian or `.mv2` stores directly.

## Candidate Contract

Input to the librarian should be compact candidate cards:

```text
id: <store>:<frame_id>
project: <project|global>
timestamp: <rfc3339>
status: <done|blocked|active|unknown>
type: <handoff|decision|risk|bug|update|unknown>
score: <heuristic score>
body: <bounded text>
```

Output must be machine-checkable:

```json
{
  "selected_ids": ["memvid.mv2:42"],
  "session_brief": "Current task state...",
  "dropped_ids": [{"id": "memvid.mv2:19", "reason": "obsolete"}]
}
```

`memvid-context` must reject malformed output and fall back to heuristic recall.

## Runtime Boundary

Model choice is external research. Framework requirements:

- runs locally on commodity 8GB VRAM
- supports deterministic-ish low-temperature instruction following
- accepts candidate cards through CLI or localhost HTTP
- returns JSON within startup latency budget
- never receives unrelated project shards

## Admin Workflow

1. Add config keys for librarian enablement, command or endpoint, timeout, and max candidate count.
2. Add `memvid-context --librarian` plus default-off settings.
3. Log selected and dropped IDs for comparison against heuristic recall.
4. Keep heuristic recall as default until repeated startup packets prove better quality.
5. Add smoke tests with a fake librarian binary that returns valid, malformed, and timeout responses.
