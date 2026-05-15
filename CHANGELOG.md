# Changelog

All notable changes to Memvid will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2026-05-15 — Sealed shard rewrite

### Removed
- Entire Rust workspace: `memvid-core`, embedder, ingestor, librarian, queue writer, and all crate dependencies.
- Queue-based write pipeline under `/var/lib/memvid/queue/` and the librarian-driven recall path.
- Embedding models, vector indices, full-text search, frame-based storage, and WAL machinery.

### Added
- MV2 v3 sealed shard format: plain UTF-8, seven-line header, fixed body framing, SHA-256 chain across rotated shards. Spec in `MV2_SPEC.md`.
- Three-slot rotation per project (`current`, `.1`, `.2`) with shards evicted from `.2` archived under `xz -9e` in a per-project archive directory, retained indefinitely.
- Three bash tools: `memvid-write` (seal and rotate), `memvid-context` (read current / `--full` / `--verify` / `--history` / `--raw`), and the `claude-memvid` / `codex-memvid` / `gemini-memvid` wrappers that inject the current shard into the corresponding agent CLI.
- Shrinkage safety check in `memvid-write`: refuses a new body smaller than 25% of the prior body unless `--force` is given (only enforced when the prior body exceeds 256 bytes).

### Changed
- Agents now own the shard outright and rewrite it in full at meaningful milestones. Incremental queue entries are gone; each write is a complete snapshot, with the three-slot rotation and the archive providing the recovery path.
- Sealed shards are made read-only (`chmod 0444`) after each write to prevent silent overwrites.

## [Unreleased]

### Added
- Initial public release of Memvid core library
- Single-file `.mv2` format for portable AI memory
- Full-text search with BM25 ranking (Tantivy)
- Vector similarity search with HNSW
- PDF, DOCX, XLSX document ingestion
- CLIP visual embeddings for image search
- Whisper audio transcription
- Timeline queries for chronological browsing
- Crash-safe WAL-based writes
- Blake3 checksums for data integrity
- Ed25519 signatures for authenticity
- Optional AES-256-GCM encryption

### Security
- Embedded WAL prevents data corruption
- Atomic commits ensure consistency
- File locking prevents concurrent write conflicts

## [2.0.0] - 2026-01-05

### Added
- Complete rewrite in Rust for performance and safety
- New `.mv2` file format (single-file, no sidecars)
- Append-only frame-based architecture
- Built-in full-text and vector search
- Cross-platform support (macOS, Linux, Windows)

### Changed
- Migrated from Python to Rust
- New API design focused on simplicity
- Improved memory efficiency

### Removed
- Legacy Python implementation
- QR code video encoding (replaced with efficient binary format)

---

[Unreleased]: https://github.com/memvid/memvid/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/memvid/memvid/releases/tag/v2.0.0
