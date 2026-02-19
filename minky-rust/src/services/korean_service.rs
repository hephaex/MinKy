use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::models::{
    AnalysisOptions, AnalyzeTextRequest, AutocompleteRequest, CreateSynonymGroup,
    ExtractedKeyword, KeywordExtraction, KoreanHighlight, KoreanSearchHit, KoreanSearchMode,
    KoreanSearchQuery, KoreanSuggestion, MatchType, Morpheme, MorphemeAnalysis,
    NormalizationChange, NormalizationResult, NormalizationType, Sentence, SpellCheckResult,
    SpellCorrection, StopwordList, SynonymGroup,
};

/// Korean language service
pub struct KoreanService {
    db: PgPool,
    stopwords: Vec<String>,
}

impl KoreanService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            stopwords: default_korean_stopwords(),
        }
    }

    /// Analyze Korean text morphemes
    pub fn analyze_text(&self, request: AnalyzeTextRequest) -> Result<MorphemeAnalysis> {
        let text = &request.text;
        let options = request.options.unwrap_or_default();

        // Simple tokenization (for production, use actual Korean NLP library)
        let tokens: Vec<&str> = text.split_whitespace().collect();

        let morphemes: Vec<Morpheme> = tokens
            .iter()
            .map(|token| {
                let start = text.find(token).unwrap_or(0) as i32;
                Morpheme {
                    surface: token.to_string(),
                    pos: guess_pos(token),
                    pos_detail: vec![],
                    reading: if options.include_reading.unwrap_or(false) {
                        Some(token.to_string())
                    } else {
                        None
                    },
                    start_offset: start,
                    end_offset: start + token.len() as i32,
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
            WHERE ($1::int IS NULL OR d.category_id = $1)
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
                WHERE chosung_index LIKE $1 || '%'
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
                WHERE title ILIKE $1 || '%'
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

    /// Extract keywords from Korean text
    pub fn extract_keywords(&self, text: &str, limit: i32) -> Result<KeywordExtraction> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut freq: HashMap<String, i32> = HashMap::new();

        for word in &words {
            // Skip stopwords
            if self.stopwords.contains(&word.to_lowercase()) {
                continue;
            }
            // Skip short words
            if word.chars().count() < 2 {
                continue;
            }
            *freq.entry(word.to_string()).or_insert(0) += 1;
        }

        let mut keywords: Vec<ExtractedKeyword> = freq
            .into_iter()
            .map(|(word, count)| ExtractedKeyword {
                keyword: word,
                score: count as f32 / words.len() as f32,
                frequency: count,
                pos: "NNG".to_string(), // Assume common noun
            })
            .collect();

        keywords.sort_by(|a, b| b.frequency.cmp(&a.frequency));
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
}
