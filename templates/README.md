# Memvid Agent-Contract Templates

Drop-in templates for new project workspaces. Each file is the agent-facing contract memvid agents read at session start (`AGENTS.md` is the generic surface; `CLAUDE.md` and `GEMINI.md` are per-agent specializations that defer to it).

## Usage

```bash
# from inside the new project root
cp /path/to/memvid/templates/{AGENTS,CLAUDE,GEMINI}.md .

# replace the placeholder with the project's actual name
PROJECT=myproject
sed -i "s/<project>/$PROJECT/g" AGENTS.md CLAUDE.md GEMINI.md
```

The project name must match `[A-Za-z0-9._-]+` and becomes the shard identifier under `/var/lib/memvid/shards/<project>.mv2`.

After the substitution, delete the `<!-- SETUP: ... -->` comment block at the top of each file.

## What survives the templating

- The full write contract, prune rules, and "what the shard should contain" guidance.
- The `memvid-write` / `memvid-context` invocation examples.
- The `[project:global]` cross-project convention.

## What changes per project

- The literal project name in the example `--project` flags.
