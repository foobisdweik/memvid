```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# TypeScript Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for TypeScript packages, services, and frontend tools. Put source under `src/`, tests under `test/` or `tests/`, shared type declarations under `types/` only when needed, examples under `examples/`, and emitted files under `dist/`.

## Build, Test, and Development Commands

- `npm install` installs dependencies when `package-lock.json` exists.
- `npm run typecheck` runs TypeScript compiler checks.
- `npm test` runs tests.
- `npm run lint` runs linting.
- `npm run format:check` verifies formatting when configured.
- `npm run build` emits package or app output.

Use `pnpm` or `yarn` only when lockfile shows that package manager.

## Coding Style & Naming Conventions

Use strict TypeScript where possible. Prefer explicit public types, discriminated unions for variants, `unknown` over `any`, and runtime validation at external boundaries. Use `camelCase` for variables and functions, `UpperCamelCase` for types/classes/components, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Put tests near source or under `tests/` using repo-standard runner such as Vitest, Jest, Playwright, or Node test runner. Add focused tests for type-sensitive transforms, async behavior, UI state, and external API boundaries.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention public type, API, build, dependency, or browser compatibility impact. Pull requests should include summary, tests run, and screenshots or traces for UI changes.

## Security & Configuration Tips

Do not commit `node_modules`, local env files, credentials, tokens, generated bundles unless intended, or coverage output. Validate untyped JSON, route params, form data, and API responses before use.
