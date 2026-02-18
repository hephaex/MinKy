-- pgvector extension and embeddings tables
-- This migration adds vector search capabilities for RAG

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Embedding models enum
CREATE TYPE embedding_model AS ENUM (
    'openai_text_embedding_3_small',
    'openai_text_embedding_3_large',
    'voyage_large_2',
    'voyage_code_2'
);

-- Document embeddings table (document-level)
CREATE TABLE IF NOT EXISTS document_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    embedding vector(1536),  -- OpenAI text-embedding-3-small dimension
    model embedding_model NOT NULL DEFAULT 'openai_text_embedding_3_small',
    token_count INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(document_id, model)
);

-- Chunk embeddings table (chunk-level for RAG)
CREATE TABLE IF NOT EXISTS chunk_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    chunk_start_offset INTEGER NOT NULL,
    chunk_end_offset INTEGER NOT NULL,
    embedding vector(1536),
    model embedding_model NOT NULL DEFAULT 'openai_text_embedding_3_small',
    token_count INTEGER,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(document_id, chunk_index, model)
);

-- Document understanding table (AI analysis results)
CREATE TABLE IF NOT EXISTS document_understanding (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE UNIQUE,
    topics TEXT[] NOT NULL DEFAULT '{}',
    summary TEXT,
    problem_solved TEXT,
    insights TEXT[] DEFAULT '{}',
    technologies TEXT[] DEFAULT '{}',
    relevant_for TEXT[] DEFAULT '{}',
    related_document_ids UUID[] DEFAULT '{}',
    analyzed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    analyzer_model VARCHAR(100) DEFAULT 'claude-3-sonnet'
);

-- Embedding queue table (for async processing)
CREATE TABLE IF NOT EXISTS embedding_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, processing, completed, failed
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes for vector similarity search
-- Using IVFFlat index for approximate nearest neighbor search
CREATE INDEX idx_document_embeddings_vector ON document_embeddings
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_chunk_embeddings_vector ON chunk_embeddings
    USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

-- Additional indexes
CREATE INDEX idx_document_embeddings_document_id ON document_embeddings(document_id);
CREATE INDEX idx_document_embeddings_model ON document_embeddings(model);

CREATE INDEX idx_chunk_embeddings_document_id ON chunk_embeddings(document_id);
CREATE INDEX idx_chunk_embeddings_model ON chunk_embeddings(model);
CREATE INDEX idx_chunk_embeddings_chunk_index ON chunk_embeddings(chunk_index);

CREATE INDEX idx_document_understanding_document_id ON document_understanding(document_id);
CREATE INDEX idx_document_understanding_topics ON document_understanding USING gin(topics);
CREATE INDEX idx_document_understanding_technologies ON document_understanding USING gin(technologies);

CREATE INDEX idx_embedding_queue_status ON embedding_queue(status);
CREATE INDEX idx_embedding_queue_priority ON embedding_queue(priority DESC, created_at ASC);
CREATE INDEX idx_embedding_queue_document_id ON embedding_queue(document_id);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_embedding_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for document_embeddings
CREATE TRIGGER trigger_document_embeddings_updated_at
    BEFORE UPDATE ON document_embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_embedding_updated_at();

-- Comments for documentation
COMMENT ON TABLE document_embeddings IS 'Stores document-level vector embeddings for semantic search';
COMMENT ON TABLE chunk_embeddings IS 'Stores chunk-level embeddings for RAG retrieval';
COMMENT ON TABLE document_understanding IS 'Stores AI-analyzed understanding of documents';
COMMENT ON TABLE embedding_queue IS 'Queue for async embedding generation';
COMMENT ON COLUMN document_embeddings.embedding IS 'Vector embedding (1536 dimensions for OpenAI)';
COMMENT ON COLUMN chunk_embeddings.chunk_text IS 'The actual text content of the chunk';
