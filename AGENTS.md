# Universal Agent Instructions

## Memvid Queue Protocol

Agents use Memvid as the shared long-term memory backend. The only write interface is the queue directory. Agents must not invoke a memvid binary, inspect backend working directories, or touch `.mv2` files directly.

## Startup Recall

Agent sessions should be launched through a Memvid wrapper so a bounded startup context packet is injected before normal work begins.

- Codex wrapper: `codex-memvid`
- Claude wrapper: `claude-memvid`
- Gemini wrapper: `gemini-memvid`
- Generic wrapper: `memvid-context-wrap -- <agent command>`
- Context generator: `memvid-context`

The context packet is a read-only, compressed view of backend-owned source-of-truth stores. It uses chrono-semantic compression: recent records are shown with more detail, and ordinary records reach maximum compression at 7 days old. Semantically critical facts, handoffs, risks, protocol rules, and project matches can survive longer, but still as compact facts.

Agents may read the injected packet. Agents must not open `.mv2` files or backend directories to perform their own recall. If more recall is needed, ask the launcher/user for a narrower `memvid-context --query ...` packet instead of accessing the store directly.

Rotating source-of-truth stores live under `/var/lib/memvid/store/YYYY-MM-DD.mv2`. The injector handles daily rotation by scanning recent stores newest-first and returning source-attributed snippets. Agents do not need to know or manage store rotation.

## Queue Contract

- Queue path: `/var/lib/memvid/queue/`
- Backend paths are reserved: `/var/lib/memvid/processing/`, `/var/lib/memvid/ingest/`, `/var/lib/memvid/done/`, `/var/lib/memvid/failed/`, `/var/lib/memvid/store/`
- Write pure Markdown only.
- Write atomically: create a hidden temp file in the queue, then rename it into place.
- Use UUID filenames. Do not rely on timestamps alone for uniqueness.
- Maximum queue file size: 256 KB unless the user explicitly overrides this for a specific migration or diagnostic task.
- If the queue exceeds 10,000 files, slow write frequency and prefer a single handoff/update over many small files.
- After a file is renamed into the queue, do not retry, edit, delete, or move it. The backend owns eventual processing or failure handling.

## Metadata Header

Every queued Markdown file starts with this header:

```text
[agent:<agent-name>]
[status:in-progress|done|handing-off|error|migrated]
[type:update|handoff|error|import]
[project:<project-or-global>]
[timestamp:<unix_ns>]
```

Use the current project name when known. Use `global` for cross-project operating context.

## Standard Write

Preferred — use the helper (handles dedup automatically):

```bash
memvid-queue-write \
  --agent "${MEMVID_AGENT:-agent}" \
  --project "<PROJECT>" \
  --status "<STATE>" \
  --type update <<'EOF'
<concise prose — no header needed, helper adds it>
EOF
```

Fallback (if helper unavailable) — raw atomic write, no dedup:

```bash
queue=/var/lib/memvid/queue
tmp=$(mktemp "$queue/.tmp.XXXXXX")
timestamp=$(date +%s%N)

cat > "$tmp" <<EOF
[agent:${AGENT_NAME:-agent}]
[status:<STATE>]
[type:update]
[project:<PROJECT>]
[timestamp:$timestamp]

<concise prose or task output>
EOF

chmod 0644 "$tmp"
mv "$tmp" "$queue/$(uuidgen).md"
```

## Mandatory Write Triggers

Write to the queue when:

- User confirms a fix works on device or in tests.
- User explicitly identifies something as a bug (not the agent).
- A decision is finalized — user accepted an approach or code was committed.
- A file, function, command, or protocol is created or renamed.
- A task the user assigned is complete.
- A test produces a concrete, unexpected result that changes direction.
- A hard blocker is hit: missing dependency, broken tool, auth failure, device rejection.
- Session is ending or handing off to another agent.
- Context compaction is imminent.

Do NOT write for:

- Speculation, hypotheses, or agent suspicions about the code.
- Behavior inferred without a failing test or user report to back it.
- Intermediate steps within a single task.
- Explanations or plans that have not been acted on.

Keep entries concise and high signal. Do not dump large logs, dependency output, generated files, or broad file contents.

## Handoff

```bash
queue=/var/lib/memvid/queue
tmp=$(mktemp "$queue/.tmp.XXXXXX")
timestamp=$(date +%s%N)

cat > "$tmp" <<EOF
[agent:${AGENT_NAME:-agent}]
[status:handing-off]
[type:handoff]
[project:<PROJECT>]
[timestamp:$timestamp]

## Handoff

### Accomplished
<description>

### State
<files, commands, open bugs>

### Next
<actions for next agent>
EOF

chmod 0644 "$tmp"
mv "$tmp" "$queue/${timestamp}_handoff_$(uuidgen).md"
```

## Memory Boundaries

- Do not use native agent memory for project facts, architecture, or conventions when the queue is available.
- Do not create persistent agent-specific memory files unless required for tool startup.
- If an agent requires a local instruction file to run, keep it limited to this protocol and route durable knowledge through Memvid.
- Search or inspect ordinary project files only as needed for the current task; do not use broad reads as a memory substitute.
