use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Korean morpheme analysis result
#[derive(Debug, Serialize)]
pub struct MorphemeAnalysis {
    pub text: String,
    pub morphemes: Vec<Morpheme>,
    pub sentences: Vec<Sentence>,
}

/// Korean morpheme
#[derive(Debug, Clone, Serialize)]
pub struct Morpheme {
    pub surface: String,
    pub pos: String, // Part of speech
    pub pos_detail: Vec<String>,
    pub reading: Option<String>,
    pub start_offset: i32,
    pub end_offset: i32,
}

/// Sentence with analysis
#[derive(Debug, Clone, Serialize)]
pub struct Sentence {
    pub text: String,
    pub morphemes: Vec<Morpheme>,
    pub start_offset: i32,
    pub end_offset: i32,
}

/// Korean text analysis request
#[derive(Debug, Deserialize)]
pub struct AnalyzeTextRequest {
    pub text: String,
    pub options: Option<AnalysisOptions>,
}

/// Analysis options
#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisOptions {
    pub include_pos: Option<bool>,
    pub include_reading: Option<bool>,
    pub normalize: Option<bool>,
    pub split_sentences: Option<bool>,
}

/// Korean search query
#[derive(Debug, Deserialize)]
pub struct KoreanSearchQuery {
    pub q: String,
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub category_id: Option<i32>,
    pub search_mode: Option<KoreanSearchMode>,
    pub include_synonyms: Option<bool>,
    pub include_jamo: Option<bool>,
    pub typo_tolerance: Option<bool>,
}

/// Korean search mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KoreanSearchMode {
    #[default]
    Morpheme,
    Exact,
    Fuzzy,
    Chosung, // 초성 검색
    Jamo,    // 자모 검색
}

/// Korean search hit
#[derive(Debug, Serialize)]
pub struct KoreanSearchHit {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub content_snippet: String,
    pub score: f32,
    pub highlights: Vec<KoreanHighlight>,
    pub matched_morphemes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Korean highlight with morpheme info
#[derive(Debug, Clone, Serialize)]
pub struct KoreanHighlight {
    pub text: String,
    pub matched_term: String,
    pub match_type: MatchType,
}

/// Match type
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    Exact,
    Morpheme,
    Synonym,
    Chosung,
    Typo,
}

/// Korean autocomplete suggestion
#[derive(Debug, Serialize)]
pub struct KoreanSuggestion {
    pub text: String,
    pub completion: String,
    pub score: f32,
    pub match_type: MatchType,
    pub frequency: i64,
}

/// Autocomplete request
#[derive(Debug, Deserialize)]
pub struct AutocompleteRequest {
    pub prefix: String,
    pub limit: Option<i32>,
    pub include_chosung: Option<bool>,
}

/// Spell check result
#[derive(Debug, Serialize)]
pub struct SpellCheckResult {
    pub original: String,
    pub corrected: String,
    pub corrections: Vec<SpellCorrection>,
    pub has_errors: bool,
}

/// Spell correction
#[derive(Debug, Clone, Serialize)]
pub struct SpellCorrection {
    pub original: String,
    pub suggested: String,
    pub confidence: f32,
    pub start_offset: i32,
    pub end_offset: i32,
}

/// Synonym group
#[derive(Debug, Serialize)]
pub struct SynonymGroup {
    pub id: i32,
    pub name: String,
    pub synonyms: Vec<String>,
    pub is_active: bool,
}

/// Create synonym group request
#[derive(Debug, Deserialize)]
pub struct CreateSynonymGroup {
    pub name: String,
    pub synonyms: Vec<String>,
}

/// Stopword list
#[derive(Debug, Serialize)]
pub struct StopwordList {
    pub words: Vec<String>,
    pub total: i64,
}

/// Korean keyword extraction
#[derive(Debug, Serialize)]
pub struct KeywordExtraction {
    pub keywords: Vec<ExtractedKeyword>,
    pub noun_phrases: Vec<String>,
}

/// Extracted keyword
#[derive(Debug, Clone, Serialize)]
pub struct ExtractedKeyword {
    pub keyword: String,
    pub score: f32,
    pub frequency: i32,
    pub pos: String,
}

/// Text normalization result
#[derive(Debug, Serialize)]
pub struct NormalizationResult {
    pub original: String,
    pub normalized: String,
    pub changes: Vec<NormalizationChange>,
}

/// Normalization change
#[derive(Debug, Clone, Serialize)]
pub struct NormalizationChange {
    pub original: String,
    pub normalized: String,
    pub change_type: NormalizationType,
}

/// Normalization type
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizationType {
    Spacing,
    Spelling,
    Abbreviation,
    NumberFormat,
    Punctuation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_korean_search_mode_default_is_morpheme() {
        assert!(matches!(KoreanSearchMode::default(), KoreanSearchMode::Morpheme));
    }

    #[test]
    fn test_morpheme_structure() {
        let morpheme = Morpheme {
            surface: "한글".to_string(),
            pos: "NNG".to_string(),
            pos_detail: vec!["일반명사".to_string()],
            reading: Some("한글".to_string()),
            start_offset: 0,
            end_offset: 2,
        };

        assert_eq!(morpheme.surface, "한글");
        assert_eq!(morpheme.pos, "NNG");
        assert_eq!(morpheme.start_offset, 0);
    }

    #[test]
    fn test_morpheme_analysis_structure() {
        let analysis = MorphemeAnalysis {
            text: "한글 분석".to_string(),
            morphemes: vec![
                Morpheme {
                    surface: "한글".to_string(),
                    pos: "NNG".to_string(),
                    pos_detail: vec![],
                    reading: None,
                    start_offset: 0,
                    end_offset: 2,
                },
            ],
            sentences: vec![],
        };

        assert_eq!(analysis.text, "한글 분석");
        assert_eq!(analysis.morphemes.len(), 1);
    }

    #[test]
    fn test_sentence_structure() {
        let sentence = Sentence {
            text: "샘플 문장입니다.".to_string(),
            morphemes: vec![],
            start_offset: 0,
            end_offset: 8,
        };

        assert_eq!(sentence.text, "샘플 문장입니다.");
        assert!(sentence.morphemes.is_empty());
    }

    #[test]
    fn test_analyze_text_request() {
        let req = AnalyzeTextRequest {
            text: "분석할 텍스트".to_string(),
            options: Some(AnalysisOptions {
                include_pos: Some(true),
                include_reading: None,
                normalize: Some(false),
                split_sentences: None,
            }),
        };

        assert_eq!(req.text, "분석할 텍스트");
        assert!(req.options.is_some());
    }

    #[test]
    fn test_korean_search_query() {
        let query = KoreanSearchQuery {
            q: "검색어".to_string(),
            page: Some(1),
            limit: Some(20),
            category_id: None,
            search_mode: Some(KoreanSearchMode::Morpheme),
            include_synonyms: Some(true),
            include_jamo: Some(false),
            typo_tolerance: Some(true),
        };

        assert_eq!(query.q, "검색어");
        assert_eq!(query.page, Some(1));
        assert_eq!(query.limit, Some(20));
    }

    #[test]
    fn test_korean_search_hit() {
        let hit = KoreanSearchHit {
            document_id: uuid::Uuid::nil(),
            title: "문서 제목".to_string(),
            content_snippet: "내용 요약".to_string(),
            score: 0.95,
            highlights: vec![],
            matched_morphemes: vec!["검색".to_string()],
            created_at: Utc::now(),
        };

        assert_eq!(hit.title, "문서 제목");
        assert!(hit.score > 0.9);
        assert_eq!(hit.matched_morphemes.len(), 1);
    }

    #[test]
    fn test_spell_check_result() {
        let result = SpellCheckResult {
            original: "맞춤법 착오".to_string(),
            corrected: "맞춤법 오류".to_string(),
            corrections: vec![SpellCorrection {
                original: "착오".to_string(),
                suggested: "오류".to_string(),
                confidence: 0.98,
                start_offset: 4,
                end_offset: 6,
            }],
            has_errors: true,
        };

        assert!(result.has_errors);
        assert_eq!(result.corrections.len(), 1);
    }

    #[test]
    fn test_synonym_group() {
        let group = SynonymGroup {
            id: 1,
            name: "컴퓨터".to_string(),
            synonyms: vec!["PC".to_string(), "컴".to_string()],
            is_active: true,
        };

        assert_eq!(group.name, "컴퓨터");
        assert_eq!(group.synonyms.len(), 2);
        assert!(group.is_active);
    }

    #[test]
    fn test_extracted_keyword() {
        let kw = ExtractedKeyword {
            keyword: "한글".to_string(),
            score: 0.87,
            frequency: 5,
            pos: "NNG".to_string(),
        };

        assert_eq!(kw.keyword, "한글");
        assert_eq!(kw.frequency, 5);
    }

    #[test]
    fn test_match_type_serialization() {
        let exact = MatchType::Exact;
        let morpheme = MatchType::Morpheme;

        assert!(matches!(exact, MatchType::Exact));
        assert!(matches!(morpheme, MatchType::Morpheme));
    }

    #[test]
    fn test_korean_search_modes() {
        let modes = vec![
            KoreanSearchMode::Morpheme,
            KoreanSearchMode::Exact,
            KoreanSearchMode::Fuzzy,
            KoreanSearchMode::Chosung,
            KoreanSearchMode::Jamo,
        ];

        assert_eq!(modes.len(), 5);
    }
}
