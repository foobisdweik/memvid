```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# JavaScript Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for JavaScript libraries, scripts, and browser or Node prototypes. Put source under `src/`, tests under `test/` or `tests/`, browser assets under `public/`, examples under `examples/`, and generated bundles under `dist/` only when release artifacts are intentional.

## Build, Test, and Development Commands

- `npm install` installs dependencies when `package-lock.json` exists.
- `npm run dev` starts local development server when configured.
- `npm test` runs test suite.
- `npm run lint` runs linting.
- `npm run format:check` verifies formatting when configured.
- `npm run build` creates production bundle or package output.

Use `pnpm` or `yarn` only when lockfile shows that package manager.

## Coding Style & Naming Conventions

Use ESLint and Prettier from local config. Prefer ES modules, `const` by default, explicit async error handling, and small pure functions around side effects. Use `camelCase` for variables and functions, `UpperCamelCase` for classes and components, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Put tests near source or under `tests/` using repo-standard runner such as Vitest, Jest, Mocha, or Node test runner. Add focused tests for parsing, async behavior, DOM interaction, and API boundaries. Mock network and time where deterministic tests need it.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention browser support, Node version, bundle size, dependency, or API impact when relevant. Pull requests should include summary, tests run, and screenshots for UI changes.

## Security & Configuration Tips

Do not commit `node_modules`, local env files, credentials, tokens, generated bundles unless intended, or coverage output. Validate user input before DOM insertion, filesystem access, subprocess calls, and network requests.
