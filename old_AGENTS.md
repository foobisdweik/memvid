# Universal Agent Workflow & Memory Protocol

**CRITICAL:** This project uses a batched, GPU-accelerated **Memvid** pipeline as the *exclusive*, chronological source of truth and long-term memory. 

You must strictly adhere to the following workflow to maximize context efficiency and maintain an unbroken memory chain.

## 1. Core Philosophy: "Ingest Heavily, Search Semantically, Read Surgically"
Do not try to hold the entire project in your context window. Do not rely on massive file reads. You are expected to interact with the Memvid pipeline continuously.

## 2. Memory Ingestion (Writing)
**WHEN:** 
- After fixing a complex bug.
- After discovering an undocumented architectural constraint.
- When establishing a new project convention.
- At the end of a major feature implementation.

**HOW:**
You do not interact with the `.mv2` files directly. The pipeline is asynchronous.
1. Write a concise, highly-descriptive markdown file summarizing the context.
2. Save it to a temporary location (e.g., `/tmp/`).
3. Move it to the designated Memvid ingestion queue directory for this workspace.
```bash
echo "# Fix for Auth Bug\nThe token must be padded to 512 bytes before..." > /tmp/auth_fix.md
mv /tmp/auth_fix.md <path-to-ingestion-queue>/auth_fix_$(date +%s).md
```
*The background embedder and ingestor services will automatically pick this up, compute embeddings, and append it to the current day's `.mv2` store file.*

## 3. Memory Recall (Reading)
**WHEN:**
- At the beginning of a session or task to gain historical context.
- Before modifying a complex module you haven't touched recently.

**HOW:**
Use Memvid's semantic search against the active `.mv2` files in the storage directory to retrieve concentrated "smart frames" of context. Use this *instead* of broad code searches.

## 4. BANNED Actions & Commands
To prevent context window flooding and ensure Memvid remains the sole source of truth, the following are **STRICTLY PROHIBITED**:

1. **Native Agent Memory:** Do not use your built-in memory storage (e.g., Gemini's `MEMORY.md`, Claude's `<memory>` tools) for project facts, architecture, or conventions. Route EVERYTHING through the Memvid ingestion queue.
2. **Massive Reads:** Do not use `cat`, `read_file`, or `view` on large files (>500 lines) without specific line ranges. Rely on Memvid search to find the exact lines you need first.
3. **Recursive Grep Dumps:** Avoid unbounded `grep -r` or `find . -exec cat {} +` commands. They return too much noise. Use Memvid semantic search first, and fallback to highly-targeted greps only if necessary.
4. **Direct `.mv2` Edits:** Never attempt to manually edit, append to, or tamper with the `.mv2` files in the storage directory. They are managed exclusively by the Memvid ingestor to guarantee crash-safety and chronological integrity.
