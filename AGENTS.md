
```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# Repository Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This is a Rust workspace. Core library code lives in `crates/core/src`, with integration tests in `crates/core/tests` and examples in `crates/core/examples`. Supporting binaries are split into focused crates: `crates/context` for startup recall, `crates/embedder` and `crates/ingestor` for queue processing, `crates/migrator` for legacy imports, and `crates/common` for shared settings and filesystem helpers. Deployment assets live in `deploy/`, installer packaging in `packaging/` and `install/`, Docker files in `docker/`, and user-facing docs in `docs/`.

## Build, Test, and Development Commands

- `make check` runs `cargo check --features lex,pdf_extract`.
- `make build` builds the workspace in debug mode.
- `make build-release` builds optimized release artifacts.
- `make test` runs the default test suite.
- `make test-integration` runs selected integration tests such as lifecycle, search, mutation, and recovery.
- `make fmt-check` verifies formatting without rewriting files.
- `make clippy` runs Clippy for all targets with warnings denied.
- `make verify` runs check, formatting, Clippy, and tests.

Use direct Cargo commands for crate-specific work, for example `cargo test -p memvid-core search --features lex,pdf_extract`.

## Coding Style & Naming Conventions

Use standard Rust formatting via `cargo fmt --all`. Follow idiomatic Rust naming: modules and functions use `snake_case`, types use `UpperCamelCase`, and constants use `SCREAMING_SNAKE_CASE`. Keep crates focused on their domain and prefer shared helpers in `crates/common` only when behavior is genuinely reused. Avoid unrelated formatting churn in large generated or data files.

## Testing Guidelines

Put integration tests under `crates/core/tests` with descriptive snake-case file names. Add focused tests for search, lifecycle, mutation, recovery, and format compatibility when touching those paths. Run `make test` before broad changes and `make verify` before submitting. Some fixtures are optional; tests should skip cleanly when large external assets are absent.

## Commit & Pull Request Guidelines

Recent history uses concise, imperative subjects, often with a scope or prefix, such as `fix: ...`, `docs(i18n): ...`, or `Tune context recall scoring`. Keep commits focused and mention user-visible behavior when relevant. Pull requests should include a short summary, tests run, linked issues, and any operational notes for installers, systemd services, Docker, or memory-store compatibility.

## Security & Configuration Tips

Default service paths are configured in `config/settings.toml` under `/var/lib/memvid`. Do not commit local stores, queues, model files, credentials, or generated `.mv2` memory artifacts.
