# MV2 Sealed Shard Specification

Version 3.0

## Purpose

A `.mv2` file is a sealed, tamper-evident, single-shard memory container for one project. It exists to give a coding agent a durable record of project context that cannot be silently rewritten, and that can be verified against a short chain of prior shards.

Rotation policy is fixed at three live shards on disk (`current`, `.1`, `.2`). The shard evicted from `.2` is compressed with `xz -9e` and retained in the per-project archive directory indefinitely. The archive is for emergency recovery; routine reads only touch the three live shards.

A v3 shard is plain UTF-8 text: a small line-oriented header followed by a fixed body framing. No embeddings, no indices, no WAL. Everything in the file is human-readable.

## File layout

```
<header line: magic + version>
<header line: project>
<header line: agent>
<header line: ts>
<header line: prev-sha256>
<header line: body-sha256>
<header line: body-bytes>
---BEGIN BODY---
<exactly body-bytes bytes of body>
\n
---END BODY---
\n
```

Each header line ends with a single `\n`. The body is exactly `body-bytes` bytes verbatim — it may contain any bytes including newlines and is not interpreted. After the body, the writer emits a single separator `\n`, then the literal line `---END BODY---\n`, and the file ends.

Total file size is therefore exactly:

```
header_bytes + len("---BEGIN BODY---\n") + body-bytes + 16
```

Where 16 is `len("\n---END BODY---\n")`.

## Header fields

All header fields are mandatory and appear in the following order:

| Line | Format | Description |
|------|--------|-------------|
| 1 | `MV2 SHARD v3` | Magic line. Identifies format and major version. |
| 2 | `project: <name>` | Project name. Pattern: `[A-Za-z0-9._-]+`. The reserved name `global` is for cross-project shards. |
| 3 | `agent: <name>` | Identifier of the agent that authored this shard. |
| 4 | `ts: <iso8601-utc>` | Write timestamp in `YYYY-MM-DDThh:mm:ssZ` form. |
| 5 | `prev-sha256: <hex|none>` | SHA-256 of the entire prior `current` file (the file that will become `.1` after this write), or the literal `none` for the first shard ever written for this project. |
| 6 | `body-sha256: <hex>` | SHA-256 of the `body-bytes` bytes of the body. |
| 7 | `body-bytes: <integer>` | Byte length of the body, decimal, no leading zeros. |

Hex hashes are 64 lowercase hex characters.

## Hash chain

Each shard authenticates the prior shard's *entire file contents* via `prev-sha256`. After rotation:

- `current.prev-sha256 == sha256(<current_.1_file>)`
- `.1.prev-sha256 == sha256(<current_.2_file>)`
- `.2.prev-sha256` references a shard that has either been archived or was the project's first shard. `--verify` does not enforce `.2`'s chain link, since the predecessor is no longer in the live rotation.

A tamper in `.2` would change its file hash, so `.1`'s `prev-sha256` would no longer match. A tamper in `.1` likewise breaks `current.prev-sha256`. A tamper in `current` is detected only by its own internal hashes (`body-sha256` and structural invariants), since there is no successor. The primary defense for `current` is the filesystem-enforced `0444` mode.

## Immutability

After a successful write, the shard file is `chmod 0444`. Routine writers and agents cannot accidentally overwrite it. A user with write permission on the containing directory can still replace it via rotation (because `rename(2)` only needs write permission on the parent), but that path is the supported one and produces a new sealed file with a fresh hash chain link.

## Rotation contract (writer)

A writer must, on each successful seal:

1. Build the new shard file as a temp file in the shards directory.
2. `fsync` (best effort) and `chmod 0444` the temp file.
3. If `.2` exists: `xz -9e` it into the per-project archive directory, then remove `.2`.
4. If `.1` exists: rename `.1` → `.2`.
5. If `current` exists: rename `current` → `.1`.
6. Rename the temp file → `current`.

All renames are atomic on local filesystems. Failure between steps 4 and 6 leaves the project in a recoverable state (no `current` file, but `.1` carries the immediately prior content).

## Archive

The per-project archive directory contains `xz -9e` compressed copies of every shard ever evicted from the `.2` slot. Filenames are:

```
<sanitized-ts>-<first-8-hex-of-file-sha256>.mv2.xz
```

Where `<sanitized-ts>` is the shard's own `ts:` header with `:` replaced by `_`. The archive directory grows monotonically; nothing is pruned.

## Verification

A verifier should, for each of `current`, `.1`, `.2`:

1. Confirm the file begins with `MV2 SHARD v3\n`.
2. Parse the header lines.
3. Locate `---BEGIN BODY---\n`, extract exactly `body-bytes` bytes immediately following it, and confirm `sha256(body) == body-sha256`.
4. Confirm the total file size equals `header_offset_of_BEGIN_marker + 17 + body-bytes + 16`.
5. Confirm the last 15 bytes of the file are `---END BODY---\n`.
6. For the hash chain, recompute `sha256(prior_file)` and compare against the successor's `prev-sha256`. `current` chains to `.1`; `.1` chains to `.2`. `.2`'s chain link is not verified against a live predecessor.

## Why this is enough

Single-agent project ownership rules out concurrent-writer races. The rotation gives one tamper-evident predecessor chain and a fully retained archive for emergency reconstruction. Self-pruning at write time is the agent's responsibility, and the shrinkage safeguard in the writer (refuse new body smaller than 25% of prior body without `--force`) catches the most common accidental-truncation failure mode.

Nothing in this format requires a database, an index, or an embedding. The file is grep-able, diff-able, and recoverable with standard Unix tools.
