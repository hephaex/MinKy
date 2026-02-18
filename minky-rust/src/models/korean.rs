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
