# Memvid Librarian Framework

Memvid librarian is config-enabled local LLM layer for startup recall selection. It ranks and summarizes bounded candidate records, not raw memory.

## Responsibility Split

- `memvid-context` owns deterministic safety gates: stable active project shard, explicit `global.mv2`, age limits, byte limits, and fallback behavior.
- Librarian owns judgment inside that bounded pool: redundancy removal, stale-record rejection, relevance ranking, and session brief synthesis.
- Agents receive final Markdown context only. They do not query the librarian or `.mv2` stores directly.
- Agents may write typed recall-steering notes to `/var/lib/memvid/librarian_queue`; `memvid-context` treats those notes as routing hints, not memory facts.

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

`selected_ids` must contain only candidate IDs and should stay small. Default maximum is 6. `dropped_ids` must include every non-selected candidate with one reason from: `duplicate`, `stale_superseded`, `resolved_done`, `wrong_project`, `global_not_needed`, `low_signal`, `too_old`, `unknown`.

`memvid-context` rejects malformed output, unknown IDs, duplicate IDs, missing drop reasons, over-selection, and empty selections. Rejection falls back to heuristic recall, so startup still ships a usable packet.

## Librarian Request Queue

Use `memvid-librarian-note` for agent-to-librarian requests:

```bash
memvid-librarian-note --agent codex --project memvid --intent recall_question <<'EOF'
Which current handoff records mention wrapper diagnostics?
EOF
```

Request records use Markdown headers:

```text
[agent:<agent>]
[project:<project|global>]
[intent:<recall_question|recall_focus|memory_correction>]
[timestamp:<unix-nanos>]

<specific request body>
```

`memvid-context` reads recent project/global request records, ignores malformed or oversized records, and includes the rest in the librarian prompt. Request bodies never become durable facts; the session brief must still use selected candidate IDs only.

## Runtime Boundary

Model choice is external research. Framework requirements:

- runs locally on commodity 8GB VRAM
- supports low-temperature JSON instruction following
- accepts candidate cards through OpenAI-compatible localhost HTTP
- returns JSON within startup latency budget
- never receives unrelated project shards or implicit cross-project data

Default Ollama profile for `qwen3:8b` uses 12 candidates, max 6 selected records, 20s timeout, 512 output tokens, `temperature = 0.0`, `top_p = 1.0`, `presence_penalty = 1.5`, `keep_alive = "-1"`, and `/no_think` in the user prompt. Run `bash scripts/memvid-librarian-cold.sh` to unload the model for the current machine session; next normal agent startup wakes it again.

## Admin Workflow

1. Use `memvid-context --no-librarian` for heuristic baseline packets.
2. Use default config for librarian packets; fail-open fallback stays active if model call fails.
3. Compare selected IDs, packet size, JSON parse success, fallback count, and repeated-run stability.
4. Keep generated `.mv2` fixtures under `target/librarian-eval`; commit Markdown fixture records only.
5. Add a local proxy later to capture request/response and test valid, malformed, and timeout paths.

## Manual Tuning Loop

Build once:

```bash
cargo build -p memvid-context
```

Compare packets:

```bash
target/debug/memvid-context --project memvid --no-librarian --query "active bug next step" > /tmp/heuristic.md
target/debug/memvid-context --project memvid --query "active bug next step" > /tmp/librarian.md
diff -u /tmp/heuristic.md /tmp/librarian.md
```

Run repeated librarian packets when changing prompt or runtime settings:

```bash
for i in 1 2 3; do
  target/debug/memvid-context --project memvid --query "active bug next step" > "/tmp/librarian-$i.md"
done
```

Accept tuning only when repeated packets preserve required current-state records, avoid wrong-project/global noise, stay inside budget, and do not fall back unexpectedly.
