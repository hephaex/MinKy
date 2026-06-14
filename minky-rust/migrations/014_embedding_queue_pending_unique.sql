-- Add partial unique index on embedding_queue(document_id) WHERE status = 'pending'
-- Required for ON CONFLICT (document_id) WHERE status = 'pending' in queue_document()
CREATE UNIQUE INDEX IF NOT EXISTS idx_embedding_queue_document_pending
    ON embedding_queue(document_id)
    WHERE status = 'pending';
