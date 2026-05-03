use anyhow::{Context, Result};
use memvid_common::*;
use ndarray::Array2;
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;
use std::fs;
use std::path::PathBuf;
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
            // Fallback to CLS token from 'last_hidden_state'
            let val = outputs
                .get("last_hidden_state")
                .context("Model must output either sentence_embedding or last_hidden_state")?;
            let (shape, tensor_data) = val.try_extract_tensor::<f32>()?;
            let seq = shape[1] as usize;
            let dim = shape[2] as usize;

            let mut result = Vec::with_capacity(batch_size);
            for i in 0..batch_size {
                // CLS token is at index 0 of the sequence
                let start = i * seq * dim;
                let end = start + dim;

                // L2 normalization for BGE/Nomic
                let cls_embedding = &tensor_data[start..end];
                let mut norm: f32 = 0.0;
                for &val in cls_embedding {
                    norm += val * val;
                }
                norm = norm.sqrt();

                let normalized: Vec<f32> = cls_embedding.iter().map(|&val| val / norm).collect();
                result.push(normalized);
            }
            result
        };

        Ok(embeddings)
    }
}

fn main() -> Result<()> {
    let settings = load_settings("config/settings.toml")?;
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
            let filename = job.path.file_name().unwrap();
            let mut emb_path = job.path.clone();
            emb_path.set_extension("emb");

            // Serialize embedding
            let emb_bytes = bincode::serialize(&embedding).expect("Failed to serialize embedding");
            fs::write(&emb_path, emb_bytes).expect("Failed to write embedding");

            // Move both to ingest directory
            let new_text_path = PathBuf::from(&ingest_dir).join(filename);
            let new_emb_path = PathBuf::from(&ingest_dir).join(emb_path.file_name().unwrap());

            fs::rename(&job.path, &new_text_path).expect("Failed to move text to ingest");
            fs::rename(&emb_path, &new_emb_path).expect("Failed to move emb to ingest");

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
            if entry.file_type().is_file() {
                jobs.push(Job {
                    path: entry.path().to_path_buf(),
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
                Ok(moved) => {
                    if let Ok(text) = fs::read_to_string(&moved.path) {
                        texts.push((moved, text));
                    }
                }
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
