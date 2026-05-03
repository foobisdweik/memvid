use anyhow::{Context, Result};
use memvid_common::*;
use ndarray::Array2;
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use tokenizers::Tokenizer;
use walkdir::WalkDir;

struct CudaEmbedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl CudaEmbedder {
    pub fn new(model_path: &str, tokenizer_path: &str, max_length: usize) -> Result<Self> {
        println!("Loading tokenizer from {}...", tokenizer_path);
        let mut tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        // Ensure padding and truncation are set up for batching
        if let Some(params) = tokenizer.get_truncation_mut() {
            params.max_length = max_length;
        } else {
            let _ = tokenizer.with_truncation(Some(tokenizers::TruncationParams {
                max_length,
                ..Default::default()
            }));
        }

        if let Some(params) = tokenizer.get_padding_mut() {
            params.strategy = tokenizers::PaddingStrategy::BatchLongest;
        } else {
            let _ = tokenizer.with_padding(Some(tokenizers::PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            }));
        }

        println!("Loading ONNX model from {} with CUDA...", model_path);

        // Use CUDA Execution Provider. Note: This requires the system to have CUDA/cuDNN installed.
        // We use the ort v2 builder API.
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            // In ort 2.0, execution providers are configured via the builder
            // We append the CUDA provider if available.
            // Note: the exact API depends on the ort version, but try_with_execution_providers is standard for v2
            .with_execution_providers([
                ort::execution_providers::CUDAExecutionProvider::default().build()
            ])?
            .commit_from_file(model_path)?;

        println!(
            "Model inputs: {:?}",
            session
                .inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>()
        );
        println!(
            "Model outputs: {:?}",
            session
                .outputs
                .iter()
                .map(|output| output.name.as_str())
                .collect::<Vec<_>>()
        );

        Ok(Self { session, tokenizer })
    }

    pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let batch_size = texts.len();
        // Since we used BatchLongest padding, all encodings in the batch have the same length.
        let seq_len = encodings[0].get_ids().len();

        // Prepare inputs for ONNX. BGE/Nomic typically expect input_ids, attention_mask, and token_type_ids.
        let mut input_ids = Vec::with_capacity(batch_size * seq_len);
        let mut attention_mask = Vec::with_capacity(batch_size * seq_len);
        let mut token_type_ids = Vec::with_capacity(batch_size * seq_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            input_ids.extend_from_slice(ids);
            attention_mask.extend_from_slice(mask);
            token_type_ids.extend_from_slice(type_ids);
        }

        let input_ids_array = Array2::from_shape_vec(
            (batch_size, seq_len),
            input_ids.into_iter().map(|id| id as i64).collect(),
        )?;
        let attention_mask_array = Array2::from_shape_vec(
            (batch_size, seq_len),
            attention_mask.into_iter().map(|m| m as i64).collect(),
        )?;
        let token_type_ids_array = Array2::from_shape_vec(
            (batch_size, seq_len),
            token_type_ids.into_iter().map(|t| t as i64).collect(),
        )?;

        let input_ids_tensor = Tensor::from_array(input_ids_array)?;
        let attention_mask_tensor = Tensor::from_array(attention_mask_array)?;
        let token_type_ids_tensor = Tensor::from_array(token_type_ids_array)?;

        // Run ONNX inference
        let inputs = ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "token_type_ids" => token_type_ids_tensor,
        ];

        let outputs = self.session.run(inputs)?;

        // Extract the last_hidden_state or sentence_embedding
        // For BGE/Nomic, we typically need the first token (CLS) embedding or mean pooling.
        // Nomic uses mean pooling over the last hidden state, or some models output 'sentence_embedding' directly.
        // Let's assume the model outputs 'sentence_embedding' directly for simplicity, or we mean pool 'last_hidden_state'.

        // Try extracting 'sentence_embedding' first (common for exported sentence-transformers)
        let embeddings: Vec<Vec<f32>> = if let Some(val) = outputs.get("sentence_embedding") {
            let (shape, tensor_data) = val.try_extract_tensor::<f32>()?;
            let dim = shape[1] as usize;

            let mut result = Vec::with_capacity(batch_size);
            for i in 0..batch_size {
                let start = i * dim;
                let end = start + dim;
                result.push(tensor_data[start..end].to_vec());
            }
            result
        } else {
            // Nomic exports `last_hidden_state`; mean-pool with attention mask and normalize.
            let val = outputs
                .get("last_hidden_state")
                .context("Model must output either sentence_embedding or last_hidden_state")?;
            let (shape, tensor_data) = val.try_extract_tensor::<f32>()?;
            let seq = shape[1] as usize;
            let dim = shape[2] as usize;

            let mut result = Vec::with_capacity(batch_size);
            for i in 0..batch_size {
                let mut pooled = vec![0.0f32; dim];
                let mut mask_sum = 0.0f32;
                for token_index in 0..seq {
                    let mask = encodings[i].get_attention_mask()[token_index] as f32;
                    if mask <= 0.0 {
                        continue;
                    }
                    mask_sum += mask;
                    let token_start = (i * seq + token_index) * dim;
                    let token_end = token_start + dim;
                    for (dst, src) in pooled.iter_mut().zip(&tensor_data[token_start..token_end]) {
                        *dst += src * mask;
                    }
                }
                if mask_sum > 0.0 {
                    for value in &mut pooled {
                        *value /= mask_sum;
                    }
                }
                let mut norm: f32 = 0.0;
                for &val in &pooled {
                    norm += val * val;
                }
                norm = norm.sqrt();

                let normalized = if norm > 0.0 {
                    pooled.iter().map(|&val| val / norm).collect()
                } else {
                    pooled
                };
                result.push(normalized);
            }
            result
        };

        Ok(embeddings)
    }
}

fn main() -> Result<()> {
    let settings_path = settings_path_from_env();
    println!("Loading settings from {settings_path}...");
    let settings = load_settings(&settings_path)?;
    ensure_directories(&settings)?;

    // Check if models exist, otherwise error out cleanly.
    if !PathBuf::from(&settings.embedding.model_path).exists() {
        println!(
            "Warning: ONNX model not found at {}. CUDA embeddings will fail to initialize.",
            settings.embedding.model_path
        );
    }
    if !PathBuf::from(&settings.embedding.tokenizer_path).exists() {
        println!(
            "Warning: Tokenizer not found at {}. CUDA embeddings will fail to initialize.",
            settings.embedding.tokenizer_path
        );
    }

    let mut embedder = CudaEmbedder::new(
        &settings.embedding.model_path,
        &settings.embedding.tokenizer_path,
        settings.embedding.max_length,
    )?;

    let (tx, rx) = crossbeam_channel::unbounded();

    // Spawn ingest forwarder thread
    let ingest_dir = settings.paths.ingest.clone();
    thread::spawn(move || {
        while let Ok((job, embedding)) = rx.recv() {
            let job: Job = job;
            let embedding: Vec<f32> = embedding;
            // Write embedding to a sidecar file or JSON
            let Some(filename) = job.path.file_name() else {
                eprintln!("Skipping job with no filename: {}", job.path.display());
                continue;
            };
            let emb_filename = format!("{}.emb", filename.to_string_lossy());
            let emb_path = job.path.with_file_name(&emb_filename);

            // Serialize embedding
            let Ok(emb_bytes) = bincode::serialize(&embedding) else {
                eprintln!("Failed to serialize embedding for {}", job.path.display());
                continue;
            };
            if let Err(err) = fs::write(&emb_path, emb_bytes) {
                eprintln!("Failed to write embedding {}: {err}", emb_path.display());
                continue;
            }

            // Move both to ingest directory
            let new_text_path = PathBuf::from(&ingest_dir).join(filename);
            let new_emb_path = PathBuf::from(&ingest_dir).join(&emb_filename);

            if let Err(err) = fs::rename(&job.path, &new_text_path) {
                eprintln!("Failed to move text to ingest: {err}");
                continue;
            }
            if let Err(err) = fs::rename(&emb_path, &new_emb_path) {
                eprintln!("Failed to move embedding to ingest: {err}");
                continue;
            }

            println!("Embedded and ready for ingest: {}", new_text_path.display());
        }
    });

    println!("Embedder listening on {}...", settings.paths.queue);

    loop {
        let mut jobs = Vec::new();
        for entry in WalkDir::new(&settings.paths.queue)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if is_queue_markdown(path, entry.file_type().is_file()) {
                jobs.push(Job {
                    path: path.to_path_buf(),
                });
            }
            if jobs.len() >= settings.embedding.batch_size {
                break;
            }
        }

        if jobs.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }

        let mut texts = Vec::new();
        for job in &jobs {
            match move_to_processing(job, &settings.paths.processing) {
                Ok(moved) => match fs::read_to_string(&moved.path) {
                    Ok(text) => texts.push((moved, text)),
                    Err(err) => {
                        eprintln!("Failed to read {}: {err}", moved.path.display());
                        let _ = move_to_failed(&moved, &settings.paths.failed);
                    }
                },
                Err(e) => {
                    println!("Failed to move job to processing: {}", e);
                }
            }
        }

        if texts.is_empty() {
            continue;
        }

        let raw_texts: Vec<&str> = texts.iter().map(|(_, t)| t.as_str()).collect();

        match embedder.embed_batch(&raw_texts) {
            Ok(embeddings) => {
                for ((job, _), embedding) in texts.into_iter().zip(embeddings) {
                    tx.send((job, embedding))?;
                }
            }
            Err(e) => {
                println!("CUDA embedding batch failed: {}", e);
                // Move jobs to failed state
                for (job, _) in texts {
                    move_to_failed(&job, &settings.paths.failed).ok();
                }
            }
        }
    }
}

fn is_queue_markdown(path: &Path, is_file: bool) -> bool {
    if !is_file {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !name.starts_with(".tmp.") && path.extension().and_then(|ext| ext.to_str()) == Some("md")
}
