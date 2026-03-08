-- Pipeline jobs table for tracking document processing
-- This migration adds the pipeline jobs table for async processing status

-- Pipeline jobs table
CREATE TABLE IF NOT EXISTS pipeline_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',  -- pending, running, completed, failed
    current_stage VARCHAR(100),  -- Current stage name (ingestion, parsing, chunking, etc.)
    stages_completed INTEGER NOT NULL DEFAULT 0,
    total_stages INTEGER NOT NULL DEFAULT 8,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    metrics JSONB DEFAULT '{}',  -- Stage timing, token counts, etc.
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for pipeline jobs
CREATE INDEX IF NOT EXISTS idx_pipeline_jobs_status ON pipeline_jobs(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_jobs_document_id ON pipeline_jobs(document_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_jobs_created_at ON pipeline_jobs(created_at DESC);

-- Comments
COMMENT ON TABLE pipeline_jobs IS 'Tracks document processing pipeline jobs';
COMMENT ON COLUMN pipeline_jobs.status IS 'Job status: pending, running, completed, failed';
COMMENT ON COLUMN pipeline_jobs.current_stage IS 'Current processing stage name';
COMMENT ON COLUMN pipeline_jobs.metrics IS 'JSON metrics including timing and counts';
