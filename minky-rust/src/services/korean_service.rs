use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::models::{
    AnalysisOptions, AnalyzeTextRequest, AutocompleteRequest, CreateSynonymGroup,
    ExtractedKeyword, KeywordExtraction, KoreanHighlight, KoreanSearchHit, KoreanSearchMode,
    KoreanSearchQuery, KoreanSuggestion, MatchType, Morpheme, MorphemeAnalysis,
    NormalizationChange, NormalizationResult, NormalizationType, Sentence, SpellCheckResult,
    SpellCorrection, StopwordList, SynonymGroup,
};

/// Shared handle to a real mecab-ko morphological tokenizer.
///
/// `mecab_ko::Tokenizer::tokenize` requires `&mut self` (it reuses an
/// internal Lattice buffer for performance), so the tokenizer is wrapped in
/// a blocking [`std::sync::Mutex`] shared across requests via [`Arc`],
/// mirroring the `fastembed`/`EmbeddingService::local_model` pattern used
/// elsewhere in this codebase. `None` means no dictionary was configured or
/// loading it failed at startup; callers fall back to the lightweight
/// whitespace heuristic rather than failing the request outright.
pub type SharedKoreanTokenizer = Arc<Mutex<mecab_ko::Tokenizer>>;

/// Load the mecab-ko tokenizer from `MECAB_KO_DICDIR` (a directory produced
/// by `mecab-ko-dict-builder`, e.g. `sys.dic.zst` + `matrix.bin.zst` +
/// `entries.bin`). Returns `None` when the env var is unset or loading
/// fails; callers must fall back gracefully rather than erroring, since
/// morphological analysis is a quality-of-search enhancement, not a
/// hard dependency.
pub fn load_shared_korean_tokenizer() -> Option<SharedKoreanTokenizer> {
    let dicdir = std::env::var("MECAB_KO_DICDIR").ok()?;
    match mecab_ko::Tokenizer::with_dict(&dicdir) {
        Ok(tokenizer) => {
            tracing::info!(dicdir = %dicdir, "mecab-ko tokenizer loaded");
            Some(Arc::new(Mutex::new(tokenizer)))
        }
        Err(e) => {
            tracing::error!(dicdir = %dicdir, error = %e, "failed to load mecab-ko dictionary; falling back to whitespace tokenization");
            None
        }
    }
}

/// Tokenize `text` into `(surface, pos)` pairs using `tokenizer` when
/// present, otherwise the whitespace+heuristic-POS fallback. Free function
/// (no `KoreanService`/DB dependency) so callers that only need
/// tokenization — e.g. `SearchService`'s query-side pre-tokenization — do
/// not need to construct a full `KoreanService`.
pub fn tokenize_with(tokenizer: Option<&SharedKoreanTokenizer>, text: &str) -> Vec<(String, String)> {
    if let Some(tokenizer) = tokenizer {
        match tokenizer.lock() {
            Ok(mut guard) => {
                return guard
                    .tokenize(text)
                    .into_iter()
                    .map(|t| (t.surface, t.pos))
                    .collect();
            }
            Err(e) => {
                tracing::error!(error = %e, "mecab-ko tokenizer mutex poisoned; falling back to whitespace tokenization");
            }
        }
    }
    text.split_whitespace()
        .map(|token| (token.to_string(), guess_pos(token)))
        .collect()
}

/// Pre-tokenize `text` for OpenSearch lexical indexing/querying: joins
/// mecab-ko morpheme surfaces (or the whitespace-heuristic fallback) with
/// single spaces. Free function counterpart of
/// [`KoreanService::tokenize_for_lexical_index`] — used directly by
/// `SearchService` so both the document-indexing side and the query side
/// stay symmetric against the whitespace-only OpenSearch analyzer without
/// requiring a `KoreanService`/DB handle.
pub fn tokenize_for_lexical_index_with(tokenizer: Option<&SharedKoreanTokenizer>, text: &str) -> String {
    tokenize_with(tokenizer, text)
        .into_iter()
        .map(|(surface, _pos)| surface)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Korean language service
pub struct KoreanService {
    db: PgPool,
    stopwords: Vec<String>,
    tokenizer: Option<SharedKoreanTokenizer>,
}

impl KoreanService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            stopwords: default_korean_stopwords(),
            tokenizer: None,
        }
    }

    /// Create a service that uses a shared real mecab-ko tokenizer for
    /// morphological analysis, falling back to the whitespace heuristic
    /// when `tokenizer` is `None`.
    pub fn with_tokenizer(db: PgPool, tokenizer: Option<SharedKoreanTokenizer>) -> Self {
        Self {
            db,
            stopwords: default_korean_stopwords(),
            tokenizer,
        }
    }

    /// Tokenize `text` into `(surface, pos)` pairs using the real mecab-ko
    /// tokenizer when available, otherwise the whitespace+heuristic fallback.
    fn tokenize(&self, text: &str) -> Vec<(String, String)> {
        tokenize_with(self.tokenizer.as_ref(), text)
    }

    /// Pre-tokenize `text` for OpenSearch lexical indexing: joins mecab-ko
    /// morpheme surfaces (or the whitespace-heuristic fallback) with single
    /// spaces, so a plain whitespace analyzer on the OpenSearch side can
    /// index real Korean morphemes without needing an OpenSearch-side
    /// Korean analysis plugin (mechanism B2 — see C2 plan: one shared
    /// tokenizer pipeline for both lexical indexing and, at the
    /// embedding-preprocessing step, pgvector embedding generation).
    pub fn tokenize_for_lexical_index(&self, text: &str) -> String {
        tokenize_for_lexical_index_with(self.tokenizer.as_ref(), text)
    }

    /// Analyze Korean text morphemes
    pub fn analyze_text(&self, request: AnalyzeTextRequest) -> Result<MorphemeAnalysis> {
        let text = &request.text;
        let options = request.options.unwrap_or_default();

        // Real mecab-ko morphological tokenization when a dictionary is
        // configured (MECAB_KO_DICDIR); falls back to whitespace splitting
        // with a heuristic POS guess otherwise.
        let tokens = self.tokenize(text);

        let morphemes: Vec<Morpheme> = tokens
            .iter()
            .map(|(surface, pos)| {
                let start = text.find(surface.as_str()).unwrap_or(0) as i32;
                Morpheme {
                    surface: surface.clone(),
                    pos: pos.clone(),
                    pos_detail: vec![],
                    reading: if options.include_reading.unwrap_or(false) {
                        Some(surface.clone())
                    } else {
                        None
                    },
                    start_offset: start,
                    end_offset: start + surface.len() as i32,
                }
            })
            .collect();

        // Split into sentences
        let sentences: Vec<Sentence> = if options.split_sentences.unwrap_or(true) {
            text.split(&['.', '!', '?', '。'][..])
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let start = text.find(s).unwrap_or(0) as i32;
                    Sentence {
                        text: s.trim().to_string(),
                        morphemes: vec![],
                        start_offset: start,
                        end_offset: start + s.len() as i32,
                    }
                })
                .collect()
        } else {
            vec![]
        };

        Ok(MorphemeAnalysis {
            text: text.clone(),
            morphemes,
            sentences,
        })
    }

    /// Search with Korean language features
    pub async fn search(&self, query: KoreanSearchQuery) -> Result<Vec<KoreanSearchHit>> {
        let mode = query.search_mode.clone().unwrap_or_default();
        let limit = query.limit.unwrap_or(20).min(100);

        // Convert query based on mode
        let search_query = match mode {
            KoreanSearchMode::Chosung => self.convert_to_chosung(&query.q),
            KoreanSearchMode::Jamo => self.decompose_jamo(&query.q),
            _ => query.q.clone(),
        };

        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            r#"
            SELECT d.id, d.title, LEFT(d.content, 200), d.created_at
            FROM documents d
            WHERE d.archived = false
              AND ($1::int IS NULL OR d.category_id = $1)
              AND (d.title ILIKE '%' || $2 || '%' OR d.content ILIKE '%' || $2 || '%')
            ORDER BY d.updated_at DESC
            LIMIT $3
            "#,
        )
        .bind(query.category_id)
        .bind(&search_query)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| KoreanSearchHit {
                document_id: r.0,
                title: r.1.clone(),
                content_snippet: r.2,
                score: 1.0,
                highlights: vec![KoreanHighlight {
                    text: r.1,
                    matched_term: search_query.clone(),
                    match_type: match mode {
                        KoreanSearchMode::Chosung => MatchType::Chosung,
                        KoreanSearchMode::Morpheme => MatchType::Morpheme,
                        _ => MatchType::Exact,
                    },
                }],
                matched_morphemes: vec![],
                created_at: r.3,
            })
            .collect())
    }

    /// Autocomplete with Korean support
    pub async fn autocomplete(&self, request: AutocompleteRequest) -> Result<Vec<KoreanSuggestion>> {
        let limit = request.limit.unwrap_or(10).min(20);

        // Check if prefix is chosung
        let is_chosung = request.include_chosung.unwrap_or(true) && is_chosung_only(&request.prefix);

        let rows: Vec<(String, i64)> = if is_chosung {
            // Search by chosung pattern
            sqlx::query_as(
                r#"
                SELECT DISTINCT title, COUNT(*) OVER () as freq
                FROM documents
                WHERE archived = false AND chosung_index LIKE $1 || '%'
                ORDER BY freq DESC
                LIMIT $2
                "#,
            )
            .bind(&request.prefix)
            .bind(limit)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT DISTINCT title, COUNT(*) OVER () as freq
                FROM documents
                WHERE archived = false AND title ILIKE $1 || '%'
                ORDER BY freq DESC
                LIMIT $2
                "#,
            )
            .bind(&request.prefix)
            .bind(limit)
            .fetch_all(&self.db)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| KoreanSuggestion {
                text: request.prefix.clone(),
                completion: r.0,
                score: 1.0,
                match_type: if is_chosung {
                    MatchType::Chosung
                } else {
                    MatchType::Exact
                },
                frequency: r.1,
            })
            .collect())
    }

    /// Spell check Korean text
    pub fn spell_check(&self, text: &str) -> Result<SpellCheckResult> {
        // Simple spell check (for production, use actual spell checker)
        let corrections: Vec<SpellCorrection> = vec![];
        let corrected = text.to_string();

        Ok(SpellCheckResult {
            original: text.to_string(),
            corrected,
            corrections,
            has_errors: false,
        })
    }

    /// Extract keywords from Korean text.
    ///
    /// Uses real mecab-ko tokenization (surface + POS) when a dictionary is
    /// configured, so `pos` reflects the tokenizer's actual tag rather than
    /// a hardcoded "NNG" guess; falls back to whitespace splitting with the
    /// existing heuristic POS guess otherwise.
    pub fn extract_keywords(&self, text: &str, limit: i32) -> Result<KeywordExtraction> {
        let tokens = self.tokenize(text);
        let total = tokens.len().max(1);
        let mut freq: HashMap<String, i32> = HashMap::new();
        let mut pos_by_word: HashMap<String, String> = HashMap::new();

        for (surface, pos) in &tokens {
            // Skip stopwords
            if self.stopwords.contains(&surface.to_lowercase()) {
                continue;
            }
            // Skip short words
            if surface.chars().count() < 2 {
                continue;
            }
            *freq.entry(surface.clone()).or_insert(0) += 1;
            pos_by_word.entry(surface.clone()).or_insert_with(|| pos.clone());
        }

        let mut keywords: Vec<ExtractedKeyword> = freq
            .into_iter()
            .map(|(word, count)| {
                let pos = pos_by_word.remove(&word).unwrap_or_else(|| "NNG".to_string());
                ExtractedKeyword {
                    keyword: word,
                    score: count as f32 / total as f32,
                    frequency: count,
                    pos,
                }
            })
            .collect();

        keywords.sort_by_key(|b| std::cmp::Reverse(b.frequency));
        keywords.truncate(limit as usize);

        Ok(KeywordExtraction {
            keywords,
            noun_phrases: vec![],
        })
    }

    /// Normalize Korean text
    pub fn normalize_text(&self, text: &str) -> Result<NormalizationResult> {
        let mut normalized = text.to_string();
        let mut changes = Vec::new();

        // Normalize spaces
        let re_spaces = regex::Regex::new(r"\s+").unwrap();
        if re_spaces.is_match(&normalized) {
            let new_text = re_spaces.replace_all(&normalized, " ").to_string();
            if new_text != normalized {
                changes.push(NormalizationChange {
                    original: normalized.clone(),
                    normalized: new_text.clone(),
                    change_type: NormalizationType::Spacing,
                });
                normalized = new_text;
            }
        }

        Ok(NormalizationResult {
            original: text.to_string(),
            normalized,
            changes,
        })
    }

    /// Get synonym groups
    pub async fn get_synonyms(&self) -> Result<Vec<SynonymGroup>> {
        let rows: Vec<(i32, String, Vec<String>, bool)> = sqlx::query_as(
            r#"
            SELECT id, name, synonyms, is_active
            FROM synonym_groups
            ORDER BY name
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SynonymGroup {
                id: r.0,
                name: r.1,
                synonyms: r.2,
                is_active: r.3,
            })
            .collect())
    }

    /// Create synonym group
    pub async fn create_synonym_group(&self, create: CreateSynonymGroup) -> Result<SynonymGroup> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO synonym_groups (name, synonyms, is_active) VALUES ($1, $2, true) RETURNING id",
        )
        .bind(&create.name)
        .bind(&create.synonyms)
        .fetch_one(&self.db)
        .await?;

        Ok(SynonymGroup {
            id: row.0,
            name: create.name,
            synonyms: create.synonyms,
            is_active: true,
        })
    }

    /// Get stopwords
    pub fn get_stopwords(&self) -> StopwordList {
        StopwordList {
            words: self.stopwords.clone(),
            total: self.stopwords.len() as i64,
        }
    }

    /// Convert text to chosung (initial consonants)
    fn convert_to_chosung(&self, text: &str) -> String {
        const CHOSUNG: [char; 19] = [
            'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ',
            'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
        ];

        text.chars()
            .map(|c| {
                if ('가'..='힣').contains(&c) {
                    let code = c as u32 - '가' as u32;
                    let cho_idx = (code / 588) as usize;
                    CHOSUNG[cho_idx]
                } else {
                    c
                }
            })
            .collect()
    }

    /// Decompose Korean to jamo
    fn decompose_jamo(&self, text: &str) -> String {
        // Simplified jamo decomposition
        text.to_string()
    }
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            include_pos: Some(true),
            include_reading: Some(false),
            normalize: Some(true),
            split_sentences: Some(true),
        }
    }
}

/// Check if string contains only chosung
fn is_chosung_only(text: &str) -> bool {
    text.chars().all(|c| {
        matches!(
            c,
            'ㄱ' | 'ㄲ'
                | 'ㄴ'
                | 'ㄷ'
                | 'ㄸ'
                | 'ㄹ'
                | 'ㅁ'
                | 'ㅂ'
                | 'ㅃ'
                | 'ㅅ'
                | 'ㅆ'
                | 'ㅇ'
                | 'ㅈ'
                | 'ㅉ'
                | 'ㅊ'
                | 'ㅋ'
                | 'ㅌ'
                | 'ㅍ'
                | 'ㅎ'
        )
    })
}

/// Guess part of speech (simplified)
fn guess_pos(token: &str) -> String {
    // Very simplified POS guessing
    if token.ends_with("다") || token.ends_with("요") {
        "VV".to_string() // Verb
    } else if token.ends_with("이") || token.ends_with("가") {
        "JKS".to_string() // Subject marker
    } else {
        "NNG".to_string() // Common noun
    }
}

/// Default Korean stopwords
fn default_korean_stopwords() -> Vec<String> {
    vec![
        "의", "가", "이", "은", "는", "를", "을", "에", "로", "으로", "와", "과", "도", "만", "에서",
        "까지", "부터", "에게", "한테", "께", "더", "덜", "가장", "매우", "정말", "그리고", "하지만",
        "그러나", "그래서", "따라서", "또는", "즉", "왜냐하면", "그런데", "하고", "이나", "거나",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_chosung_only_true() {
        assert!(is_chosung_only("ㄱㄴㄷ"));
        assert!(is_chosung_only("ㅎ"));
    }

    #[test]
    fn test_is_chosung_only_false() {
        assert!(!is_chosung_only("가나다"));
        assert!(!is_chosung_only("hello"));
        assert!(!is_chosung_only("ㄱ나"));
    }

    #[test]
    fn test_is_chosung_only_empty() {
        // all() on empty iterator returns true
        assert!(is_chosung_only(""));
    }

    #[test]
    fn test_guess_pos_verb_endings() {
        assert_eq!(guess_pos("먹다"), "VV");
        assert_eq!(guess_pos("가요"), "VV");
    }

    #[test]
    fn test_guess_pos_subject_marker() {
        assert_eq!(guess_pos("학생이"), "JKS");
        assert_eq!(guess_pos("고양이가"), "JKS");
    }

    #[test]
    fn test_guess_pos_default_noun() {
        assert_eq!(guess_pos("기술"), "NNG");
        assert_eq!(guess_pos("데이터"), "NNG");
    }

    #[test]
    fn test_default_korean_stopwords_not_empty() {
        let stopwords = default_korean_stopwords();
        assert!(!stopwords.is_empty());
    }

    #[test]
    fn test_default_korean_stopwords_contains_common_particles() {
        let stopwords = default_korean_stopwords();
        assert!(stopwords.contains(&"의".to_string()));
        assert!(stopwords.contains(&"는".to_string()));
        assert!(stopwords.contains(&"에".to_string()));
    }

    // --- extract_keywords (needs KoreanService instance) ---

    fn make_service() -> KoreanService {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        KoreanService::new(pool)
    }

    #[tokio::test]
    async fn test_extract_keywords_skips_stopwords() {
        let svc = make_service();
        // "의" and "는" are Korean stopwords and should be excluded
        let result = svc.extract_keywords("의 는 기술 데이터", 10).unwrap();
        let words: Vec<&str> = result.keywords.iter().map(|k| k.keyword.as_str()).collect();
        assert!(!words.contains(&"의"), "'의' is a stopword and should be excluded");
        assert!(!words.contains(&"는"), "'는' is a stopword and should be excluded");
    }

    #[tokio::test]
    async fn test_extract_keywords_returns_at_most_limit() {
        let svc = make_service();
        let text = "기술 데이터 분석 학습 모델 알고리즘 시스템 네트워크 코드 함수 변수 클래스";
        let result = svc.extract_keywords(text, 5).unwrap();
        assert!(result.keywords.len() <= 5, "Should return at most 5 keywords");
    }

    #[tokio::test]
    async fn test_extract_keywords_empty_text_returns_empty() {
        let svc = make_service();
        let result = svc.extract_keywords("", 10).unwrap();
        assert!(result.keywords.is_empty(), "Empty text should yield no keywords");
    }

    #[tokio::test]
    async fn test_extract_keywords_skips_single_char_words() {
        let svc = make_service();
        // Single-char words (1 char count) should be skipped (< 2 chars)
        let result = svc.extract_keywords("a b c 기술", 10).unwrap();
        let words: Vec<&str> = result.keywords.iter().map(|k| k.keyword.as_str()).collect();
        assert!(!words.contains(&"a"), "Single-char word should be excluded");
        assert!(!words.contains(&"b"), "Single-char word should be excluded");
    }

    // --- normalize_text ---

    #[tokio::test]
    async fn test_normalize_text_collapses_multiple_spaces() {
        let svc = make_service();
        let result = svc.normalize_text("hello   world").unwrap();
        assert_eq!(result.normalized, "hello world", "Multiple spaces should collapse to one");
    }

    #[tokio::test]
    async fn test_normalize_text_preserves_single_space() {
        let svc = make_service();
        let result = svc.normalize_text("hello world").unwrap();
        assert_eq!(result.normalized, "hello world");
        assert!(result.changes.is_empty(), "No change should be recorded if already normalized");
    }

    #[tokio::test]
    async fn test_normalize_text_records_change_when_modified() {
        let svc = make_service();
        let result = svc.normalize_text("a  b").unwrap();
        assert!(!result.changes.is_empty(), "A change should be recorded when spaces are collapsed");
    }

    #[tokio::test]
    async fn tokenize_without_tokenizer_falls_back_to_whitespace_heuristic() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = KoreanService::with_tokenizer(pool, None);
        let tokens = svc.tokenize("먹다 학생이 기술");
        assert_eq!(
            tokens,
            vec![
                ("먹다".to_string(), "VV".to_string()),
                ("학생이".to_string(), "JKS".to_string()),
                ("기술".to_string(), "NNG".to_string()),
            ]
        );
    }

    #[test]
    fn load_shared_korean_tokenizer_returns_none_when_env_var_unset() {
        // SAFETY: test-only env var manipulation, no other test reads MECAB_KO_DICDIR.
        unsafe {
            std::env::remove_var("MECAB_KO_DICDIR");
        }
        assert!(load_shared_korean_tokenizer().is_none());
    }

    /// Regression test for the real mecab-ko dictionary integration.
    ///
    /// Requires a dictionary built by `mecab-ko-dict-builder` from the
    /// classic eunjeon mecab-ko-dic 2.1.1 CSV source (NOT the Homebrew
    /// `mecab-ko-dic` binary package, which uses an incompatible on-disk
    /// format despite matching filenames — confirmed by direct testing).
    /// Skipped in CI; run with:
    ///   MECAB_KO_DICDIR=/path/to/dict-output cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn tokenize_with_real_dictionary_produces_correct_pos_tags() {
        let dicdir = std::env::var("MECAB_KO_DICDIR")
            .expect("MECAB_KO_DICDIR must be set to run this ignored test");
        let tokenizer = mecab_ko::Tokenizer::with_dict(&dicdir)
            .expect("failed to load mecab-ko dictionary from MECAB_KO_DICDIR");
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = KoreanService::with_tokenizer(pool, Some(Arc::new(Mutex::new(tokenizer))));

        let tokens = svc.tokenize("한국어 형태소 분석기입니다");
        let surfaces: Vec<&str> = tokens.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(surfaces, vec!["한국어", "형태소", "분석기", "입니다"]);
        assert_eq!(tokens[0].1, "NNG", "한국어 should tag as common noun");
        assert_eq!(tokens[1].1, "NNG", "형태소 should tag as common noun");
    }

    #[tokio::test]
    #[ignore]
    async fn extract_keywords_with_real_dictionary_uses_actual_pos_tags() {
        let dicdir = std::env::var("MECAB_KO_DICDIR")
            .expect("MECAB_KO_DICDIR must be set to run this ignored test");
        let tokenizer = mecab_ko::Tokenizer::with_dict(&dicdir)
            .expect("failed to load mecab-ko dictionary from MECAB_KO_DICDIR");
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let svc = KoreanService::with_tokenizer(pool, Some(Arc::new(Mutex::new(tokenizer))));

        let result = svc
            .extract_keywords("오늘 날씨가 정말 좋습니다 기술 발전이 빠릅니다", 10)
            .unwrap();
        let nng_keywords: Vec<&str> = result
            .keywords
            .iter()
            .filter(|k| k.pos == "NNG")
            .map(|k| k.keyword.as_str())
            .collect();
        assert!(
            nng_keywords.contains(&"기술") || nng_keywords.contains(&"날씨"),
            "expected at least one real NNG-tagged noun among {:?}",
            result.keywords
        );
    }
}
