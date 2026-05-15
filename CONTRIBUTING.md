# Contributing to Memvid

Contributions welcome. Memvid is now a small set of bash tools plus a documented shard format. Keep changes minimal and aligned with the design in `MV2_SPEC.md` and `AGENTS.md`.

## Setup

Prerequisites: `bash`, GNU coreutils (`sha256sum`, `head`, `tail`, `mktemp`), `awk`, `xz`, `grep`, `sed`. For development you also want `shellcheck` and `make`.

```bash
git clone https://github.com/foobisdweik/memvid.git
cd memvid
make test
```

## Development workflow

1. Branch from `main`: `git checkout -b feature/<short-name>` or `fix/<short-name>`.
2. Edit. Keep tools self-contained and POSIX-friendly where reasonable.
3. Lint and test:
   ```bash
   make lint
   make test
   ```
4. Commit with a concise imperative subject (`fix:`, `docs:`, `feat:`).
5. Open a PR. CI runs `make lint` and `make test`.

## Shell style

- Shebang: `#!/usr/bin/env bash`.
- First line of body: `set -euo pipefail`.
- Quote variables (`"$var"`) and use `[[ ]]` for tests.
- No silent failures. Print to stderr with a script-name prefix on error paths.
- Prefer GNU coreutils and standard Unix tools. No Python, Rust, or external runtime dependencies for the tools.

## Adding a test

Tests live in `tests/test_*.sh`. Each script is invoked from `make test`. A test:

- Creates its own `mktemp -d` scratch dir.
- Exports `MEMVID_SHARDS_DIR` and `MEMVID_ARCHIVE_DIR` to isolate state.
- Uses the helpers `fail`/`pass`/`assert_eq` already established in the existing tests.
- Cleans up via `trap`.

## Format changes

The MV2 v3 shard format is defined in `MV2_SPEC.md`. Format changes must:

1. Update `MV2_SPEC.md` first.
2. Update `memvid-write` (serializer) and `memvid-context` (parser, verifier).
3. Add a regression test in `tests/test_shard_lifecycle.sh`.
4. Document the format version bump in `CHANGELOG.md`.

## Reporting bugs

Include in the report:

- Shell version (`bash --version`), OS, coreutils version.
- The `.mv2` file involved (if not sensitive) or its `head -n7` header lines.
- Reproduction steps.
- Expected vs actual behavior.

Open an issue at https://github.com/foobisdweik/memvid/issues.
