```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# Java Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this repository. Treat Memvid startup context as the only durable recall surface. Treat `/var/lib/memvid/queue` as the only durable write path.

If an agent-native memory surface is available, ignore it for this project. Do not read it to answer project questions. Do not write it when work completes. Queue Markdown is the source of truth.

## Project Structure & Module Organization

This folder is for Java libraries, services, and command-line tools. Use Maven or Gradle standard layout: `src/main/java`, `src/main/resources`, `src/test/java`, and `src/test/resources`. Put integration tests under `src/integrationTest/java` or a clearly named module when project supports it.

## Build, Test, and Development Commands

- `./mvnw test` runs Maven tests when Maven wrapper exists.
- `./mvnw package` builds Maven artifact.
- `./gradlew test` runs Gradle tests when Gradle wrapper exists.
- `./gradlew build` builds Gradle project.
- `./gradlew spotlessCheck` or `./mvnw spotless:check` verifies formatting when configured.

Prefer wrapper scripts checked into project over system `mvn` or `gradle`.

## Coding Style & Naming Conventions

Use local Checkstyle, Spotless, or formatter config. Prefer constructor injection, immutable value objects, clear package boundaries, and explicit exception handling. Use `lowerCamelCase` for methods and fields, `UpperCamelCase` for classes and interfaces, and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Use JUnit 5 unless project standard differs. Add focused unit tests for domain logic and integration tests for database, HTTP, filesystem, or messaging boundaries. Keep tests deterministic with explicit clocks, temp directories, and isolated state.

## Commit & Pull Request Guidelines

Keep commits focused and imperative. Mention JVM version, API, schema, dependency, or deployment impact when relevant. Pull requests should include summary, tests run, and operational notes for services.

## Security & Configuration Tips

Do not commit `target/`, `build/`, local IDE metadata, credentials, keystores, dumps, or generated artifacts. Validate deserialization, SQL parameters, file paths, HTTP inputs, and process launches.
