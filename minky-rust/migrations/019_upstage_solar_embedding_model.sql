-- 019: Add Upstage Solar embedding model variants and tags.user_id default
-- solar-open2 embedding-passage (indexing) and embedding-query (search), both 4096 dims.

DO $$
BEGIN
    ALTER TYPE embedding_model ADD VALUE IF NOT EXISTS 'upstage_solar_passage';
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    ALTER TYPE embedding_model ADD VALUE IF NOT EXISTS 'upstage_solar_query';
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Allow auto-generated tags (user_id defaults to 1 for system-generated tags)
ALTER TABLE tags ALTER COLUMN user_id SET DEFAULT 1;
