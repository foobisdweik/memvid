use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use clap::Parser;
use memvid_common::{load_settings, settings_path_from_env, Librarian};
use memvid_core::{FrameStatus, Memvid};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

const DEFAULT_BUDGET_TOKENS: usize = 4_000;
const TOKEN_TO_CHAR_RATIO: usize = 4;
const DEFAULT_MAX_STORE_DAYS: i64 = 30;
const DEFAULT_MAX_RECORDS: usize = 48;
const DEFAULT_COMPRESSION_HORIZON_HOURS: f64 = 168.0;
const MAX_BODY_READ_BYTES: usize = 256 * 1024;
const RECENCY_CLIFF_HOURS: f64 = 16.0;
const RECENT_DETAIL_HOURS: f64 = 4.0;
const MAX_OLDER_RECORDS: usize = 12;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Emit bounded startup context from read-only Memvid stores"
)]
struct Args {
    /// Memvid settings file. Defaults to MEMVID_CONFIG or /etc/memvid/settings.toml.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Override the store directory from settings.
    #[arg(long)]
    store: Option<PathBuf>,

    /// Project name to prioritize. Defaults to the current directory name.
    #[arg(long)]
    project: Option<String>,

    /// Agent name for packet metadata.
    #[arg(long, default_value = "agent")]
    agent: String,

    /// Working directory used for project inference and context metadata.
    #[arg(long)]
    cwd: Option<PathBuf>,

    /// Extra semantic query terms to prioritize. Repeatable.
    #[arg(long = "query")]
    queries: Vec<String>,

    /// Approximate output budget in tokens.
    #[arg(long, default_value_t = DEFAULT_BUDGET_TOKENS)]
    budget_tokens: usize,

    /// Maximum store age to scan.
    #[arg(long, default_value_t = DEFAULT_MAX_STORE_DAYS)]
    max_store_days: i64,

    /// Maximum candidate records to render before final budget trimming.
    #[arg(long, default_value_t = DEFAULT_MAX_RECORDS)]
    max_records: usize,

    /// Age where ordinary records reach maximum startup compression.
    #[arg(long, default_value_t = DEFAULT_COMPRESSION_HORIZON_HOURS)]
    compression_horizon_hours: f64,

    /// Include skipped store errors in the emitted packet.
    #[arg(long)]
    include_store_errors: bool,

    /// Allow records from projects other than the current project and global.
    #[arg(long)]
    include_other_projects: bool,

    /// Enable librarian reranking for startup recall.
    #[arg(long)]
    librarian: bool,

    /// Disable librarian reranking even when config enables it.
    #[arg(long)]
    no_librarian: bool,

    /// Override the librarian endpoint.
    #[arg(long)]
    librarian_endpoint: Option<String>,

    /// Override the librarian model.
    #[arg(long)]
    librarian_model: Option<String>,

    /// Override librarian timeout in milliseconds.
    #[arg(long)]
    librarian_timeout_ms: Option<u64>,

    /// Override the number of candidates sent to librarian.
    #[arg(long)]
    librarian_max_candidates: Option<usize>,

    /// Override maximum records the librarian may select.
    #[arg(long)]
    librarian_max_selected: Option<usize>,
}

#[derive(Debug, Clone)]
struct StorePath {
    path: PathBuf,
    date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default)]
struct Header {
    values: BTreeMap<String, String>,
}

impl Header {
    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    store: PathBuf,
    store_date: Option<NaiveDate>,
    frame_id: u64,
    frame_ts: i64,
    header: Header,
    body: String,
    age_hours: f64,
    score: f64,
}

#[derive(Debug, Clone)]
struct LibrarianRuntime {
    endpoint: String,
    model: String,
    timeout_ms: u64,
    max_candidates: usize,
    max_selected: usize,
    max_tokens: usize,
    temperature: f32,
    top_p: f32,
    presence_penalty: f32,
}

#[derive(Debug, Clone, Default)]
struct LibrarianResult {
    active: bool,
    candidate_count: usize,
    elapsed_ms: u64,
    brief: Option<String>,
    selected_ids: Vec<String>,
    warning: Option<String>,
}

impl LibrarianResult {
    fn status(&self) -> &'static str {
        if !self.active {
            "disabled"
        } else if self.warning.is_some() {
            "fallback"
        } else {
            "enabled"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectMatch {
    Exact,
    Embedded,
    Global,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Handoff,
    Fresh,
    Project,
    Risk,
    Older,
    Recall,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cwd = args
        .cwd
        .clone()
        .unwrap_or(std::env::current_dir().context("failed to read current directory")?);
    let project = args.project.clone().unwrap_or_else(|| infer_project(&cwd));
    let settings_path = args
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from(settings_path_from_env()));
    let settings = load_settings(&settings_path.to_string_lossy())
        .with_context(|| format!("failed to load {}", settings_path.display()))?;
    let store_dir = args
        .store
        .clone()
        .unwrap_or_else(|| PathBuf::from(settings.paths.store));
    let budget_chars = args.budget_tokens.saturating_mul(TOKEN_TO_CHAR_RATIO);

    let stores = discover_stores(&store_dir, args.max_store_days)?;
    let query_tokens = query_tokens(&project, &cwd, &args.queries);
    let mut errors = Vec::new();
    let mut candidates = collect_candidates(
        &stores,
        &project,
        &query_tokens,
        args.compression_horizon_hours,
        args.include_other_projects,
        &mut errors,
    );
    dedupe_candidates(&mut candidates);
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    candidates = trim_candidates_for_budget(candidates, args.max_records);
    let candidate_count = candidates.len();
    let librarian = librarian_runtime(&args, settings.librarian.as_ref());
    let mut librarian_result = if let Some(runtime) = librarian.as_ref() {
        let started = Instant::now();
        let mut result = rerank_with_librarian(runtime, &project, &cwd, &args.queries, &candidates);
        result.elapsed_ms = millis_u64(started.elapsed());
        result
    } else {
        LibrarianResult::default()
    };
    librarian_result.candidate_count = candidate_count;
    if librarian_result.active && !librarian_result.selected_ids.is_empty() {
        candidates = apply_librarian_selection(candidates, &librarian_result.selected_ids);
    }
    if let Some(warning) = librarian_result.warning.as_ref() {
        errors.push(format!("librarian: {warning}"));
    }

    let packet = render_packet(RenderInput {
        args: &args,
        cwd: &cwd,
        project: &project,
        store_dir: &store_dir,
        stores_searched: stores.len(),
        candidates: &candidates,
        errors: &errors,
        budget_chars,
        librarian: &librarian_result,
    });
    print!("{packet}");
    Ok(())
}

struct RenderInput<'a> {
    args: &'a Args,
    cwd: &'a Path,
    project: &'a str,
    store_dir: &'a Path,
    stores_searched: usize,
    candidates: &'a [Candidate],
    errors: &'a [String],
    budget_chars: usize,
    librarian: &'a LibrarianResult,
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    temperature: f32,
    top_p: f32,
    presence_penalty: f32,
    max_tokens: usize,
    stream: bool,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LibrarianReply {
    selected_ids: Vec<String>,
    session_brief: String,
    dropped_ids: Vec<DroppedRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DroppedRecord {
    id: String,
    reason: DropReason,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum DropReason {
    Duplicate,
    StaleSuperseded,
    ResolvedDone,
    WrongProject,
    GlobalNotNeeded,
    LowSignal,
    TooOld,
    Unknown,
}

fn discover_stores(store_dir: &Path, max_store_days: i64) -> Result<Vec<StorePath>> {
    let cutoff = Utc::now().date_naive() - chrono::Duration::days(max_store_days.max(0));
    let mut stores = Vec::new();
    for entry in fs::read_dir(store_dir)
        .with_context(|| format!("failed to read store dir {}", store_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("mv2") {
            continue;
        }
        let date = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok());
        if date.is_some_and(|date| date < cutoff) {
            continue;
        }
        stores.push(StorePath { path, date });
    }
    stores.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.path.cmp(&a.path)));
    Ok(stores)
}

fn collect_candidates(
    stores: &[StorePath],
    project: &str,
    query_tokens: &BTreeSet<String>,
    horizon_hours: f64,
    include_other_projects: bool,
    errors: &mut Vec<String>,
) -> Vec<Candidate> {
    let now = Utc::now();
    let mut candidates = Vec::new();
    for store in stores {
        let mut mem = match open_store(&store.path) {
            Ok(mem) => mem,
            Err(err) => {
                errors.push(format!("{}: {err}", store.path.display()));
                continue;
            }
        };
        let frame_count = mem.frame_count();
        for frame_id in (0..frame_count).rev() {
            let Ok(frame) = mem.frame_by_id(frame_id as u64) else {
                continue;
            };
            if frame.status != FrameStatus::Active {
                continue;
            }
            let text = read_frame_text(&mut mem, frame_id as u64);
            let text = trim_record_bytes(&text);
            if text.trim().is_empty() {
                continue;
            }

            let (header, body) = parse_header(text);
            let project_match = classify_project_match(&header, &body, project);
            if !include_other_projects && !is_project_visible(project_match) {
                continue;
            }
            let record_ts = header
                .get("timestamp")
                .and_then(parse_timestamp)
                .or_else(|| timestamp_seconds(frame.timestamp))
                .or_else(|| store.date.and_then(date_start));
            let age_hours = record_ts
                .map(|ts| (now - ts).num_minutes().max(0) as f64 / 60.0)
                .unwrap_or(horizon_hours);
            let score = score_candidate(&header, &body, project_match, query_tokens, age_hours);

            candidates.push(Candidate {
                store: store.path.clone(),
                store_date: store.date,
                frame_id: frame_id as u64,
                frame_ts: frame.timestamp,
                header,
                body,
                age_hours,
                score,
            });
        }
    }
    candidates
}

fn is_project_visible(project_match: ProjectMatch) -> bool {
    matches!(
        project_match,
        ProjectMatch::Exact | ProjectMatch::Embedded | ProjectMatch::Global
    )
}

fn librarian_runtime(args: &Args, configured: Option<&Librarian>) -> Option<LibrarianRuntime> {
    let enabled =
        !args.no_librarian && (args.librarian || configured.is_some_and(|cfg| cfg.enabled));
    if !enabled {
        return None;
    }

    let endpoint = args
        .librarian_endpoint
        .clone()
        .or_else(|| configured.map(|cfg| cfg.endpoint.clone()))
        .unwrap_or_else(|| "http://127.0.0.1:11434/v1/chat/completions".to_string());
    let model = args
        .librarian_model
        .clone()
        .or_else(|| configured.map(|cfg| cfg.model.clone()))
        .unwrap_or_else(|| "qwen3:8b".to_string());
    let timeout_ms = args
        .librarian_timeout_ms
        .or_else(|| configured.map(|cfg| cfg.timeout_ms))
        .unwrap_or(8_000);
    let max_candidates = args
        .librarian_max_candidates
        .or_else(|| configured.map(|cfg| cfg.max_candidates))
        .unwrap_or(12)
        .max(1);
    let max_selected = args
        .librarian_max_selected
        .or_else(|| configured.map(|cfg| cfg.max_selected))
        .unwrap_or(6)
        .max(1);
    let max_tokens = configured.map(|cfg| cfg.max_tokens).unwrap_or(256).max(64);
    let temperature = configured.map(|cfg| cfg.temperature).unwrap_or(0.0);
    let top_p = configured.map(|cfg| cfg.top_p).unwrap_or(1.0);
    let presence_penalty = configured.map(|cfg| cfg.presence_penalty).unwrap_or(1.5);

    Some(LibrarianRuntime {
        endpoint,
        model,
        timeout_ms,
        max_candidates,
        max_selected,
        max_tokens,
        temperature,
        top_p,
        presence_penalty,
    })
}

fn rerank_with_librarian(
    runtime: &LibrarianRuntime,
    project: &str,
    cwd: &Path,
    queries: &[String],
    candidates: &[Candidate],
) -> LibrarianResult {
    if candidates.is_empty() {
        return LibrarianResult {
            active: true,
            warning: Some("no candidates available for librarian".to_string()),
            ..Default::default()
        };
    }
    if candidates.len() == 1 {
        return LibrarianResult {
            active: true,
            selected_ids: vec![candidate_id(&candidates[0])],
            ..Default::default()
        };
    }

    let chosen: Vec<&Candidate> = candidates.iter().take(runtime.max_candidates).collect();
    let prompt = librarian_prompt(project, cwd, queries, &chosen);
    let client = match Client::builder()
        .timeout(Duration::from_millis(runtime.timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return LibrarianResult {
                active: true,
                warning: Some(format!("client init failed: {err}")),
                ..Default::default()
            };
        }
    };
    let req = OpenAiChatRequest {
        model: &runtime.model,
        messages: vec![
            OpenAiMessage {
                role: "system",
                content: "You are Memvid librarian for startup recall pruning. Return exactly one JSON object. No markdown. No prose before or after JSON. No extra keys.",
            },
            OpenAiMessage {
                role: "user",
                content: &prompt,
            },
        ],
        temperature: runtime.temperature,
        top_p: runtime.top_p,
        presence_penalty: runtime.presence_penalty,
        max_tokens: runtime.max_tokens,
        stream: false,
        response_format: ResponseFormat {
            kind: "json_object",
        },
    };
    let response = match client.post(&runtime.endpoint).json(&req).send() {
        Ok(response) => match response.error_for_status() {
            Ok(response) => response,
            Err(err) => {
                return LibrarianResult {
                    active: true,
                    warning: Some(format!("request failed: {err}")),
                    ..Default::default()
                };
            }
        },
        Err(err) => {
            return LibrarianResult {
                active: true,
                warning: Some(format!("request failed: {err}")),
                ..Default::default()
            };
        }
    };
    let parsed: OpenAiChatResponse = match response.json() {
        Ok(parsed) => parsed,
        Err(err) => {
            return LibrarianResult {
                active: true,
                warning: Some(format!("response parse failed: {err}")),
                ..Default::default()
            };
        }
    };
    let content = match parsed.choices.into_iter().next() {
        Some(choice) => choice.message.content,
        None => {
            return LibrarianResult {
                active: true,
                warning: Some("empty librarian response".to_string()),
                ..Default::default()
            };
        }
    };
    let reply = match parse_librarian_reply(&content) {
        Ok(reply) => reply,
        Err(err) => {
            return LibrarianResult {
                active: true,
                warning: Some(err),
                ..Default::default()
            };
        }
    };
    match validate_librarian_reply(reply, &chosen, runtime.max_selected) {
        Ok((selected_ids, brief)) => LibrarianResult {
            active: true,
            brief: non_empty(&brief).map(truncate_brief),
            selected_ids,
            warning: None,
            ..Default::default()
        },
        Err(err) => LibrarianResult {
            active: true,
            warning: Some(err),
            ..Default::default()
        },
    }
}

fn millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn librarian_prompt(
    project: &str,
    cwd: &Path,
    queries: &[String],
    candidates: &[&Candidate],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Project: ");
    prompt.push_str(project);
    prompt.push_str("\nCwd: ");
    prompt.push_str(&cwd.display().to_string());
    prompt.push_str("\nTask queries: ");
    if queries.is_empty() {
        prompt.push_str("current state, handoff, risk, next step");
    } else {
        prompt.push_str(&queries.join(" | "));
    }
    prompt.push_str(
        "\nMode: /no_think\n\nSelect startup context for coding agent. Use only candidate IDs shown. Select fewer records when unsure. Prefer active handoffs, unresolved blockers, live risks, recent decisions, current next steps, and durable protocol facts. Drop obsolete, repetitive, resolved, wrong-project, or low-signal items. Select global records only when explicitly cross-project and useful now. Return JSON with exactly these keys: selected_ids, session_brief, dropped_ids. selected_ids must be ordered by usefulness. session_brief must use facts from selected IDs only. dropped_ids must contain every non-selected candidate with reason enum: duplicate, stale_superseded, resolved_done, wrong_project, global_not_needed, low_signal, too_old, unknown.\n\nCandidates:\n",
    );
    for candidate in candidates {
        prompt.push_str(&render_candidate_card(candidate));
        prompt.push('\n');
    }
    prompt
}

fn render_candidate_card(candidate: &Candidate) -> String {
    let ts = timestamp_seconds(candidate.frame_ts)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| "unknown".to_string());
    let project = candidate.header.get("project").unwrap_or("unknown");
    let status = candidate.header.get("status").unwrap_or("unknown");
    let kind = candidate.header.get("type").unwrap_or("unknown");
    let body = truncate_clean(&strip_markdown_noise(&candidate.body), 600);
    format!(
        "id: {}\nproject: {}\ntimestamp: {}\nstatus: {}\ntype: {}\nscore: {:.1}\nbody: {}\n",
        candidate_id(candidate),
        project,
        ts,
        status,
        kind,
        candidate.score,
        body
    )
}

fn parse_librarian_reply(raw: &str) -> std::result::Result<LibrarianReply, String> {
    let json = extract_json_object(raw)
        .ok_or_else(|| "no JSON object in librarian response".to_string())?;
    serde_json::from_str::<LibrarianReply>(&json)
        .map_err(|err| format!("invalid librarian JSON: {err}"))
}

fn validate_librarian_reply(
    reply: LibrarianReply,
    candidates: &[&Candidate],
    max_selected: usize,
) -> std::result::Result<(Vec<String>, String), String> {
    let allowed_ids: HashSet<String> = candidates
        .iter()
        .map(|candidate| candidate_id(candidate))
        .collect();
    let mut selected_ids = Vec::new();
    let mut seen_selected = HashSet::new();
    for id in reply.selected_ids {
        if !allowed_ids.contains(&id) {
            return Err(format!("selected unknown candidate id: {id}"));
        }
        if !seen_selected.insert(id.clone()) {
            return Err(format!("duplicate selected candidate id: {id}"));
        }
        selected_ids.push(id);
    }
    if selected_ids.is_empty() {
        return Err("empty selected_ids; using heuristic fallback".to_string());
    }
    if selected_ids.len() > max_selected {
        return Err(format!(
            "selected {} records, max allowed is {max_selected}",
            selected_ids.len()
        ));
    }

    let selected: HashSet<&str> = selected_ids.iter().map(String::as_str).collect();
    let mut dropped = HashSet::new();
    for dropped_record in reply.dropped_ids {
        let _reason = dropped_record.reason;
        if !allowed_ids.contains(&dropped_record.id) {
            return Err(format!(
                "dropped unknown candidate id: {}",
                dropped_record.id
            ));
        }
        if selected.contains(dropped_record.id.as_str()) {
            return Err(format!(
                "candidate appears in selected_ids and dropped_ids: {}",
                dropped_record.id
            ));
        }
        if !dropped.insert(dropped_record.id.clone()) {
            return Err(format!(
                "duplicate dropped candidate id: {}",
                dropped_record.id
            ));
        }
    }
    let missing_drop_count = allowed_ids
        .iter()
        .filter(|id| !selected.contains(id.as_str()) && !dropped.contains(*id))
        .count();
    if missing_drop_count > 0 {
        return Err(format!(
            "dropped_ids omitted {missing_drop_count} non-selected candidates"
        ));
    }

    Ok((selected_ids, reply.session_brief))
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (offset, ch) in raw[start..].char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return raw.get(start..end).map(ToString::to_string);
                }
            }
            _ => {}
        }
    }
    None
}

fn apply_librarian_selection(
    candidates: Vec<Candidate>,
    selected_ids: &[String],
) -> Vec<Candidate> {
    if selected_ids.is_empty() {
        return candidates;
    }
    let mut by_id = BTreeMap::new();
    for candidate in candidates {
        let id = candidate_id(&candidate);
        by_id.insert(id, candidate);
    }
    let mut ordered = Vec::new();
    for id in selected_ids {
        if let Some(candidate) = by_id.remove(id) {
            ordered.push(candidate);
        }
    }
    ordered
}

fn candidate_id(candidate: &Candidate) -> String {
    let store_name = candidate
        .store
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.mv2");
    format!("{store_name}:{}", candidate.frame_id)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn truncate_brief(text: String) -> String {
    truncate_clean(&text, 400)
}

fn classify_project_match(header: &Header, body: &str, project: &str) -> ProjectMatch {
    if let Some(value) = header.get("project") {
        if normalized_project_key(value) == normalized_project_key(project) {
            return ProjectMatch::Exact;
        }
        if value.eq_ignore_ascii_case("global") {
            return ProjectMatch::Global;
        }
    }
    if body_mentions_project(body, project) {
        return ProjectMatch::Embedded;
    }
    ProjectMatch::Other
}

fn body_mentions_project(body: &str, project: &str) -> bool {
    let project_key = normalized_project_key(project);
    if project_key.len() < 4 {
        return false;
    }
    let body_key = normalized_project_key(body);
    if body_key.contains(&project_key) {
        return true;
    }
    let project_tokens = project_identity_tokens(project);
    if project_tokens.is_empty() {
        return false;
    }
    let body_tokens = tokenize(body);
    let matches = project_tokens
        .iter()
        .filter(|token| body_tokens.contains(*token))
        .count();
    matches >= project_tokens.len().min(2)
}

fn normalized_project_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn project_identity_tokens(project: &str) -> BTreeSet<String> {
    project
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .filter(|token| !STOP_WORDS.contains(&token.as_str()))
        .collect()
}

fn open_store(path: &Path) -> Result<Memvid> {
    match Memvid::open_read_only(path) {
        Ok(mem) => Ok(mem),
        Err(first_err) => {
            let tmp = std::env::temp_dir().join(format!(
                "memvid-context-{}-{}.mv2",
                std::process::id(),
                Uuid::new_v4()
            ));
            fs::copy(path, &tmp).with_context(|| {
                format!(
                    "read-only open failed ({first_err}); snapshot copy failed for {}",
                    path.display()
                )
            })?;
            let result = Memvid::open_read_only(&tmp).with_context(|| {
                format!(
                    "read-only open failed ({first_err}); snapshot copy also failed for {}",
                    path.display()
                )
            });
            let _ = fs::remove_file(&tmp);
            result
        }
    }
}

fn read_frame_text(mem: &mut Memvid, frame_id: u64) -> String {
    mem.frame_text_by_id(frame_id)
        .or_else(|_| {
            mem.frame_canonical_payload(frame_id)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        })
        .unwrap_or_default()
}

fn trim_record_bytes(text: &str) -> &str {
    if text.len() <= MAX_BODY_READ_BYTES {
        return text;
    }
    let mut end = MAX_BODY_READ_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn parse_header(text: &str) -> (Header, String) {
    let mut header = Header::default();
    let mut body_start = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        let line_len = line.len() + 1;
        if trimmed.is_empty() {
            body_start += line_len;
            continue;
        }
        if let Some((key, value)) = parse_header_line(trimmed) {
            header.values.insert(key.to_string(), value.to_string());
            body_start += line_len;
            continue;
        }
        break;
    }
    let body = text
        .get(body_start.min(text.len())..)
        .unwrap_or_default()
        .trim()
        .to_string();
    (header, body)
}

fn parse_header_line(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let (key, value) = inner.split_once(':')?;
    Some((key.trim(), value.trim()))
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&Utc));
    }
    let value: i128 = raw.trim().parse().ok()?;
    if value > 10_000_000_000_000_000 {
        let secs = value / 1_000_000_000;
        let nanos = (value % 1_000_000_000) as u32;
        timestamp_parts(secs, nanos)
    } else if value > 10_000_000_000 {
        timestamp_parts(value / 1_000, ((value % 1_000) as u32) * 1_000_000)
    } else {
        timestamp_parts(value, 0)
    }
}

fn timestamp_seconds(secs: i64) -> Option<DateTime<Utc>> {
    timestamp_parts(i128::from(secs), 0)
}

fn timestamp_parts(secs: i128, nanos: u32) -> Option<DateTime<Utc>> {
    let secs_i64 = i64::try_from(secs).ok()?;
    if !(946_684_800..=4_102_444_800).contains(&secs_i64) {
        return None;
    }
    Utc.timestamp_opt(secs_i64, nanos).single()
}

fn date_start(date: NaiveDate) -> Option<DateTime<Utc>> {
    date.and_hms_opt(0, 0, 0)
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn score_candidate(
    header: &Header,
    body: &str,
    project_match: ProjectMatch,
    query_tokens: &BTreeSet<String>,
    age_hours: f64,
) -> f64 {
    let mut score = 0.0;
    match project_match {
        ProjectMatch::Exact => score += 70.0,
        ProjectMatch::Embedded => score += 62.0,
        ProjectMatch::Global => score += 4.0,
        ProjectMatch::Other => score += 2.0,
    }
    let record_type = header.get("type").unwrap_or_default();
    score += match record_type {
        "handoff" => 30.0,
        "error" => 24.0,
        "update" => 16.0,
        "import" => 6.0,
        _ => 8.0,
    };
    score += match header.get("status").unwrap_or_default() {
        "handing-off" => 26.0,
        "error" => 22.0,
        "in-progress" => 14.0,
        "done" => 10.0,
        "migrated" => 4.0,
        _ => 4.0,
    };
    score += recency_bonus(age_hours);

    let body_tokens = tokenize(body);
    if !query_tokens.is_empty() && !body_tokens.is_empty() {
        let overlap = query_tokens
            .iter()
            .filter(|token| body_tokens.contains(*token))
            .count();
        score += (overlap as f64 / query_tokens.len() as f64).min(1.0) * 35.0;
    }
    let lower = body.to_ascii_lowercase();
    if matches!(project_match, ProjectMatch::Embedded) && record_type == "import" {
        score += 12.0;
    }
    if contains_any(
        &lower,
        &["risk", "blocked", "bug", "error", "failed", "todo", "next"],
    ) {
        score += 10.0;
    }
    if contains_any(
        &lower,
        &["must", "never", "source of truth", "protocol", "invariant"],
    ) && !matches!(project_match, ProjectMatch::Global)
    {
        score += 8.0;
    }
    score
}

fn recency_bonus(age_hours: f64) -> f64 {
    if age_hours <= RECENT_DETAIL_HOURS {
        return 78.0;
    }
    if age_hours <= RECENCY_CLIFF_HOURS {
        let progress =
            (age_hours - RECENT_DETAIL_HOURS) / (RECENCY_CLIFF_HOURS - RECENT_DETAIL_HOURS);
        return 78.0 - (progress * 24.0);
    }
    if age_hours <= 48.0 {
        let progress = (age_hours - RECENCY_CLIFF_HOURS) / (48.0 - RECENCY_CLIFF_HOURS);
        return 18.0 - (progress * 14.0);
    }
    if age_hours <= DEFAULT_COMPRESSION_HORIZON_HOURS {
        let progress = (age_hours - 48.0) / (DEFAULT_COMPRESSION_HORIZON_HOURS - 48.0);
        return (4.0 - (progress * 4.0)).max(0.0);
    }
    0.0
}

fn query_tokens(project: &str, cwd: &Path, queries: &[String]) -> BTreeSet<String> {
    let mut seed = String::new();
    seed.push_str(project);
    seed.push(' ');
    if queries.is_empty() {
        seed.push_str(&cwd.to_string_lossy());
        seed.push_str(" handoff next risk bug decision current state task");
    } else {
        if let Some(name) = cwd.file_name().and_then(|name| name.to_str()) {
            seed.push_str(name);
            seed.push(' ');
        }
        for query in queries {
            seed.push(' ');
            seed.push_str(query);
        }
    }
    tokenize(&seed)
}

fn tokenize(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .filter(|token| !STOP_WORDS.contains(&token.as_str()))
        .collect()
}

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "into", "only", "must", "should", "when",
    "where", "will", "are", "was", "were", "have", "has", "had", "not", "but", "all", "any",
];

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn dedupe_candidates(candidates: &mut Vec<Candidate>) {
    let mut seen = HashSet::new();
    candidates.retain(|candidate| {
        let norm = normalize_for_dedupe(&candidate.body);
        if norm.is_empty() {
            return false;
        }
        seen.insert(norm)
    });
}

fn trim_candidates_for_budget(candidates: Vec<Candidate>, max_records: usize) -> Vec<Candidate> {
    let mut recent = 0usize;
    let mut older = 0usize;
    let older_budget = MAX_OLDER_RECORDS.min(max_records);
    let mut kept = Vec::new();

    for candidate in candidates {
        if kept.len() >= max_records {
            break;
        }
        if candidate.age_hours <= RECENCY_CLIFF_HOURS {
            recent += 1;
            kept.push(candidate);
            continue;
        }
        if older < older_budget || (recent == 0 && kept.len() < max_records) {
            older += 1;
            kept.push(candidate);
        }
    }

    kept
}

fn normalize_for_dedupe(text: &str) -> String {
    normalize_ws(text)
        .chars()
        .take(512)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn render_packet(input: RenderInput<'_>) -> String {
    let mut out = String::new();
    let generated = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut header = format!(
        "# Memvid Startup Context\n\n- agent: `{}`\n- project: `{}`\n- cwd: `{}`\n- generated: `{generated}`\n- store_dir: `{}`\n- stores_searched: `{}`\n- compression_horizon: `7 days`\n- librarian: `{}` (candidates: `{}`, selected: `{}`, elapsed_ms: `{}`)\n",
        input.args.agent,
        input.project,
        input.cwd.display(),
        input.store_dir.display(),
        input.stores_searched,
        input.librarian.status(),
        input.librarian.candidate_count,
        input.librarian.selected_ids.len(),
        input.librarian.elapsed_ms,
    );
    if let Some(warning) = input.librarian.warning.as_ref() {
        header.push_str(&format!("- librarian_warning: `{}`\n", warning));
    }
    header.push('\n');
    push_block(&mut out, input.budget_chars, &header);
    if let Some(brief) = input.librarian.brief.as_ref() {
        push_block(
            &mut out,
            input.budget_chars,
            &format!("## Librarian Brief\n\n{}\n\n", brief),
        );
    }
    push_block(
        &mut out,
        input.budget_chars,
        "## Operating Rules\n\nAgents write durable memory only by atomically renaming Markdown files into `/var/lib/memvid/queue`. Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state. Do not invoke memvid binaries for writes. Do not touch `.mv2`, `/var/lib/memvid/processing`, `/var/lib/memvid/ingest`, `/var/lib/memvid/done`, `/var/lib/memvid/failed`, or `/var/lib/memvid/store`. Treat this packet as read-only startup recall.\n\n",
    );
    push_block(
        &mut out,
        input.budget_chars,
        "## Queue Write Checkpoints\n\nWrite to the queue when a task is complete, code or protocol decisions are finalized, a file/function/command is created or renamed, a concrete blocker or bug is observed, tests change direction, or the session is ending. Use `[project:global]` only for explicit cross-project coordination; ordinary workspace facts stay in the current project shard.\n\n",
    );

    let sections = [
        (Section::Handoff, "## Recent Handoffs"),
        (Section::Fresh, "## Fresh Context"),
        (Section::Project, "## Active Project State"),
        (Section::Risk, "## Open Risks"),
        (Section::Older, "## Older Canonical Facts"),
        (Section::Recall, "## Relevant Recall"),
    ];
    let latest_handoff_frame = input
        .candidates
        .iter()
        .filter(|candidate| classify(candidate) == Section::Handoff)
        .max_by_key(|candidate| candidate.frame_ts)
        .map(|candidate| candidate.frame_id);
    let mut rendered_any = false;
    for (section, title) in sections {
        let records: Vec<&Candidate> = input
            .candidates
            .iter()
            .filter(|candidate| classify(candidate) == section)
            .take(10)
            .collect();
        if records.is_empty() {
            continue;
        }
        if !push_block(&mut out, input.budget_chars, &format!("{title}\n\n")) {
            break;
        }
        let mut section_rendered = false;
        for candidate in records {
            let expand_handoff =
                latest_handoff_frame == Some(candidate.frame_id) && section == Section::Handoff;
            let record = format!(
                "{}\n",
                render_candidate(
                    candidate,
                    input.args.compression_horizon_hours,
                    expand_handoff,
                )
            );
            if push_block(&mut out, input.budget_chars, &record) {
                section_rendered = true;
            }
        }
        push_block(&mut out, input.budget_chars, "\n");
        if section_rendered {
            rendered_any = true;
        }
    }

    if !rendered_any {
        push_block(
            &mut out,
            input.budget_chars,
            "## Recall\n\nNo matching source-of-truth records were available in the scanned store window.\n\n",
        );
    }

    if input.args.include_store_errors && !input.errors.is_empty() {
        let mut block = String::from("## Store Read Warnings\n\n");
        for err in input.errors.iter().take(8) {
            block.push_str("- ");
            block.push_str(err);
            block.push('\n');
        }
        block.push('\n');
        push_block(&mut out, input.budget_chars, &block);
    }

    push_block(
        &mut out,
        input.budget_chars,
        "## Recall Boundary\n\nThis context is a compressed view. Raw `.mv2` stores remain backend-owned. Ask the launcher or user for a narrower `memvid-context --query ...` packet when more detail is needed.\n",
    );
    out
}

fn classify(candidate: &Candidate) -> Section {
    match candidate.header.get("type") {
        Some("handoff") => return Section::Handoff,
        Some("error") => return Section::Risk,
        _ => {}
    }
    match candidate.header.get("status") {
        Some("handing-off" | "error") => return Section::Risk,
        _ => {}
    }
    let lower = candidate.body.to_ascii_lowercase();
    if contains_any(
        &lower,
        &["blocked", "unresolved", "open risk", "bug", "error", "todo"],
    ) {
        return Section::Risk;
    }
    if candidate.age_hours <= RECENCY_CLIFF_HOURS {
        return Section::Fresh;
    }
    if matches!(candidate.header.get("project"), Some("global")) {
        return Section::Older;
    }
    if candidate.header.get("project").is_some() {
        return Section::Project;
    }
    Section::Recall
}

fn render_candidate(candidate: &Candidate, horizon_hours: f64, expand_handoff: bool) -> String {
    let max_chars = compression_limit(candidate, horizon_hours);
    let compressed = if expand_handoff {
        compress_handoff_body(&candidate.body, max_chars.max(2_400))
    } else {
        compress_body(&candidate.body, max_chars, candidate.age_hours)
    };
    let source_date = timestamp_seconds(candidate.frame_ts)
        .map(|timestamp| timestamp.date_naive().to_string())
        .or_else(|| candidate.store_date.map(|date| date.to_string()))
        .unwrap_or_else(|| "unknown-date".to_string());
    let store_name = candidate
        .store
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.mv2");
    let project = candidate.header.get("project").unwrap_or("unknown");
    let kind = candidate.header.get("type").unwrap_or("unknown");
    let status = candidate.header.get("status").unwrap_or("unknown");
    format!(
        "- [{source_date} frame:{} score:{:.1} project:{} type:{} status:{} store:{} frame_ts:{}] {}",
        candidate.frame_id,
        candidate.score,
        project,
        kind,
        status,
        store_name,
        candidate.frame_ts,
        compressed
    )
}

fn compress_handoff_body(body: &str, max_chars: usize) -> String {
    let body = strip_markdown_noise(body);
    let mut parts = Vec::new();
    let mut first_paragraph = Vec::new();
    let mut in_plan_details = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !first_paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.eq_ignore_ascii_case("Plan details:") {
            break;
        }
        first_paragraph.push(trimmed);
    }
    if !first_paragraph.is_empty() {
        parts.push(normalize_ws(&first_paragraph.join(" ")));
    }

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("Plan details:") {
            in_plan_details = true;
            continue;
        }
        if in_plan_details {
            if trimmed.starts_with("- ") {
                parts.push(normalize_ws(trimmed));
            } else if !trimmed.is_empty() {
                break;
            }
        }
    }

    if parts.is_empty() {
        compress_body(&body, max_chars, 0.0)
    } else {
        truncate_clean(&parts.join("; "), max_chars)
    }
}

fn compression_limit(candidate: &Candidate, horizon_hours: f64) -> usize {
    let age = candidate.age_hours;
    let importance = (candidate.score / 120.0).clamp(0.0, 1.0);
    let resistance = 0.35 * importance;
    let aggression = ((age / horizon_hours).clamp(0.0, 1.0) * (1.0 - resistance)).clamp(0.0, 1.0);
    let base = if age <= RECENT_DETAIL_HOURS {
        1_800
    } else if age <= RECENCY_CLIFF_HOURS {
        1_200
    } else if age <= 24.0 {
        500
    } else if age <= 48.0 {
        260
    } else if age <= 96.0 {
        180
    } else if age <= horizon_hours {
        140
    } else {
        100
    };
    ((base as f64) * (1.0 - aggression * 0.45)).round() as usize
}

fn compress_body(body: &str, max_chars: usize, age_hours: f64) -> String {
    let body = strip_markdown_noise(body);
    if age_hours <= RECENT_DETAIL_HOURS {
        return truncate_clean(&body, max_chars);
    }
    let lines = informative_lines(&body);
    let joined = if lines.is_empty() {
        normalize_ws(&body)
    } else {
        lines.join("; ")
    };
    if age_hours >= DEFAULT_COMPRESSION_HORIZON_HOURS {
        return truncate_clean(&one_line_fact(&joined), max_chars);
    }
    truncate_clean(&joined, max_chars)
}

fn strip_markdown_noise(body: &str) -> String {
    body.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && trimmed != "## Content" && trimmed != "## Legacy Import"
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn informative_lines(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim().trim_start_matches("- ").trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.starts_with('#')
            || contains_any(
                &lower,
                &[
                    "must",
                    "never",
                    "source",
                    "protocol",
                    "service",
                    "installed",
                    "enabled",
                    "failed",
                    "error",
                    "risk",
                    "next",
                    "todo",
                    "created",
                    "fixed",
                    "changed",
                    "migration",
                    "/var/lib/memvid",
                    "/usr/local",
                    ".mv2",
                ],
            )
        {
            lines.push(normalize_ws(trimmed));
        }
        if lines.len() >= 5 {
            break;
        }
    }
    lines
}

fn one_line_fact(text: &str) -> String {
    let mut best = String::new();
    for sentence in text.split(['.', '\n', ';']) {
        let sentence = normalize_ws(sentence);
        if sentence.len() > best.len() {
            best = sentence;
        }
    }
    if best.is_empty() {
        normalize_ws(text)
    } else {
        best
    }
}

fn truncate_clean(text: &str, max_chars: usize) -> String {
    let text = normalize_ws(text);
    if text.chars().count() <= max_chars {
        return text;
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars.saturating_sub(3)) {
        out.push(ch);
    }
    if let Some(idx) = out.rfind(['.', ';', ',']) {
        if idx > max_chars / 2 {
            out.truncate(idx + 1);
        }
    }
    out.push_str("...");
    out
}

fn normalize_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_block(out: &mut String, budget_chars: usize, block: &str) -> bool {
    if out.len() >= budget_chars {
        return false;
    }
    let remaining = budget_chars - out.len();
    if block.len() <= remaining {
        out.push_str(block);
        return true;
    }
    if remaining < 160 {
        return false;
    }
    out.push_str(&truncate_preserve_newlines(block, remaining));
    out.push('\n');
    true
}

fn truncate_preserve_newlines(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars.saturating_sub(3)) {
        out.push(ch);
    }
    if let Some(idx) = out.rfind('\n') {
        if idx > max_chars / 3 {
            out.truncate(idx);
        }
    }
    out.push_str("...");
    out
}

fn infer_project(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("global")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(age_hours: f64, score: f64) -> Candidate {
        Candidate {
            store: PathBuf::from("/tmp/memvid.mv2"),
            store_date: None,
            frame_id: 1,
            frame_ts: 0,
            header: Header::default(),
            body: "body".to_string(),
            age_hours,
            score,
        }
    }

    #[test]
    fn recency_bonus_drops_sharply_after_sixteen_hours() {
        assert!(recency_bonus(2.0) > recency_bonus(12.0));
        assert!(recency_bonus(12.0) > recency_bonus(20.0));
        assert!(recency_bonus(20.0) > recency_bonus(72.0));
        assert!(recency_bonus(20.0) < 20.0);
    }

    #[test]
    fn trim_candidates_caps_older_records() {
        let mut candidates = Vec::new();
        for _ in 0..20 {
            candidates.push(candidate(2.0, 100.0));
        }
        for _ in 0..40 {
            candidates.push(candidate(36.0, 90.0));
        }

        let kept = trim_candidates_for_budget(candidates, 48);
        let older = kept
            .iter()
            .filter(|candidate| candidate.age_hours > RECENCY_CLIFF_HOURS)
            .count();
        assert_eq!(kept.len(), 32);
        assert_eq!(older, MAX_OLDER_RECORDS);
    }

    #[test]
    fn classify_treats_sixteen_hour_records_as_fresh() {
        let fresh = candidate(RECENCY_CLIFF_HOURS, 80.0);
        let older = candidate(RECENCY_CLIFF_HOURS + 0.1, 80.0);
        assert_eq!(classify(&fresh), Section::Fresh);
        assert_ne!(classify(&older), Section::Fresh);
    }

    #[test]
    fn render_candidate_uses_frame_timestamp_date_for_project_shards() {
        let mut candidate = candidate(1.0, 100.0);
        candidate.frame_ts = 1_778_222_734;

        let rendered = render_candidate(&candidate, DEFAULT_COMPRESSION_HORIZON_HOURS, false);

        assert!(rendered.starts_with("- [2026-05-08 frame:1"));
        assert!(!rendered.contains("unknown-date"));
    }

    #[test]
    fn only_latest_handoff_renders_plan_details() {
        let args = Args::parse_from(["memvid-context", "--budget-tokens", "2000"]);
        let mut older = candidate(DEFAULT_COMPRESSION_HORIZON_HOURS + 1.0, 100.0);
        older.frame_id = 1;
        older.frame_ts = 1_700_000_000;
        older.header.values.insert("type".into(), "handoff".into());
        older.body = format!(
            "Older summary.\n\nPlan details:\n- {} Older detail kept only if latest.",
            "padding ".repeat(80)
        );

        let mut latest = candidate(1.0, 100.0);
        latest.frame_id = 2;
        latest.frame_ts = 1_778_222_734;
        latest.header.values.insert("type".into(), "handoff".into());
        latest.body = "Latest summary.\n\nPlan details:\n- Non-shell launchers.\n- Wrapper regression tests.\n- Startup diagnostics.\n- Full integration.\n- Installer dry-run.".into();

        let output = render_packet(RenderInput {
            args: &args,
            project: "memvid",
            cwd: Path::new("/home/foobis/memvid"),
            store_dir: Path::new("/var/lib/memvid/store"),
            stores_searched: 1,
            candidates: &[older, latest],
            errors: &[],
            budget_chars: 8_000,
            librarian: &LibrarianResult::default(),
        });

        assert!(output.contains("Latest summary."));
        assert!(output.contains("Non-shell launchers."));
        assert!(output.contains("Installer dry-run."));
        assert!(!output.contains("Older detail kept only if latest."));
    }

    #[test]
    fn render_header_reports_disabled_librarian_diagnostics() {
        let args = Args::parse_from(["memvid-context", "--budget-tokens", "2000"]);
        let output = render_packet(RenderInput {
            args: &args,
            project: "memvid",
            cwd: Path::new("/home/foobis/memvid"),
            store_dir: Path::new("/var/lib/memvid/store"),
            stores_searched: 1,
            candidates: &[],
            errors: &[],
            budget_chars: 8_000,
            librarian: &LibrarianResult {
                candidate_count: 3,
                ..Default::default()
            },
        });

        assert!(output
            .contains("- librarian: `disabled` (candidates: `3`, selected: `0`, elapsed_ms: `0`)"));
        assert!(output.contains("Do not use agent-native memory tools"));
        assert!(!output.contains("librarian_warning"));
    }

    #[test]
    fn render_header_reports_enabled_librarian_diagnostics() {
        let args = Args::parse_from(["memvid-context", "--budget-tokens", "2000"]);
        let selected_ids = vec!["memvid.mv2:1".to_string(), "memvid.mv2:2".to_string()];
        let librarian = LibrarianResult {
            active: true,
            candidate_count: 7,
            elapsed_ms: 42,
            brief: None,
            selected_ids,
            warning: None,
        };
        let output = render_packet(RenderInput {
            args: &args,
            project: "memvid",
            cwd: Path::new("/home/foobis/memvid"),
            store_dir: Path::new("/var/lib/memvid/store"),
            stores_searched: 1,
            candidates: &[],
            errors: &[],
            budget_chars: 8_000,
            librarian: &librarian,
        });

        assert!(output
            .contains("- librarian: `enabled` (candidates: `7`, selected: `2`, elapsed_ms: `42`)"));
        assert!(!output.contains("librarian_warning"));
    }

    #[test]
    fn render_header_reports_librarian_fallback_warning_without_store_errors() {
        let args = Args::parse_from(["memvid-context", "--budget-tokens", "2000"]);
        let librarian = LibrarianResult {
            active: true,
            candidate_count: 5,
            elapsed_ms: 11,
            brief: None,
            selected_ids: Vec::new(),
            warning: Some("request failed: offline".to_string()),
        };
        let output = render_packet(RenderInput {
            args: &args,
            project: "memvid",
            cwd: Path::new("/home/foobis/memvid"),
            store_dir: Path::new("/var/lib/memvid/store"),
            stores_searched: 1,
            candidates: &[],
            errors: &["librarian: request failed: offline".to_string()],
            budget_chars: 8_000,
            librarian: &librarian,
        });

        assert!(output.contains(
            "- librarian: `fallback` (candidates: `5`, selected: `0`, elapsed_ms: `11`)"
        ));
        assert!(output.contains("- librarian_warning: `request failed: offline`"));
        assert!(!output.contains("## Store Read Warnings"));
    }

    #[test]
    fn parse_librarian_reply_accepts_embedded_json() {
        let raw = "preface {\"selected_ids\":[\"memvid.mv2:42\"],\"session_brief\":\"Current task\",\"dropped_ids\":[{\"id\":\"memvid.mv2:41\",\"reason\":\"duplicate\"}]} suffix";
        let parsed = parse_librarian_reply(raw).unwrap();
        assert_eq!(parsed.selected_ids, vec!["memvid.mv2:42"]);
        assert_eq!(parsed.session_brief, "Current task");
        assert_eq!(parsed.dropped_ids[0].reason, DropReason::Duplicate);
    }

    #[test]
    fn apply_librarian_selection_keeps_only_selected_records() {
        let mut first = candidate(1.0, 90.0);
        first.frame_id = 10;
        let mut second = candidate(2.0, 80.0);
        second.frame_id = 20;
        let selected =
            apply_librarian_selection(vec![first, second], &["memvid.mv2:20".to_string()]);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].frame_id, 20);
    }

    #[test]
    fn librarian_runtime_enables_from_config_without_flag() {
        let args = Args::parse_from(["memvid-context"]);
        let configured = Librarian {
            enabled: true,
            endpoint: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
            model: "qwen3:8b".to_string(),
            timeout_ms: 20_000,
            max_candidates: 12,
            max_selected: 6,
            max_tokens: 512,
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 1.5,
        };
        let runtime = librarian_runtime(&args, Some(&configured)).unwrap();
        assert_eq!(runtime.model, "qwen3:8b");
        assert_eq!(runtime.timeout_ms, 20_000);
        assert_eq!(runtime.max_candidates, 12);
        assert_eq!(runtime.max_selected, 6);
    }

    #[test]
    fn no_librarian_overrides_enabled_config() {
        let args = Args::parse_from(["memvid-context", "--no-librarian"]);
        let configured = Librarian {
            enabled: true,
            endpoint: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
            model: "qwen3:8b".to_string(),
            timeout_ms: 20_000,
            max_candidates: 12,
            max_selected: 6,
            max_tokens: 512,
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 1.5,
        };
        assert!(librarian_runtime(&args, Some(&configured)).is_none());
    }

    #[test]
    fn validate_librarian_reply_rejects_over_selection() {
        let first = candidate(1.0, 90.0);
        let mut second = candidate(2.0, 80.0);
        second.frame_id = 2;
        let reply = LibrarianReply {
            selected_ids: vec!["memvid.mv2:1".to_string(), "memvid.mv2:2".to_string()],
            session_brief: "too many".to_string(),
            dropped_ids: Vec::new(),
        };
        let err = validate_librarian_reply(reply, &[&first, &second], 1).unwrap_err();
        assert!(err.contains("max allowed"));
    }

    #[test]
    fn single_candidate_librarian_path_skips_model() {
        let runtime = LibrarianRuntime {
            endpoint: "http://127.0.0.1:1/v1/chat/completions".to_string(),
            model: "qwen3:8b".to_string(),
            timeout_ms: 1,
            max_candidates: 12,
            max_selected: 6,
            max_tokens: 512,
            temperature: 0.0,
            top_p: 1.0,
            presence_penalty: 1.5,
        };
        let item = candidate(1.0, 90.0);
        let result = rerank_with_librarian(
            &runtime,
            "memvid",
            Path::new("/home/foobis/memvid"),
            &[],
            &[item],
        );
        assert_eq!(result.selected_ids, vec!["memvid.mv2:1"]);
        assert!(result.warning.is_none());
    }
}
