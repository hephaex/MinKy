-- 016: switch embedding vectors 1536 (OpenAI) -> 4096 (Strix Qwen3-Embedding-8B)
-- Existing 1536-dim embeddings are discarded and re-embedded at 4096 after deploy.
-- Note: pgvector ivfflat/hnsw indexes support <=2000 dims, so the ANN vector
-- index is dropped and NOT recreated; semantic search falls back to a sequential
-- scan (acceptable at this corpus size). btree indexes are unaffected.

DROP INDEX IF EXISTS idx_document_embeddings_vector;
DROP INDEX IF EXISTS idx_chunk_embeddings_vector;

TRUNCATE document_embeddings;
TRUNCATE chunk_embeddings;

ALTER TABLE document_embeddings ALTER COLUMN embedding TYPE vector(4096);
ALTER TABLE chunk_embeddings  ALTER COLUMN embedding TYPE vector(4096);
