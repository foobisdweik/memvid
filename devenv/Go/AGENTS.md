```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# Go Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for Go modules, services, and command-line tools. Put commands under `cmd/<name>/`, reusable packages under `internal/` or `pkg/`, tests beside source as `*_test.go`, examples under `examples/`, and deployment config under `deploy/`.

## Build, Test, and Development Commands

- `go test ./...` runs all tests.
- `go test -race ./...` runs race detector when concurrency or shared state changes.
- `go vet ./...` runs static checks.
- `gofmt -w .` formats source.
- `go build ./...` compiles all packages.

Use `go run ./cmd/<name>` for local command execution.

## Coding Style & Naming Conventions

Use `gofmt` and idiomatic Go. Prefer small interfaces at call sites, explicit context propagation, error wrapping with `%w`, and table-driven tests. Use short names in small scopes, descriptive names in exported APIs, `MixedCaps` for exported identifiers, and `mixedCaps` for unexported identifiers.

## Testing Guidelines

Put tests beside package code. Add table-driven tests for pure logic and integration tests for network, database, filesystem, and process boundaries. Use `t.TempDir`, explicit contexts, and deterministic clocks.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention module, API, concurrency, dependency, or deployment impact when relevant. Pull requests should include summary, tests run, and any race or benchmark notes.

## Security & Configuration Tips

Do not commit binaries, coverage output, credentials, local env files, or generated artifacts. Validate paths, HTTP input, SQL parameters, archive extraction, and subprocess arguments.
