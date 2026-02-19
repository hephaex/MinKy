use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

use crate::models::{
    AnalyzeTextRequest, AutocompleteRequest, CreateSynonymGroup, KeywordExtraction,
    KoreanSearchHit, KoreanSearchQuery, KoreanSuggestion, MorphemeAnalysis, NormalizationResult,
    SpellCheckResult, StopwordList, SynonymGroup,
};
use crate::services::KoreanService;
use crate::AppState;

/// Analyze Korean text morphemes
async fn analyze_text(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeTextRequest>,
) -> Result<Json<MorphemeAnalysis>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .analyze_text(request)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Search with Korean language features
async fn search(
    State(state): State<AppState>,
    Query(query): Query<KoreanSearchQuery>,
) -> Result<Json<Vec<KoreanSearchHit>>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .search(query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Autocomplete with Korean support
async fn autocomplete(
    State(state): State<AppState>,
    Query(request): Query<AutocompleteRequest>,
) -> Result<Json<Vec<KoreanSuggestion>>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .autocomplete(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Spell check Korean text
async fn spell_check(
    State(state): State<AppState>,
    Json(request): Json<SpellCheckRequest>,
) -> Result<Json<SpellCheckResult>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .spell_check(&request.text)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Extract keywords from Korean text
async fn extract_keywords(
    State(state): State<AppState>,
    Json(request): Json<ExtractKeywordsRequest>,
) -> Result<Json<KeywordExtraction>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());
    let limit = request.limit.unwrap_or(10);

    service
        .extract_keywords(&request.text, limit)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Normalize Korean text
async fn normalize_text(
    State(state): State<AppState>,
    Json(request): Json<NormalizeRequest>,
) -> Result<Json<NormalizationResult>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .normalize_text(&request.text)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get synonym groups
async fn get_synonyms(
    State(state): State<AppState>,
) -> Result<Json<Vec<SynonymGroup>>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .get_synonyms()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create synonym group
async fn create_synonym_group(
    State(state): State<AppState>,
    Json(create): Json<CreateSynonymGroup>,
) -> Result<Json<SynonymGroup>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());

    service
        .create_synonym_group(create)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get stopwords
async fn get_stopwords(
    State(state): State<AppState>,
) -> Result<Json<StopwordList>, (StatusCode, String)> {
    let service = KoreanService::new(state.db.clone());
    Ok(Json(service.get_stopwords()))
}

#[derive(Debug, serde::Deserialize)]
pub struct SpellCheckRequest {
    pub text: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ExtractKeywordsRequest {
    pub text: String,
    pub limit: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct NormalizeRequest {
    pub text: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/analyze", post(analyze_text))
        .route("/search", get(search))
        .route("/autocomplete", get(autocomplete))
        .route("/spell-check", post(spell_check))
        .route("/keywords", post(extract_keywords))
        .route("/normalize", post(normalize_text))
        .route("/synonyms", get(get_synonyms).post(create_synonym_group))
        .route("/stopwords", get(get_stopwords))
}
