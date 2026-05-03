# Memvid GPU Embedding Pipeline Refactor Plan

## 1. Goal
Break through CPU embedding bottlenecks by introducing a batched, GPU-backed pipeline while enforcing a universal, time-series memory substrate for agentic systems. This is achieved by creating a strict single-writer ingestor that rotates `.mv2` files daily, ensuring an unbroken, chronological chain of ground-truth memory (`YYYY-MM-DD.mv2`).

## 2. Architecture Overview
We will transition to a "Clean Workspace" layout. The existing library code will move into `crates/core`, and new services will be added to create a pipeline connected via a shared directory queue.

### 2.1 Workspace Structure
```text
memvid/
├── Cargo.toml               # Root workspace manifest
├── crates/
│   ├── core/                # Current memvid-core library
│   ├── common/              # Shared types, config, job models
│   ├── embedder/            # GPU inference service (ONNX Runtime / CUDA)
│   └── ingestor/            # Single-writer MV2 storage engine
├── config/
│   └── settings.toml        # Global configuration
└── queue/                   # Shared directory for IPC (queue, processing, done, failed)
```

### 2.2 Component Responsibilities

#### `memvid-common`
- Defines the `Settings` struct populated from `config/settings.toml`.
- Defines the `Job` schema for items passing through the pipeline.
- Implements utility functions for moving jobs between queue states (e.g., `queue/` -> `processing/`).

#### `memvid-embedder`
- Continuously polls the `queue/` directory.
- Batches multiple text jobs (up to `batch_size`).
- Tokenizes and computes embeddings using a GPU-accelerated ONNX Runtime session (`ort` crate with CUDA execution provider).
- Writes the resulting embeddings alongside the job payload and atomically moves them to the `processing/` (or a dedicated `ingest/`) directory for the ingestor.

#### `memvid-ingestor`
- Continuously polls for completed embedding batches.
- **Daily Rotation Logic:** 
  - Before writing, it checks the current date (UTC or local, configurable but UTC recommended for agents).
  - It constructs the filename: `store/YYYY-MM-DD.mv2`.
  - If the file does not exist, it initializes a new `Memvid` instance.
  - If the day changes during operation, it commits, closes yesterday's file, and opens today's.
- Appends the text and embeddings to the active `.mv2` file.
- Commits batches based on `commit_interval`.
- Moves finished jobs to the `done/` directory.

## 3. Implementation Steps

### Phase 1: Workspace Restructuring
1. Create `crates/` directory.
2. Move all current source code, tests, examples, and benches into `crates/core`.
3. Update paths in `crates/core/Cargo.toml` (if any local relative paths exist, though most should be fine).
4. Create the root `Cargo.toml` as a `[workspace]`.

### Phase 2: Common Crate & Configuration
1. Initialize `crates/common`.
2. Implement `Settings` parsing using `toml` and `serde`.
3. Implement the IPC directory structure manager (ensure directories exist on startup).
4. Create the `config/settings.toml` template.

### Phase 3: The Embedder Service
1. Initialize `crates/embedder`.
2. Implement directory polling logic.
3. Integrate `tokenizers` and `ort` (with `cuda` feature).
4. *Stub Implementation First:* Build the polling and batching logic with a fake/dummy embedder to test pipeline flow, then plug in the actual ONNX session initialization.

### Phase 4: The Ingestor Service (Daily Rollover)
1. Initialize `crates/ingestor`.
2. Implement polling for embedded jobs.
3. Implement the **Daily File Rotation** logic.
4. Integrate `memvid-core` to write frames and commit based on interval.
5. Move completed jobs to the `done` folder.

### Phase 5: Verification & End-to-End Testing
1. Ensure all `memvid-core` tests still pass in the new workspace layout.
2. Write a pipeline integration test that drops dummy text files into the `queue/`, runs both services briefly, and verifies that a `YYYY-MM-DD.mv2` file is generated containing the data.

## 4. Dependencies
- `memvid-core` (internal)
- `memvid-common` (internal)
- `serde`, `toml`, `anyhow`, `uuid`
- `crossbeam-channel` (for internal thread coordination)
- `walkdir` or `notify` (for directory polling)
- `ort` (with `cuda` feature), `ndarray`
- `tokenizers`
- `chrono` (for daily date logic)

## 5. Security & Safety
- Moving the core into a workspace does not alter the fundamental crash-safety of the `.mv2` format.
- The `ingestor` acts as the exclusive writer for the active day's file, preventing file-level lock contention.
