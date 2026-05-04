use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use memvid_core::{FrameStatus, Memvid};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const DEFAULT_MAX_FILE_BYTES: usize = 256 * 1024;
const HEADER_RESERVE_BYTES: usize = 2048;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Extract legacy .mv2 text and emit queue-ready Markdown"
)]
struct Args {
    /// Input .mv2 files.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Output queue directory.
    #[arg(long, default_value = "/var/lib/memvid/queue")]
    queue: PathBuf,

    /// Project value for the queue metadata header.
    #[arg(long, default_value = "legacy")]
    project: String,

    /// Max queue-ready Markdown file size in bytes.
    #[arg(long, default_value_t = DEFAULT_MAX_FILE_BYTES)]
    max_file_bytes: usize,

    /// Number of emitted queue files between throttles.
    #[arg(long, default_value_t = 500)]
    batch_size: usize,

    /// Throttle in milliseconds between batches.
    #[arg(long, default_value_t = 300)]
    throttle_ms: u64,

    /// Slow migration when queue contains more than this many non-temp files.
    #[arg(long, default_value_t = 10_000)]
    max_queue_files: usize,
}

#[derive(Debug, Default)]
struct Totals {
    frames_seen: usize,
    active_frames: usize,
    emitted_files: usize,
    duplicate_frames: usize,
    skipped_empty: usize,
    read_errors: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate_args(&args)?;
    fs::create_dir_all(&args.queue)
        .with_context(|| format!("failed to create queue {}", args.queue.display()))?;

    let mut seen_hashes = HashSet::new();
    let mut totals = Totals::default();
    let mut emitted_since_throttle = 0usize;

    for input in &args.inputs {
        println!("Processing {}", input.display());
        let mut mem = Memvid::open_read_only(input)
            .with_context(|| format!("failed to open {}", input.display()))?;
        let frame_count = mem.frame_count();
        totals.frames_seen += frame_count;

        for frame_id in 0..frame_count {
            let frame = match mem.frame_by_id(frame_id as u64) {
                Ok(frame) => frame,
                Err(err) => {
                    totals.read_errors += 1;
                    eprintln!("Skipping frame {frame_id} in {}: {err}", input.display());
                    continue;
                }
            };

            if frame.status != FrameStatus::Active {
                continue;
            }
            totals.active_frames += 1;

            let text = match mem.frame_text_by_id(frame_id as u64) {
                Ok(text) => text,
                Err(_) => match mem.frame_canonical_payload(frame_id as u64) {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                    Err(err) => {
                        totals.read_errors += 1;
                        eprintln!(
                            "Skipping unreadable frame {frame_id} in {}: {err}",
                            input.display()
                        );
                        continue;
                    }
                },
            };
            let text = text.trim();
            if text.is_empty() {
                totals.skipped_empty += 1;
                continue;
            }

            let hash = sha256_hex(text.as_bytes());
            if !seen_hashes.insert(hash) {
                totals.duplicate_frames += 1;
                continue;
            }

            wait_for_queue_capacity(&args.queue, args.max_queue_files)?;
            let written = write_queue_records(&args, input, frame_id as u64, text)?;
            totals.emitted_files += written;
            emitted_since_throttle += written;

            if emitted_since_throttle >= args.batch_size {
                thread::sleep(Duration::from_millis(args.throttle_ms));
                emitted_since_throttle = 0;
            }
        }
    }

    println!(
        "Done. frames_seen={} active_frames={} emitted_files={} duplicates={} empty={} read_errors={}",
        totals.frames_seen,
        totals.active_frames,
        totals.emitted_files,
        totals.duplicate_frames,
        totals.skipped_empty,
        totals.read_errors
    );
    Ok(())
}

fn validate_args(args: &Args) -> Result<()> {
    anyhow::ensure!(
        args.max_file_bytes > HEADER_RESERVE_BYTES + 1024,
        "--max-file-bytes must leave room for metadata and content"
    );
    anyhow::ensure!(
        args.batch_size > 0,
        "--batch-size must be greater than zero"
    );
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn wait_for_queue_capacity(queue: &Path, max_queue_files: usize) -> Result<()> {
    while queue_depth(queue)? > max_queue_files {
        eprintln!("Queue depth above {max_queue_files}; throttling migration.");
        thread::sleep(Duration::from_secs(5));
    }
    Ok(())
}

fn queue_depth(queue: &Path) -> Result<usize> {
    let mut count = 0usize;
    for entry in fs::read_dir(queue)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && !entry.file_name().to_string_lossy().starts_with(".tmp.")
        {
            count += 1;
        }
    }
    Ok(count)
}

fn write_queue_records(args: &Args, source: &Path, frame_id: u64, content: &str) -> Result<usize> {
    let max_content_bytes = args.max_file_bytes.saturating_sub(HEADER_RESERVE_BYTES);
    let chunks = split_utf8(content, max_content_bytes);
    let total_parts = chunks.len();

    for (index, chunk) in chunks.into_iter().enumerate() {
        write_queue_file(
            &args.queue,
            &args.project,
            source,
            frame_id,
            index + 1,
            total_parts,
            chunk,
        )?;
    }

    Ok(total_parts)
}

fn split_utf8(content: &str, max_bytes: usize) -> Vec<&str> {
    if content.len() <= max_bytes {
        return vec![content];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < content.len() {
        let mut end = (start + max_bytes).min(content.len());
        while end > start && !content.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = content[start..]
                .char_indices()
                .nth(1)
                .map_or(content.len(), |(idx, _)| start + idx);
        }
        chunks.push(content[start..end].trim());
        start = end;
    }
    chunks
        .into_iter()
        .filter(|chunk| !chunk.is_empty())
        .collect()
}

fn write_queue_file(
    queue_dir: &Path,
    project: &str,
    source: &Path,
    frame_id: u64,
    part: usize,
    total_parts: usize,
    content: &str,
) -> Result<()> {
    let timestamp = unix_ns();
    let tmp_path = queue_dir.join(format!(".tmp.{}", Uuid::new_v4()));
    let final_path = queue_dir.join(format!("{}_import_{}.md", timestamp, Uuid::new_v4()));
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.mv2");

    let mut file = File::create(&tmp_path)
        .with_context(|| format!("failed to create temp file {}", tmp_path.display()))?;

    writeln!(file, "[agent:legacy]")?;
    writeln!(file, "[status:migrated]")?;
    writeln!(file, "[type:import]")?;
    writeln!(file, "[project:{project}]")?;
    writeln!(file, "[timestamp:{timestamp}]")?;
    writeln!(file)?;
    writeln!(file, "## Legacy Import")?;
    writeln!(file)?;
    writeln!(file, "- source: `{}`", source.display())?;
    writeln!(file, "- frame_id: `{frame_id}`")?;
    writeln!(file, "- part: `{part}/{total_parts}`")?;
    writeln!(file, "- source_file: `{source_name}`")?;
    writeln!(file)?;
    writeln!(file, "## Content")?;
    writeln!(file)?;
    writeln!(file, "{content}")?;
    file.flush()?;
    file.sync_all()?;

    fs::rename(&tmp_path, &final_path).with_context(|| {
        format!(
            "failed to atomically move {} to {}",
            tmp_path.display(),
            final_path.display()
        )
    })?;
    Ok(())
}

fn unix_ns() -> i128 {
    let now = Utc::now();
    i128::from(now.timestamp()) * 1_000_000_000 + i128::from(now.timestamp_subsec_nanos())
}
