use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub paths: Paths,
    pub embedding: Embedding,
    pub ingestion: Ingestion,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub queue: String,
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

pub fn load_settings(path: &str) -> Result<Settings> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

#[derive(Clone, Debug)]
pub struct Job {
    pub path: PathBuf,
}

pub fn move_to_processing(job: &Job, processing_dir: &str) -> Result<Job> {
    let filename = job.path.file_name().ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(processing_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_ingest(job: &Job, ingest_dir: &str) -> Result<Job> {
    let filename = job.path.file_name().ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(ingest_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_done(job: &Job, done_dir: &str) -> Result<Job> {
    let filename = job.path.file_name().ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(done_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn move_to_failed(job: &Job, failed_dir: &str) -> Result<Job> {
    let filename = job.path.file_name().ok_or_else(|| anyhow::anyhow!("Job path has no filename"))?;
    let new_path = PathBuf::from(failed_dir).join(filename);
    fs::rename(&job.path, &new_path)?;
    Ok(Job { path: new_path })
}

pub fn ensure_directories(settings: &Settings) -> Result<()> {
    fs::create_dir_all(&settings.paths.queue)?;
    fs::create_dir_all(&settings.paths.processing)?;
    fs::create_dir_all(&settings.paths.ingest)?;
    fs::create_dir_all(&settings.paths.done)?;
    fs::create_dir_all(&settings.paths.failed)?;
    
    if let Some(store_parent) = PathBuf::from(&settings.paths.store).parent() {
        fs::create_dir_all(store_parent)?;
    }
    
    Ok(())
}
