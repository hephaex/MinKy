//! On-demand wiki generation (`generate_wiki` MCP tool).
//!
//! Approved plan/spec summary:
//! - Explicit, scoped exception to the broker's system-wide vendor-neutral,
//!   no-synthesis principle: this module (and [`crate::synth_client`]) is the
//!   only place the broker calls an external LLM to synthesize content.
//! - Reuses [`crate::cascade::run`] unchanged for evidence collection.
//! - Hard-rejects (writes nothing) if either minky or the synthesis endpoint
//!   is unreachable, or if the rendered document is missing required
//!   verification elements (paragraph citations + unverified banner).
//! - Output lives outside `working_set.memory_root` and outside the
//!   sync-shared config tree (see [`crate::config::resolve_safe_output_dir`]),
//!   so generated pages are never re-ingested as their own evidence.

use crate::cascade::CascadeError;

/// Element missing from a synthesized document that fails the mandatory
/// verification gate ([`crate::wiki::validate_rendered`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingElement {
    /// The top-of-file "AI-generated / unverified" banner is absent.
    UnverifiedBanner,
    /// No `[^n]`-style paragraph citation/footnote markers were found.
    ParagraphCitations,
}

#[derive(Debug, thiserror::Error)]
pub enum WikiError {
    #[error("synthesis disabled in config ([synthesis].enable=false); refused")]
    SynthesisDisabled,
    #[error("cascade failed: {0}")]
    Cascade(#[from] CascadeError),
    #[error("minky unreachable; hard-refused (no file written)")]
    MinkyOffline,
    #[error("synthesis LLM endpoint unreachable/timeout ({0}); hard-refused")]
    SynthOffline(String),
    #[error("synthesis LLM HTTP error {status}: {body}")]
    SynthHttp { status: u16, body: String },
    #[error("synthesized document missing required element: {0:?}")]
    ValidationFailed(MissingElement),
    #[error("no evidence collected; nothing to synthesize")]
    NoEvidence,
    #[error("wiki output_dir {0} resolves inside memory_root/sync tree; refused")]
    UnsafeOutputDir(std::path::PathBuf),
    #[error("wiki io error at {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

impl From<crate::config::ConfigError> for WikiError {
    /// `resolve_safe_output_dir` returns `ConfigError::UnsafeOutputDir` on a
    /// detected self-reference; every other `ConfigError` variant cannot
    /// actually occur on that call path, but is threaded through rather than
    /// panicking so a future change to the guard's error surface fails safe.
    fn from(err: crate::config::ConfigError) -> Self {
        match err {
            crate::config::ConfigError::UnsafeOutputDir(path) => WikiError::UnsafeOutputDir(path),
            other => WikiError::Io {
                path: std::path::PathBuf::new(),
                source: std::io::Error::other(other.to_string()),
            },
        }
    }
}

use crate::cascade::Evidence;
use crate::config::BrokerConfig;
use crate::minky_client::MinkyClient;
use crate::synth_client::SynthClient;
use crate::storage::Storage;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Characters that are illegal or unsafe in a path component on the platforms
/// the broker targets, plus the `..` traversal sequence (handled separately).
const FORBIDDEN_SLUG_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|', '.'];

/// Maximum number of same-second filename collisions to disambiguate before
/// giving up (a re-request in the same UTC second reuses the stamp).
const MAX_FILENAME_ATTEMPTS: u32 = 10;

/// Normalize an arbitrary topic string into a filesystem-safe slug.
///
/// Pipeline: lowercase → strip forbidden/control characters and `..` traversal
/// → collapse runs of whitespace/separators into a single `-` → trim leading
/// and trailing `-` → truncate to `max_len` **characters** (not bytes, so a
/// multi-byte boundary is never split) → fall back to `"untitled"` if empty.
pub fn slugify(input: &str, max_len: usize) -> String {
    // Drop the `..` traversal sequence outright before per-char filtering.
    let without_traversal = input.replace("..", " ");
    let lowercased = without_traversal.to_lowercase();

    let mut out = String::with_capacity(lowercased.len());
    let mut pending_sep = false;
    for ch in lowercased.chars() {
        let is_separator = ch.is_whitespace() || ch == '-' || ch == '_';
        let is_forbidden = FORBIDDEN_SLUG_CHARS.contains(&ch) || ch.is_control();
        if is_separator {
            pending_sep = true;
            continue;
        }
        if is_forbidden {
            // A stripped forbidden char also acts as a separator so that
            // "a/b" becomes "a-b" rather than "ab".
            pending_sep = true;
            continue;
        }
        if pending_sep && !out.is_empty() {
            out.push('-');
        }
        pending_sep = false;
        out.push(ch);
    }

    let trimmed = out.trim_matches('-');
    let truncated: String = trimmed.chars().take(max_len).collect();
    // Truncation can leave a trailing `-`; trim once more.
    let cleaned = truncated.trim_matches('-');

    if cleaned.is_empty() {
        "untitled".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Build the wiki filename `{slug}-{stamp}.md` from an already-formatted UTC
/// timestamp string (see [`utc_stamp`]).
pub fn wiki_filename(slug: &str, stamp: &str) -> String {
    format!("{slug}-{stamp}.md")
}

/// Convert epoch seconds (UTC) into calendar fields
/// `(year, month, day, hour, minute, second)` using the standard
/// days-from-civil algorithm — no `chrono`/`time` dependency.
fn epoch_to_civil(epoch_secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = epoch_secs.div_euclid(86_400);
    let secs = epoch_secs.rem_euclid(86_400);
    let hour = (secs / 3600) as u32;
    let minute = ((secs % 3600) / 60) as u32;
    let second = (secs % 60) as u32;

    // Howard Hinnant's civil-from-days: valid for the full proleptic
    // Gregorian range. `days` is the count since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day, hour, minute, second)
}

/// Compact UTC timestamp `%Y%m%dT%H%M%SZ` for filenames.
pub fn utc_stamp(epoch_secs: i64) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_civil(epoch_secs);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

/// Extended UTC timestamp `%Y-%m-%dT%H:%M:%SZ` (ISO-8601) for front-matter.
pub fn utc_iso8601(epoch_secs: i64) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_civil(epoch_secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Escape a value for embedding inside a double-quoted YAML scalar.
fn yaml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Trim a snippet to a short single-line excerpt for the footnote list.
fn snippet_excerpt(snippet: &str) -> String {
    let one_line = snippet.replace(['\n', '\r'], " ");
    let collapsed = one_line.split_whitespace().collect::<Vec<_>>().join(" ");
    let excerpt: String = collapsed.chars().take(120).collect();
    excerpt
}

/// Render the final markdown document: YAML front-matter, the mandatory
/// unverified banner, the synthesized body, and a footnote source list.
pub fn render_markdown(
    topic: &str,
    query: &str,
    generated_at_iso: &str,
    model: &str,
    evidence: &[Evidence],
    body: &str,
) -> String {
    let mut out = String::new();

    // Front-matter.
    out.push_str("---\n");
    out.push_str(&format!("topic: \"{}\"\n", yaml_escape(topic)));
    out.push_str(&format!("query: \"{}\"\n", yaml_escape(query)));
    out.push_str(&format!("generated_at: \"{}\"\n", yaml_escape(generated_at_iso)));
    out.push_str("generator: \"broker/generate_wiki\"\n");
    out.push_str(&format!("synthesis_model: \"{}\"\n", yaml_escape(model)));
    out.push_str("verified: false\n");
    out.push_str("---\n\n");

    // Unverified banner (blockquote). The literal "AI 자동생성" is the string
    // the validation gate keys on.
    out.push_str(
        "> ⚠️ **AI 자동생성 · 미검증(UNVERIFIED)** — 이 문서는 외부 LLM이 검색 증거로부터\n",
    );
    out.push_str(
        "> 자동 합성했으며 사람이 검증하지 않았습니다. 각주 원문을 반드시 대조하세요.\n\n",
    );

    // Title + body.
    out.push_str(&format!("# {topic}\n\n"));
    out.push_str(body.trim_end());
    out.push_str("\n\n");

    // Sources.
    out.push_str("## 출처\n");
    for (i, ev) in evidence.iter().enumerate() {
        let n = i + 1;
        out.push_str(&format!(
            "[^{n}]: `{}` — \"{}\" (source_id: {}, sim: {:.3})\n",
            ev.source_id,
            snippet_excerpt(&ev.snippet),
            ev.source_id,
            ev.similarity,
        ));
    }

    out
}

/// Mandatory verification gate. A synthesized document may only be written if
/// it carries the unverified banner **and** at least one footnote marker plus
/// a sources section.
pub fn validate_rendered(rendered: &str) -> Result<(), WikiError> {
    if !rendered.contains("AI 자동생성") {
        return Err(WikiError::ValidationFailed(MissingElement::UnverifiedBanner));
    }
    if !(rendered.contains("[^") && rendered.contains("## 출처")) {
        return Err(WikiError::ValidationFailed(
            MissingElement::ParagraphCitations,
        ));
    }
    Ok(())
}

/// Build the synthesis prompt from the collected evidence, instructing the LLM
/// to attach `[^N]` footnote markers that line up with the rendered sources.
fn build_synthesis_prompt(topic: &str, query: &str, evidence: &[Evidence]) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!(
        "다음 검색 근거를 바탕으로 주제 \"{topic}\"에 대한 위키 문서 본문을 한국어 마크다운으로 작성하라.\n"
    ));
    prompt.push_str(&format!("원 질의: {query}\n"));
    prompt.push_str(
        "규칙: 각 문단마다 사용한 근거의 번호를 [^N] 형태의 각주 마커로 붙여라. \
         제목(# 헤더)이나 '## 출처' 섹션은 작성하지 말고 본문 문단만 작성하라.\n\n",
    );
    prompt.push_str("근거:\n");
    for (i, ev) in evidence.iter().enumerate() {
        let n = i + 1;
        prompt.push_str(&format!(
            "[{n}] (source_id={}, title={}, sim={:.3}) {}\n",
            ev.source_id,
            ev.title,
            ev.similarity,
            snippet_excerpt(&ev.snippet),
        ));
    }
    prompt
}

/// Pick the first non-existing wiki path in `dir`, disambiguating same-second
/// collisions with `-1`, `-2`, … suffixes. `exists` is injected so the retry
/// logic is unit-testable without touching the filesystem.
fn resolve_unique_path(
    dir: &Path,
    slug: &str,
    stamp: &str,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let base = dir.join(wiki_filename(slug, stamp));
    if !exists(&base) {
        return Some(base);
    }
    for n in 1..MAX_FILENAME_ATTEMPTS {
        let name = format!("{slug}-{stamp}-{n}.md");
        let candidate = dir.join(&name);
        if !exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Removes an orphaned temp file on drop unless explicitly disarmed after a
/// successful rename.
struct TmpGuard {
    path: Option<PathBuf>,
}

impl TmpGuard {
    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TmpGuard {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Atomically write `contents` into `output_dir` under a unique filename
/// derived from `slug`/`stamp`. Writes to a `.tmp-` sibling with `O_EXCL`
/// first, then renames into place, so a partial/crashed write never leaves a
/// visible document. Existing documents are never clobbered.
fn write_document_atomic(
    output_dir: &Path,
    slug: &str,
    stamp: &str,
    contents: &str,
) -> Result<PathBuf, WikiError> {
    let final_path = resolve_unique_path(output_dir, slug, stamp, |p| p.exists()).ok_or_else(|| {
        WikiError::Io {
            path: output_dir.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "could not find a unique wiki filename after retries",
            ),
        }
    })?;

    let final_name = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled.md")
        .to_string();

    // Create the temp file with O_EXCL so concurrent generators cannot share
    // it; disambiguate the temp name on the rare create_new collision.
    let (mut file, tmp_path) = {
        let mut attempt = 0u32;
        loop {
            let tmp_name = if attempt == 0 {
                format!(".tmp-{final_name}")
            } else {
                format!(".tmp-{attempt}-{final_name}")
            };
            let tmp_path = output_dir.join(&tmp_name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)
            {
                Ok(f) => break (f, tmp_path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists
                    && attempt < MAX_FILENAME_ATTEMPTS =>
                {
                    attempt += 1;
                    continue;
                }
                Err(e) => {
                    return Err(WikiError::Io {
                        path: tmp_path,
                        source: e,
                    })
                }
            }
        }
    };

    let mut guard = TmpGuard {
        path: Some(tmp_path.clone()),
    };

    file.write_all(contents.as_bytes()).map_err(|e| WikiError::Io {
        path: tmp_path.clone(),
        source: e,
    })?;
    file.sync_all().map_err(|e| WikiError::Io {
        path: tmp_path.clone(),
        source: e,
    })?;
    drop(file);

    std::fs::rename(&tmp_path, &final_path).map_err(|e| WikiError::Io {
        path: final_path.clone(),
        source: e,
    })?;
    guard.disarm();

    Ok(final_path)
}

/// Structured observability event for one `generate_wiki` attempt.
///
/// Deliberately dependency-free (no `tracing`/`log` crate — none is in
/// `Cargo.toml` and the approved plan requires zero new dependencies): emits
/// a single `key=value` line, which is grep/parse-friendly without pulling in
/// a structured-logging framework. Written to stderr so it never corrupts the
/// stdio MCP server's JSON-RPC stream on stdout.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateWikiLogEvent<'a> {
    pub query: &'a str,
    pub topic: &'a str,
    /// Stable outcome tag: `success`, `synthesis_disabled`, `minky_offline`,
    /// `no_evidence`, `unsafe_output_dir`, `synth_offline`, `synth_http`,
    /// `validation_failed`, or `io_error`.
    pub outcome: &'static str,
    /// Extra human-readable detail (e.g. the written path, or an error cause).
    pub detail: String,
}

impl<'a> GenerateWikiLogEvent<'a> {
    /// Render as a single `key=value` line (values are quoted so embedded
    /// whitespace/`=` never breaks naive line-oriented parsing).
    pub fn render_line(&self) -> String {
        format!(
            "event=generate_wiki query={:?} topic={:?} outcome={:?} detail={:?}",
            self.query, self.topic, self.outcome, self.detail
        )
    }
}

/// Stable outcome tag for a [`WikiError`], mirroring [`GenerateWikiLogEvent::outcome`].
fn error_outcome_tag(err: &WikiError) -> &'static str {
    match err {
        WikiError::SynthesisDisabled => "synthesis_disabled",
        WikiError::Cascade(_) => "cascade_error",
        WikiError::MinkyOffline => "minky_offline",
        WikiError::SynthOffline(_) => "synth_offline",
        WikiError::SynthHttp { .. } => "synth_http",
        WikiError::ValidationFailed(_) => "validation_failed",
        WikiError::NoEvidence => "no_evidence",
        WikiError::UnsafeOutputDir(_) => "unsafe_output_dir",
        WikiError::Io { .. } => "io_error",
    }
}

/// Write a [`GenerateWikiLogEvent`] to stderr (never stdout — the stdio MCP
/// server reserves stdout for the JSON-RPC transport).
fn log_to_stderr(event: &GenerateWikiLogEvent<'_>) {
    eprintln!("{}", event.render_line());
}

/// Orchestrate on-demand wiki generation. Ordering is load-bearing (pre-mortem
/// defenses): synthesis must be enabled, minky must be reachable (hard refuse
/// offline), evidence must exist, the output dir must be self-reference safe,
/// synthesis must succeed, and the rendered document must pass the validation
/// gate — all **before** any file is written.
///
/// Every attempt logs exactly one structured [`GenerateWikiLogEvent`] (success
/// or the specific failure branch) via `sink`, so the MCP server binary can
/// wire this to stderr while tests inject a capturing closure.
#[allow(clippy::too_many_arguments)]
pub async fn generate_with_sink(
    query: &str,
    topic: &str,
    config: &BrokerConfig,
    storage: &mut Storage,
    minky: &MinkyClient,
    synth: &SynthClient,
    now: i64,
    sink: &mut (dyn FnMut(&GenerateWikiLogEvent<'_>) + Send),
) -> Result<PathBuf, WikiError> {
    let result = generate_inner(query, topic, config, storage, minky, synth, now).await;
    let event = match &result {
        Ok(path) => GenerateWikiLogEvent {
            query,
            topic,
            outcome: "success",
            detail: path.display().to_string(),
        },
        Err(err) => GenerateWikiLogEvent {
            query,
            topic,
            outcome: error_outcome_tag(err),
            detail: err.to_string(),
        },
    };
    sink(&event);
    result
}

/// Convenience wrapper over [`generate_with_sink`] that logs to stderr — the
/// entry point production callers (the MCP tool) should use.
#[allow(clippy::too_many_arguments)]
pub async fn generate(
    query: &str,
    topic: &str,
    config: &BrokerConfig,
    storage: &mut Storage,
    minky: &MinkyClient,
    synth: &SynthClient,
    now: i64,
) -> Result<PathBuf, WikiError> {
    generate_with_sink(query, topic, config, storage, minky, synth, now, &mut log_to_stderr).await
}

#[allow(clippy::too_many_arguments)]
async fn generate_inner(
    query: &str,
    topic: &str,
    config: &BrokerConfig,
    storage: &mut Storage,
    minky: &MinkyClient,
    synth: &SynthClient,
    now: i64,
) -> Result<PathBuf, WikiError> {
    // 1. Synthesis must be explicitly enabled.
    if !config.synthesis.enable {
        return Err(WikiError::SynthesisDisabled);
    }

    // 2. Collect evidence via the unchanged retrieval cascade.
    let outcome = crate::cascade::run(query, config, storage, minky, now).await?;

    // 3. Minky unreachable → hard refuse, write nothing.
    if outcome.offline {
        return Err(WikiError::MinkyOffline);
    }

    // 4. No evidence → nothing to synthesize.
    if outcome.evidence.is_empty() {
        return Err(WikiError::NoEvidence);
    }

    // 5. Resolve a self-reference-safe output directory.
    let output_dir = crate::config::resolve_safe_output_dir(
        &config.wiki.output_dir,
        &config.working_set.memory_root,
        &crate::config::default_config_dir(),
    )?;

    // 6. Synthesize the body from the evidence.
    let prompt = build_synthesis_prompt(topic, query, &outcome.evidence);
    let body = synth.synthesize(&prompt).await?;

    // 7. Render the final document.
    let generated_at = utc_iso8601(now);
    let rendered = render_markdown(
        topic,
        query,
        &generated_at,
        &config.synthesis.model,
        &outcome.evidence,
        &body,
    );

    // 8. Verification gate — reject before writing anything.
    validate_rendered(&rendered)?;

    // 9. Ensure the output directory exists.
    std::fs::create_dir_all(&output_dir).map_err(|e| WikiError::Io {
        path: output_dir.clone(),
        source: e,
    })?;

    // 10. Atomic, non-clobbering write.
    let slug = slugify(topic, config.wiki.max_topic_slug_len);
    let stamp = utc_stamp(now);
    write_document_atomic(&output_dir, &slug, &stamp, &rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cascade::Evidence;
    use crate::config::BrokerConfig;
    use crate::minky_client::MinkyClient;
    use crate::storage::Storage;
    use crate::synth_client::SynthClient;
    use std::net::TcpListener;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path as req_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ev(source_id: &str, snippet: &str, sim: f32) -> Evidence {
        Evidence {
            title: format!("title-{source_id}"),
            snippet: snippet.to_string(),
            source_id: source_id.to_string(),
            similarity: sim,
        }
    }

    // ---- slugify ------------------------------------------------------------

    #[test]
    fn slugify_empty_is_untitled() {
        assert_eq!(slugify("", 64), "untitled");
        assert_eq!(slugify("   ", 64), "untitled");
        assert_eq!(slugify("///", 64), "untitled");
    }

    #[test]
    fn slugify_strips_forbidden_chars() {
        assert_eq!(slugify("a/b:c*d?e\"f<g>h|i.j", 64), "a-b-c-d-e-f-g-h-i-j");
        assert_eq!(slugify("../etc/passwd", 64), "etc-passwd");
    }

    #[test]
    fn slugify_collapses_separators() {
        assert_eq!(slugify("hello   world", 64), "hello-world");
        assert_eq!(slugify("a---b___c", 64), "a-b-c");
        assert_eq!(slugify("  spaced  out  ", 64), "spaced-out");
    }

    #[test]
    fn slugify_truncates_by_chars() {
        let long = "a".repeat(100);
        assert_eq!(slugify(&long, 64).chars().count(), 64);
        // Multibyte truncation must land on a char boundary (no panic).
        let hangul = "가".repeat(100);
        let out = slugify(&hangul, 10);
        assert_eq!(out.chars().count(), 10);
    }

    #[test]
    fn slugify_passes_through_clean_lowercase() {
        assert_eq!(slugify("rust-ownership", 64), "rust-ownership");
    }

    // ---- timestamp / filename ----------------------------------------------

    #[test]
    fn utc_stamp_known_epoch() {
        // 0 -> 1970-01-01T00:00:00Z
        assert_eq!(utc_stamp(0), "19700101T000000Z");
        assert_eq!(utc_iso8601(0), "1970-01-01T00:00:00Z");
        // 1_700_000_000 -> 2023-11-14T22:13:20Z
        assert_eq!(utc_stamp(1_700_000_000), "20231114T221320Z");
        assert_eq!(utc_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn wiki_filename_format() {
        assert_eq!(
            wiki_filename("rust-ownership", "20231114T221320Z"),
            "rust-ownership-20231114T221320Z.md"
        );
    }

    #[test]
    fn resolve_unique_path_disambiguates_same_second() {
        let dir = Path::new("/wiki");
        let slug = "topic";
        let stamp = "20231114T221320Z";
        let base = dir.join("topic-20231114T221320Z.md");
        let first_suffix = dir.join("topic-20231114T221320Z-1.md");

        // Nothing exists -> base name.
        let got = resolve_unique_path(dir, slug, stamp, |_| false).unwrap();
        assert_eq!(got, base);

        // Base exists -> first suffixed name, which differs from base.
        let got = resolve_unique_path(dir, slug, stamp, |p| p == base).unwrap();
        assert_eq!(got, first_suffix);
        assert_ne!(got, base);

        // Everything exists -> None (give up after retries).
        let got = resolve_unique_path(dir, slug, stamp, |_| true);
        assert!(got.is_none());
    }

    // ---- render_markdown ----------------------------------------------------

    #[test]
    fn render_markdown_contains_all_required_elements() {
        let evidence = vec![ev("src-1", "first snippet", 0.81), ev("src-2", "second", 0.72)];
        let rendered = render_markdown(
            "Rust \"Ownership\"",
            "how does borrow checking work",
            "2023-11-14T22:13:20Z",
            "gpt-4o-mini",
            &evidence,
            "본문 문단입니다 [^1]. 두번째 문단 [^2].",
        );

        // Banner literal.
        assert!(rendered.contains("AI 자동생성"));
        // Front-matter fields (with escaped quotes in topic).
        assert!(rendered.contains("topic: \"Rust \\\"Ownership\\\"\""));
        assert!(rendered.contains("query: \"how does borrow checking work\""));
        assert!(rendered.contains("generated_at: \"2023-11-14T22:13:20Z\""));
        assert!(rendered.contains("generator: \"broker/generate_wiki\""));
        assert!(rendered.contains("synthesis_model: \"gpt-4o-mini\""));
        assert!(rendered.contains("verified: false"));
        // Title.
        assert!(rendered.contains("# Rust \"Ownership\""));
        // Sources header + footnote definitions.
        assert!(rendered.contains("## 출처"));
        assert!(rendered.contains("[^1]: `src-1`"));
        assert!(rendered.contains("[^2]: `src-2`"));
        assert!(rendered.contains("sim: 0.810"));
    }

    // ---- validate_rendered --------------------------------------------------

    #[test]
    fn validate_rejects_missing_banner() {
        let doc = "no banner here\n\n## 출처\n[^1]: `x`\n";
        let err = validate_rendered(doc).unwrap_err();
        assert!(matches!(
            err,
            WikiError::ValidationFailed(MissingElement::UnverifiedBanner)
        ));
    }

    #[test]
    fn validate_rejects_missing_citations() {
        let doc = "AI 자동생성 banner but no footnotes and no sources section\n";
        let err = validate_rendered(doc).unwrap_err();
        assert!(matches!(
            err,
            WikiError::ValidationFailed(MissingElement::ParagraphCitations)
        ));
    }

    #[test]
    fn validate_accepts_complete_document() {
        let doc = "> AI 자동생성 배너\n\n본문 [^1]\n\n## 출처\n[^1]: `x`\n";
        assert!(validate_rendered(doc).is_ok());
    }

    // ---- generate() orchestration ------------------------------------------

    fn write_key(dir: &TempDir) -> PathBuf {
        let p = dir.path().join(".minky-key");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "test-key").unwrap();
        p
    }

    /// Build a config whose output_dir/memory_root are isolated temp dirs.
    fn config_for(
        minky_base: &str,
        synth_enable: bool,
        memory_root: &Path,
        output_dir: &Path,
        key_path: PathBuf,
    ) -> BrokerConfig {
        let mut config = BrokerConfig::default();
        config.working_set.memory_root = memory_root.to_path_buf();
        config.working_set.always_include = Vec::new();
        config.minky.base_url = minky_base.to_string();
        config.minky.api_key_file = key_path;
        config.minky.timeout_secs = 2;
        config.synthesis.enable = synth_enable;
        config.synthesis.base_url = "http://127.0.0.1:1".to_string();
        config.synthesis.timeout_secs = 2;
        config.wiki.output_dir = output_dir.to_path_buf();
        config
    }

    fn dir_is_empty(dir: &Path) -> bool {
        match std::fs::read_dir(dir) {
            Ok(mut entries) => entries.next().is_none(),
            // Nonexistent dir counts as "no file written".
            Err(_) => true,
        }
    }

    #[tokio::test]
    async fn generate_refuses_when_synthesis_disabled() {
        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keydir = TempDir::new().unwrap();
        let key = write_key(&keydir);

        // minky base points at a closed port, but it must never be contacted.
        let config = config_for(
            "http://127.0.0.1:1",
            false,
            memory.path(),
            output.path(),
            key,
        );
        let mut storage = Storage::open_in_memory().unwrap();
        let minky = MinkyClient::new(&config.minky).unwrap();
        let synth = SynthClient::new(&config.synthesis);

        let err = generate(
            "query",
            "topic",
            &config,
            &mut storage,
            &minky,
            &synth,
            1_700_000_000,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, WikiError::SynthesisDisabled), "got {err:?}");
        assert!(dir_is_empty(output.path()), "no file must be written");
    }

    #[tokio::test]
    async fn generate_hard_refuses_when_minky_offline() {
        // A bound-then-dropped listener yields a guaranteed-closed port so the
        // cascade degrades to offline.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let minky_base = format!("http://127.0.0.1:{port}");

        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keydir = TempDir::new().unwrap();
        let key = write_key(&keydir);

        let config = config_for(&minky_base, true, memory.path(), output.path(), key);
        let mut storage = Storage::open_in_memory().unwrap();
        let minky = MinkyClient::new(&config.minky).unwrap();
        let synth = SynthClient::new(&config.synthesis);

        let err = generate(
            "kubernetes deploy",
            "topic",
            &config,
            &mut storage,
            &minky,
            &synth,
            1_700_000_000,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, WikiError::MinkyOffline), "got {err:?}");
        assert!(dir_is_empty(output.path()), "no file must be written when offline");
    }

    // -- observability: generate_wiki logs one structured event per attempt --

    fn captured(events: &std::sync::Mutex<Vec<String>>) -> Vec<String> {
        events.lock().unwrap().clone()
    }

    #[tokio::test]
    async fn logs_success_event() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(req_path("/api/search/semantic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": {
                    "results": [{
                        "document_id": "doc-1",
                        "chunk_id": null,
                        "chunk_text": "ownership notes",
                        "similarity": 0.9,
                        "document_title": "doc-1",
                    }],
                    "total": 1,
                    "query": "q",
                }
            })))
            .mount(&server)
            .await;
        let synth_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(req_path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "content": "body [^1]" } }]
            })))
            .mount(&synth_server)
            .await;

        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keydir = TempDir::new().unwrap();
        let key = write_key(&keydir);
        let mut config = config_for(&server.uri(), true, memory.path(), output.path(), key);
        config.synthesis.base_url = synth_server.uri();
        let mut storage = Storage::open_in_memory().unwrap();
        let minky = MinkyClient::new(&config.minky).unwrap();
        let synth = SynthClient::new(&config.synthesis);

        let events = std::sync::Mutex::new(Vec::new());
        let mut sink = |e: &GenerateWikiLogEvent<'_>| events.lock().unwrap().push(e.render_line());

        let result = generate_with_sink(
            "query",
            "topic",
            &config,
            &mut storage,
            &minky,
            &synth,
            1_700_000_000,
            &mut sink,
        )
        .await;
        assert!(result.is_ok(), "expected success, got {result:?}");

        let lines = captured(&events);
        assert_eq!(lines.len(), 1, "exactly one log event per attempt");
        assert!(lines[0].contains("outcome=\"success\""), "got: {}", lines[0]);
        assert!(lines[0].contains("query=\"query\""), "got: {}", lines[0]);
        assert!(lines[0].contains("topic=\"topic\""), "got: {}", lines[0]);
    }

    #[tokio::test]
    async fn logs_synthesis_disabled_event() {
        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keydir = TempDir::new().unwrap();
        let key = write_key(&keydir);
        let config = config_for("http://127.0.0.1:1", false, memory.path(), output.path(), key);
        let mut storage = Storage::open_in_memory().unwrap();
        let minky = MinkyClient::new(&config.minky).unwrap();
        let synth = SynthClient::new(&config.synthesis);

        let events = std::sync::Mutex::new(Vec::new());
        let mut sink = |e: &GenerateWikiLogEvent<'_>| events.lock().unwrap().push(e.render_line());

        let result = generate_with_sink(
            "query",
            "topic",
            &config,
            &mut storage,
            &minky,
            &synth,
            1_700_000_000,
            &mut sink,
        )
        .await;
        assert!(result.is_err());

        let lines = captured(&events);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("outcome=\"synthesis_disabled\""), "got: {}", lines[0]);
    }

    #[tokio::test]
    async fn logs_minky_offline_event() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let minky_base = format!("http://127.0.0.1:{port}");

        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keydir = TempDir::new().unwrap();
        let key = write_key(&keydir);
        let config = config_for(&minky_base, true, memory.path(), output.path(), key);
        let mut storage = Storage::open_in_memory().unwrap();
        let minky = MinkyClient::new(&config.minky).unwrap();
        let synth = SynthClient::new(&config.synthesis);

        let events = std::sync::Mutex::new(Vec::new());
        let mut sink = |e: &GenerateWikiLogEvent<'_>| events.lock().unwrap().push(e.render_line());

        let result = generate_with_sink(
            "kubernetes deploy",
            "topic",
            &config,
            &mut storage,
            &minky,
            &synth,
            1_700_000_000,
            &mut sink,
        )
        .await;
        assert!(result.is_err());

        let lines = captured(&events);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("outcome=\"minky_offline\""), "got: {}", lines[0]);
    }

    #[test]
    fn error_outcome_tag_covers_constructible_variants() {
        // Covers every WikiError variant that can be constructed directly in
        // a unit test (Cascade/Io wrap other error types and are exercised
        // indirectly by generate()'s integration tests instead).
        let cases: Vec<(WikiError, &str)> = vec![
            (WikiError::SynthesisDisabled, "synthesis_disabled"),
            (WikiError::MinkyOffline, "minky_offline"),
            (WikiError::SynthOffline("x".into()), "synth_offline"),
            (
                WikiError::SynthHttp { status: 500, body: "e".into() },
                "synth_http",
            ),
            (
                WikiError::ValidationFailed(MissingElement::UnverifiedBanner),
                "validation_failed",
            ),
            (WikiError::NoEvidence, "no_evidence"),
            (
                WikiError::UnsafeOutputDir(PathBuf::from("/x")),
                "unsafe_output_dir",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(error_outcome_tag(&err), expected, "mismatch for {err:?}");
        }
    }
}
