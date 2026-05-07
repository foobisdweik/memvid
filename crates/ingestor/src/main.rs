use anyhow::{Context, Result, anyhow};
use memvid_common::*;
use memvid_core::{Memvid, PutOptions};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

struct ShardStore {
    memvid: Memvid,
    counter: usize,
}

struct ProjectShardStores {
    base_dir: PathBuf,
    shards: BTreeMap<String, ShardStore>,
    commit_interval: usize,
}

impl ProjectShardStores {
    fn new(base_dir: PathBuf, commit_interval: usize) -> Result<Self> {
        Ok(Self {
            base_dir,
            shards: BTreeMap::new(),
            commit_interval,
        })
    }

    fn open_or_create(path: &Path) -> Result<Memvid> {
        if path.exists() {
            println!("Opening existing memory shard: {}", path.display());
            Ok(Memvid::open(path)?)
        } else {
            println!("Creating memory shard: {}", path.display());
            let mut mem = Memvid::create(path)?;
            mem.enable_vec()?;
            mem.enable_lex()?;
            mem.commit()?;
            Ok(mem)
        }
    }

    fn shard_mut(&mut self, shard: &str) -> Result<&mut ShardStore> {
        if !self.shards.contains_key(shard) {
            let path = self.base_dir.join(format!("{shard}.mv2"));
            let memvid = Self::open_or_create(&path)?;
            self.shards
                .insert(shard.to_string(), ShardStore { memvid, counter: 0 });
        }
        self.shards
            .get_mut(shard)
            .ok_or_else(|| anyhow!("shard store disappeared for {shard}"))
    }

    fn ingest(&mut self, data: &[u8], embedding: Vec<f32>, source_name: &str) -> Result<String> {
        let text = std::str::from_utf8(data).context("queue record is not valid UTF-8 Markdown")?;
        let project = extract_project_header(text)
            .ok_or_else(|| anyhow!("queue record is missing a [project:...] header"))?;
        let shard = shard_name_for_project(project)?;
        let source_uri = format!("mv2://ingest/{shard}/{source_name}");
        let opts = PutOptions::builder().uri(&source_uri).build();

        let commit_interval = self.commit_interval;
        let store = self.shard_mut(&shard)?;
        store
            .memvid
            .put_with_embedding_and_options(data, embedding, opts)?;
        store.counter += 1;

        if store.counter >= commit_interval {
            store.memvid.commit()?;
            store.counter = 0;
            println!("Committed shard batch: {shard}");
        }

        Ok(shard)
    }

    fn commit_pending(&mut self) -> Result<()> {
        for (shard, store) in &mut self.shards {
            if store.counter > 0 {
                store.memvid.commit()?;
                store.counter = 0;
                println!("Committed idle shard batch: {shard}");
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let settings_path = settings_path_from_env();
    println!("Loading settings from {settings_path}...");
    let settings = load_settings(&settings_path)?;
    ensure_directories(&settings)?;

    let mut stores = ProjectShardStores::new(
        PathBuf::from(&settings.paths.store),
        settings.ingestion.commit_interval,
    )?;

    println!("Ingestor listening on {}...", settings.paths.ingest);
    let mut last_processed = Instant::now();

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
            if path.extension().and_then(|s| s.to_str()) != Some("emb") {
                continue;
            }

            let emb_path = path.to_path_buf();
            let Some(text_path) = text_path_for_embedding(&emb_path) else {
                continue;
            };

            if !text_path.exists() {
                continue;
            }

            let emb_bytes = fs::read(&emb_path).context("Failed to read embedding file")?;
            let embedding: Vec<f32> =
                bincode::deserialize(&emb_bytes).context("Failed to deserialize embedding")?;

            let data = fs::read(&text_path).context("Failed to read text file")?;
            let source_name = text_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown.md")
                .to_string();

            match stores.ingest(&data, embedding, &source_name) {
                Ok(shard) => {
                    let job = Job {
                        path: text_path.clone(),
                    };
                    move_to_done(&job, &settings.paths.done).ok();
                    fs::remove_file(&emb_path).ok();
                    processed_any = true;
                    last_processed = Instant::now();
                    println!("Ingested {} into shard {}", source_name, shard);
                }
                Err(e) => {
                    println!("Failed to ingest {}: {}", source_name, e);
                    let job = Job {
                        path: text_path.clone(),
                    };
                    move_to_failed(&job, &settings.paths.failed).ok();
                    fs::remove_file(&emb_path).ok();
                }
            }
        }

        if !processed_any {
            if last_processed.elapsed() >= Duration::from_secs(2) {
                if let Err(err) = stores.commit_pending() {
                    eprintln!("Failed to commit idle batch: {err}");
                }
                last_processed = Instant::now();
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

fn text_path_for_embedding(emb_path: &Path) -> Option<PathBuf> {
    let filename = emb_path.file_name()?.to_str()?;
    let text_filename = filename.strip_suffix(".emb")?;
    Some(emb_path.with_file_name(text_filename))
}

fn extract_project_header(text: &str) -> Option<&str> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(inner) = trimmed
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        else {
            break;
        };
        let Some((key, value)) = inner.split_once(':') else {
            break;
        };
        if key.trim().eq_ignore_ascii_case("project") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value);
            }
            return None;
        }
    }
    None
}

fn shard_name_for_project(project: &str) -> Result<String> {
    let project = project.trim();
    if project.is_empty() {
        return Err(anyhow!("project header is empty"));
    }
    if project.eq_ignore_ascii_case("global") {
        return Ok("global".to_string());
    }

    let mut shard = String::new();
    let mut last_dash = false;
    for ch in project.chars() {
        if ch.is_ascii_alphanumeric() {
            shard.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            shard.push('-');
            last_dash = true;
        }
    }

    let shard = shard.trim_matches('-').to_string();
    if shard.is_empty() {
        return Err(anyhow!(
            "project header did not contain any usable filename characters"
        ));
    }

    Ok(shard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_project_header_from_queue_record() {
        let text = "[agent:codex]\n[status:done]\n[project:memvid]\n\nbody";
        assert_eq!(extract_project_header(text), Some("memvid"));
    }

    #[test]
    fn ignores_project_mentions_outside_header() {
        let text = "memvid appears in the body only";
        assert_eq!(extract_project_header(text), None);
    }

    #[test]
    fn shard_name_is_stable_and_sanitized() {
        assert_eq!(shard_name_for_project("Memvid").unwrap(), "memvid");
        assert_eq!(
            shard_name_for_project("Customer Support/API").unwrap(),
            "customer-support-api"
        );
        assert_eq!(shard_name_for_project("global").unwrap(), "global");
    }

    #[test]
    fn invalid_project_names_fail() {
        assert!(shard_name_for_project("   ").is_err());
        assert!(shard_name_for_project("///").is_err());
    }

    #[test]
    fn ingests_into_project_and_global_shards() {
        let dir = tempdir().unwrap();
        let mut stores = ProjectShardStores::new(dir.path().to_path_buf(), 1).unwrap();

        let project_record =
            b"[agent:codex]\n[status:done]\n[type:update]\n[project:memvid]\n\nproject body";
        let global_record =
            b"[agent:codex]\n[status:done]\n[type:update]\n[project:global]\n\nglobal body";

        stores
            .ingest(project_record, vec![0.1, 0.2], "project.md")
            .unwrap();
        stores
            .ingest(global_record, vec![0.3, 0.4], "global.md")
            .unwrap();
        stores.commit_pending().unwrap();

        let project_path = dir.path().join("memvid.mv2");
        let global_path = dir.path().join("global.mv2");

        assert!(project_path.exists());
        assert!(global_path.exists());
        assert_eq!(Memvid::open(&project_path).unwrap().frame_count(), 1);
        assert_eq!(Memvid::open(&global_path).unwrap().frame_count(), 1);
    }
}
