# Universal Agent Instructions

## Memvid Queue Protocol

Agents use Memvid as the shared long-term memory backend. The only write interface is the queue directory. Agents must not invoke a memvid binary, inspect backend working directories, or touch `.mv2` files directly.

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

- Significant decision made.
- Bug found or fixed.
- New file, function, command, or convention created.
- Task completed.
- Context risk appears: compaction, handoff, model switch, tool switch, or long pause.

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
