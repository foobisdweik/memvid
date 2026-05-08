```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# Ruby Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for Ruby gems, scripts, and web apps. Put library code under `lib/`, executables under `exe/` or `bin/`, tests under `test/` or `spec/`, fixtures under `test/fixtures` or `spec/fixtures`, and app code in framework-standard directories when using Rails or Hanami.

## Build, Test, and Development Commands

- `bundle install` installs dependencies.
- `bundle exec rake test` runs Minitest when configured.
- `bundle exec rspec` runs RSpec when configured.
- `bundle exec rubocop` runs linting.
- `bundle exec rake build` builds gem packages when configured.

Use binstubs such as `bin/rails` or `bin/rake` when present.

## Coding Style & Naming Conventions

Use RuboCop rules from local config. Prefer small objects, explicit keyword arguments for complex calls, frozen constants, and clear boundaries around IO and persistence. Use `snake_case` for methods, variables, and files; `UpperCamelCase` for classes and modules; `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Use Minitest or RSpec according to project standard. Add focused tests for parsing, persistence, validation, background jobs, and authorization. Keep tests deterministic with temp directories, transaction rollback, and explicit time control.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention Ruby version, gem API, migration, dependency, or deployment impact. Pull requests should include summary, tests run, and migration or operational notes.

## Security & Configuration Tips

Do not commit `.bundle`, `vendor/bundle`, credentials, master keys, dumps, uploads, caches, or generated artifacts. Validate request input, escape rendered output, parameterize queries, and avoid unsafe deserialization.
