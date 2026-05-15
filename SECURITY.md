# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 3.x     | :white_check_mark: |
| < 3.0   | :x:                |

Version 3 is a ground-up rewrite. Prior versions (Rust workspace with embedder/ingestor pipeline) are out of scope for security maintenance.

## Reporting a Vulnerability

Do not open a public GitHub issue for security vulnerabilities. Email **security@memvid.com** instead.

Include in your report:

- Description of the vulnerability.
- Steps to reproduce.
- Potential impact.
- Suggested fix (if any).

### Response timeline

- Acknowledgment within 48 hours.
- Severity assessment and triage.
- Critical issues addressed within 7 days.
- Coordinated disclosure with credit (unless you prefer anonymity).

## Scope

In scope:

- Path traversal in `--project` argument handling.
- Data leakage from sealed shards or archives.
- Tamper detection bypass in `--verify`.
- Permission escalation through the installer.
- Symlink races in the rotation path.

Out of scope:

- Tampering by an attacker with the same UID as the agent (the `chmod 0444` defense is for accidents, not for adversarial users sharing your account).

## Security model

Sealed shards are written with `chmod 0444` and rotated via atomic `rename(2)`. Each shard records the SHA-256 of the prior shard's full file (`prev-sha256`), forming a tamper-evident chain across the live rotation. `memvid-context --verify` recomputes the chain and the structural invariants (magic line, body byte count, end marker, no trailing bytes).

The archive is append-only by convention: rotated-out shards are compressed with `xz -9e`, set `chmod 0444`, and retained indefinitely. The system does not encrypt shards; if you need encrypted storage, encrypt the shards or state directory at a layer above (e.g., LUKS, encrypted home directory).

## Best practices

- Run agents as a dedicated UID with write access only to the shards and archive directories.
- Restrict the parent directory permissions if you do not want UID peers reading shards.
- Validate `--project` arguments are constrained to `[A-Za-z0-9._-]+` (the tools enforce this; do not bypass it).
- Keep `memvid-write` and `memvid-context` on the same release version as the format spec they were built against.
