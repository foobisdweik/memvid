use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_SETTINGS_PATH: &str = "/etc/memvid/settings.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub paths: Paths,
    pub embedding: Embedding,
    pub ingestion: Ingestion,
    #[serde(default)]
    pub librarian: Option<Librarian>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub queue: String,
    #[serde(default = "default_librarian_queue")]
    pub librarian_queue: String,
    pub processing: String,
    pub ingest: String,
    pub done: String,
    pub failed: String,
    pub store: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Embedding {
    pub model_path: String,
    pub tokenizer_path: String,
    pub batch_size: usize,
    pub max_length: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ingestion {
    pub commit_interval: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Librarian {
    #[serde(default)]
    pub enabled: bool,
    pub endpoint: String,
    pub model: String,
    #[serde(default = "default_librarian_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_librarian_max_candidates")]
    pub max_candidates: usize,
    #[serde(default = "default_librarian_max_selected")]
    pub max_selected: usize,
    #[serde(default = "default_librarian_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_librarian_temperature")]
    pub temperature: f32,
    #[serde(default = "default_librarian_top_p")]
    pub top_p: f32,
    #[serde(default = "default_librarian_presence_penalty")]
    pub presence_penalty: f32,
    #[serde(default)]
    pub keep_alive: Option<String>,
}

fn default_librarian_queue() -> String {
    "/var/lib/memvid/librarian_queue".to_string()
}

fn default_librarian_timeout_ms() -> u64 {
    20_000
}

fn default_librarian_max_candidates() -> usize {
    12
}

fn default_librarian_max_selected() -> usize {
    6
}

fn default_librarian_max_tokens() -> usize {
    512
}

fn default_librarian_temperature() -> f32 {
    0.0
}

fn default_librarian_top_p() -> f32 {
    1.0
}

fn default_librarian_presence_penalty() -> f32 {
    1.5
}

pub fn load_settings(path: &str) -> Result<Settings> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

#[must_use]
pub fn settings_path_from_env() -> String {
    std::env::var("MEMVID_CONFIG").unwrap_or_else(|_| DEFAULT_SETTINGS_PATH.to_string())
}

#[derive(Clone, Debug)]
pub struct Job {
    pub path: PathBuf,
}

pub fn move_to_processing(job: &Job, processing_dir: &str) -> Result<Job> {
    let filename = job
        .path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(processing_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_ingest(job: &Job, ingest_dir: &str) -> Result<Job> {
    let filename = job
        .path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(ingest_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_done(job: &Job, done_dir: &str) -> Result<Job> {
    let filename = job
        .path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(done_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_failed(job: &Job, failed_dir: &str) -> Result<Job> {
    let filename = job
        .path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(failed_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn ensure_directories(settings: &Settings) -> Result<()> {
    fs::create_dir_all(&settings.paths.queue)?;
    fs::create_dir_all(&settings.paths.librarian_queue)?;
    fs::create_dir_all(&settings.paths.processing)?;
    fs::create_dir_all(&settings.paths.ingest)?;
    fs::create_dir_all(&settings.paths.done)?;
    fs::create_dir_all(&settings.paths.failed)?;
    fs::create_dir_all(&settings.paths.store)?;

    if let Some(store_parent) = Path::new(&settings.paths.store).parent() {
        fs::create_dir_all(store_parent)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_settings(extra_paths: &str, extra_librarian: &str) -> String {
        format!(
            r#"
[paths]
queue = "/tmp/memvid/queue"
processing = "/tmp/memvid/processing"
ingest = "/tmp/memvid/ingest"
done = "/tmp/memvid/done"
failed = "/tmp/memvid/failed"
store = "/tmp/memvid/store"
{extra_paths}

[embedding]
model_path = "/opt/models/model.onnx"
tokenizer_path = "/opt/models/tokenizer.json"
batch_size = 32
max_length = 512

[ingestion]
commit_interval = 32

[librarian]
enabled = true
endpoint = "http://127.0.0.1:11434/v1/chat/completions"
model = "qwen3:8b"
{extra_librarian}
"#
        )
    }

    #[test]
    fn settings_default_librarian_queue_when_missing() {
        let settings: Settings = toml::from_str(&minimal_settings("", "")).unwrap();

        assert_eq!(
            settings.paths.librarian_queue,
            "/var/lib/memvid/librarian_queue"
        );
    }

    #[test]
    fn settings_parse_librarian_queue_and_keep_alive() {
        let settings: Settings = toml::from_str(&minimal_settings(
            r#"librarian_queue = "/tmp/memvid/librarian_queue""#,
            r#"keep_alive = "-1""#,
        ))
        .unwrap();

        assert_eq!(
            settings.paths.librarian_queue,
            "/tmp/memvid/librarian_queue"
        );
        assert_eq!(
            settings.librarian.unwrap().keep_alive.as_deref(),
            Some("-1")
        );
    }
}
