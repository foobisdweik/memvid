```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# PHP Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for PHP libraries, web apps, and command-line tools. Put application code under `src/` or framework-standard directories, tests under `tests/`, public web entry points under `public/`, config under `config/`, and database migrations under `migrations/` or framework-standard path.

## Build, Test, and Development Commands

- `composer install` installs dependencies.
- `composer test` runs tests when configured.
- `vendor/bin/phpunit` runs PHPUnit directly.
- `vendor/bin/phpstan analyse` runs static analysis when configured.
- `vendor/bin/php-cs-fixer fix --dry-run --diff` verifies formatting when configured.

Use framework commands such as `php artisan`, `bin/console`, or `vendor/bin/phinx` when project standard requires them.

## Coding Style & Naming Conventions

Use PSR-12 unless project config differs. Prefer strict types, dependency injection, value objects for structured data, and parameterized queries through framework database APIs. Use `camelCase` for methods and variables, `UpperCamelCase` for classes, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Put tests under `tests/` using PHPUnit or Pest. Add focused tests for request handling, validation, persistence, serialization, and authorization. Use isolated databases or transactions for integration tests.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention PHP version, framework, database migration, public API, or deployment impact. Pull requests should include summary, tests run, and migration or rollback notes.

## Security & Configuration Tips

Do not commit `vendor/`, `.env`, credentials, uploaded files, dumps, caches, or generated artifacts. Validate request input, escape output, protect CSRF boundaries, and avoid unserializing untrusted data.
