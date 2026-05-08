```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# C# Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for .NET libraries, services, and tools. Put solution files at folder root, production projects under `src/`, tests under `tests/`, samples under `samples/`, and shared build props in `Directory.Build.props` or `Directory.Packages.props`.

## Build, Test, and Development Commands

- `dotnet restore` restores packages.
- `dotnet build` compiles solution.
- `dotnet test` runs tests.
- `dotnet format --verify-no-changes` verifies formatting when available.
- `dotnet run --project src/<ProjectName>` runs local app.

Use SDK version pinned by `global.json` when present.

## Coding Style & Naming Conventions

Use `.editorconfig` and nullable reference types where enabled. Prefer dependency injection, immutable records for value data, async APIs for IO, and explicit cancellation tokens for long-running operations. Use `PascalCase` for public members and types, `camelCase` for locals and parameters, and `_camelCase` for private fields when local style uses it.

## Testing Guidelines

Use xUnit, NUnit, or MSTest according to project standard. Add focused tests for domain logic, serialization, async behavior, and external boundaries. Use temp directories and fake clocks or test doubles for deterministic tests.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention target framework, public API, package, migration, or deployment impact. Pull requests should include summary, tests run, and operational notes for services.

## Security & Configuration Tips

Do not commit `bin/`, `obj/`, local user secrets, credentials, dumps, packages, or generated artifacts. Validate file paths, JSON input, SQL parameters, process starts, and network inputs.
