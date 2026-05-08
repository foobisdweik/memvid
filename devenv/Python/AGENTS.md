```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# Python Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for Python packages, scripts, and notebooks. Put package code under `src/` or a named package directory, tests under `tests/`, command-line entry points under `scripts/`, fixtures under `tests/fixtures/`, and examples under `examples/`. Keep virtual environments and generated caches out of source control.

## Build, Test, and Development Commands

- `python -m venv .venv` creates local virtual environment.
- `python -m pip install -e '.[dev]'` installs editable package with development extras when `pyproject.toml` supports it.
- `python -m pytest` runs tests.
- `python -m ruff check .` runs linting.
- `python -m ruff format --check .` verifies formatting.
- `python -m mypy .` runs type checking when configured.

Use `uv` commands when project already standardizes on `uv`.

## Coding Style & Naming Conventions

Use Ruff formatting and lint rules from local config. Prefer type hints for public functions, dataclasses or Pydantic models for structured data, pathlib for paths, and context managers for files and resources. Use `snake_case` for modules, functions, and variables; `UpperCamelCase` for classes; `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Put tests in `tests/` with descriptive `test_*.py` names. Add focused tests for parsing, IO boundaries, errors, and CLI behavior. Use fixtures for reusable setup and mark slow or integration tests clearly.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention user-visible CLI, API, dependency, or migration impact. Pull requests should include summary, tests run, Python versions, and any packaging notes.

## Security & Configuration Tips

Do not commit `.venv`, `__pycache__`, notebooks with secrets, credentials, tokens, local data, model files, or generated artifacts. Validate untrusted paths, subprocess arguments, deserialization input, and template rendering.
