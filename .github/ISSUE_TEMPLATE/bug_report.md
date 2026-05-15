---
name: Bug Report
about: Report a bug to help us improve Memvid
title: '[BUG] '
labels: 'bug'
assignees: ''
---

## Bug Description
A clear and concise description of the bug.

## Steps to Reproduce
1.
2.
3.

## Expected Behavior

## Actual Behavior

## Environment
- **OS**: [e.g., Ubuntu 22.04, Arch, macOS 14]
- **Shell**: `bash --version`
- **coreutils**: `sha256sum --version | head -n1`
- **Memvid version**: commit SHA or release tag

## Shard header (if relevant)
Paste the first seven lines of the affected shard (the header before `---BEGIN BODY---`). Redact `project:` and `agent:` if sensitive.

```
MV2 SHARD v3
project: ...
agent: ...
ts: ...
prev-sha256: ...
body-sha256: ...
body-bytes: ...
```

## Command + error output
```
$ memvid-write ...
<paste error here>
```

## Additional Context
Any other context (logs, file sizes, archive count, etc.).

## Checklist
- [ ] I have searched existing issues for duplicates
- [ ] I have tested with the latest version
- [ ] I can reproduce this consistently
