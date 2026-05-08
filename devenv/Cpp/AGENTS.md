```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# C++ Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for C and C++ experiments, examples, and small tools. Put public headers under `include/`, implementation files under `src/`, tests under `tests/`, examples under `examples/`, and build presets or toolchain files under `cmake/`. Keep generated build output in `build/` and out of source control.

## Build, Test, and Development Commands

- `cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug` configures a debug build.
- `cmake --build build` compiles targets.
- `ctest --test-dir build --output-on-failure` runs tests.
- `cmake --build build --target clang-tidy` runs linting when target exists.
- `cmake --build build --target format` formats code when target exists.

Use direct compiler commands only for single-file probes; keep repeatable workflows in CMake presets or targets.

## Coding Style & Naming Conventions

Use `clang-format` with repo config when available. Prefer RAII, standard library containers, `std::unique_ptr` for ownership, `std::shared_ptr` only for shared lifetime, and `std::string_view` for borrowed text. Use `snake_case` for functions and variables, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for macros or constants that must be macros. Avoid raw `new` and `delete`.

## Testing Guidelines

Place tests in `tests/` using Catch2, GoogleTest, or repo-standard framework. Add focused tests for parsing, ownership, error handling, and platform boundaries. Include sanitizer runs for memory-sensitive changes when practical.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention ABI, compiler, platform, or packaging impact when relevant. Pull requests should include summary, tests run, supported compilers, and any sanitizer or benchmark notes.

## Security & Configuration Tips

Do not commit binaries, object files, build trees, local toolchains, credentials, core dumps, or generated artifacts. Validate external input before pointer arithmetic, filesystem writes, process launches, or deserialization.
