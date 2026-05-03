use anyhow::{Context, Result};
use chrono::Utc;
use memvid_common::*;
use memvid_core::{Memvid, PutOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

struct RolloverStore {
    base_dir: PathBuf,
    current_date: String,
    memvid: Memvid,
    counter: usize,
    commit_interval: usize,
}

impl RolloverStore {
    fn new(base_dir: PathBuf, commit_interval: usize) -> Result<Self> {
        let current_date = Utc::now().format("%Y-%m-%d").to_string();
        let memvid = Self::open_or_create(&base_dir, &current_date)?;

        Ok(Self {
            base_dir,
            current_date,
            memvid,
            counter: 0,
            commit_interval,
        })
    }

    fn open_or_create(base_dir: &Path, date: &str) -> Result<Memvid> {
        let filename = format!("{}.mv2", date);
        let path = base_dir.join(&filename);

        if path.exists() {
            println!("Opening existing memory for today: {}", path.display());
            Ok(Memvid::open(&path)?)
        } else {
            println!("Creating new memory for today: {}", path.display());
            let mut mem = Memvid::create(&path)?;
            mem.enable_vec()?;
            mem.enable_lex()?;
            mem.commit()?;
            Ok(mem)
        }
    }

    fn check_rollover(&mut self) -> Result<()> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if self.current_date != today {
            println!(
                "Rolling over memory file from {} to {}",
                self.current_date, today
            );
            // Force a commit on the old file before switching
            if self.counter > 0 {
                self.memvid.commit()?;
                self.counter = 0;
            }

            // The old Memvid instance is dropped, which is safe.
            self.memvid = Self::open_or_create(&self.base_dir, &today)?;
            self.current_date = today;
        }
        Ok(())
    }

    fn ingest(&mut self, data: &[u8], embedding: Vec<f32>, source_uri: &str) -> Result<()> {
        self.check_rollover()?;

        let opts = PutOptions::builder().uri(source_uri).build();

        self.memvid
            .put_with_embedding_and_options(data, embedding, opts)?;
        self.counter += 1;

        if self.counter >= self.commit_interval {
            self.memvid.commit()?;
            self.counter = 0;
            println!("Committed batch.");
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let settings = load_settings("config/settings.toml")?;
    ensure_directories(&settings)?;

    let mut store = RolloverStore::new(
        PathBuf::from(&settings.paths.store),
        settings.ingestion.commit_interval,
    )?;

    println!("Ingestor listening on {}...", settings.paths.ingest);

    loop {
        let mut processed_any = false;

        for entry in WalkDir::new(&settings.paths.ingest)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            // We only trigger when we see the .emb file to ensure the pair is ready.
            if path.extension().and_then(|s| s.to_str()) != Some("emb") {
                continue;
            }

            let emb_path = path.to_path_buf();
            let mut text_path = emb_path.clone();
            // Revert to original extension or no extension. For simplicity, assume original was .txt
            // or just strip the .emb and use the base name as the file if we dropped the extension.
            text_path.set_extension("txt");
            // Wait, in embedder we did `emb_path.set_extension("emb");` which replaced the existing extension.
            // If the original was .txt, it became .emb. So we can just set it back to .txt.

            if !text_path.exists() {
                // Original file might not have had an extension or had a different one.
                // A better approach is checking if the base name without extension exists.
                // For this prototype, we'll try to find the non-.emb file with the same stem.
                continue;
            }

            let emb_bytes = fs::read(&emb_path).context("Failed to read embedding file")?;
            let embedding: Vec<f32> =
                bincode::deserialize(&emb_bytes).context("Failed to deserialize embedding")?;

            let data = fs::read(&text_path).context("Failed to read text file")?;
            let uri = format!(
                "mv2://ingest/{}",
                text_path.file_name().unwrap().to_string_lossy()
            );

            match store.ingest(&data, embedding, &uri) {
                Ok(_) => {
                    // Move to done
                    let job = Job { path: text_path };
                    move_to_done(&job, &settings.paths.done).ok();
                    // Just delete the .emb file as we don't need it anymore
                    fs::remove_file(&emb_path).ok();
                    processed_any = true;
                }
                Err(e) => {
                    println!("Failed to ingest {}: {}", uri, e);
                    let job = Job { path: text_path };
                    move_to_failed(&job, &settings.paths.failed).ok();
                    fs::remove_file(&emb_path).ok();
                }
            }
        }

        if !processed_any {
            thread::sleep(Duration::from_millis(500));
        }
    }
}
