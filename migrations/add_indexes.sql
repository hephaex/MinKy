-- Add indexes for better search performance
-- This file should be run manually after database migration

-- Index for full-text search on title and content
CREATE INDEX IF NOT EXISTS idx_documents_search 
ON documents USING gin(to_tsvector('english', title || ' ' || markdown_content));

-- Index for user documents
CREATE INDEX IF NOT EXISTS idx_documents_user_id ON documents(user_id);

-- Index for public documents
CREATE INDEX IF NOT EXISTS idx_documents_public ON documents(is_public);

-- Index for document timestamps
CREATE INDEX IF NOT EXISTS idx_documents_updated_at ON documents(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- Composite index for user documents with visibility
CREATE INDEX IF NOT EXISTS idx_documents_user_public ON documents(user_id, is_public);

-- Index for users
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);