-- Migration 005: Slack / Teams platform integration tables
--
-- Tables:
--   platform_configs    – OAuth credentials per workspace
--   platform_messages   – captured messages from connected workspaces
--   extraction_jobs     – knowledge extraction task lifecycle
--   extracted_knowledge – persisted extraction results

-- Enable pgvector if not already present (idempotent)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---------------------------------------------------------------------------
-- platform_configs
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS platform_configs (
    id              SERIAL PRIMARY KEY,
    platform        VARCHAR(20)  NOT NULL CHECK (platform IN ('slack', 'teams', 'discord')),
    workspace_id    VARCHAR(128) NOT NULL,
    workspace_name  VARCHAR(256) NOT NULL,
    -- Store encrypted/obfuscated token – application layer handles decryption
    bot_token       TEXT         NOT NULL,
    is_active       BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    UNIQUE (platform, workspace_id)
);

CREATE INDEX IF NOT EXISTS idx_platform_configs_platform
    ON platform_configs (platform);

-- ---------------------------------------------------------------------------
-- platform_messages
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS platform_messages (
    id              VARCHAR(128) NOT NULL,
    platform        VARCHAR(20)  NOT NULL CHECK (platform IN ('slack', 'teams', 'discord')),
    workspace_id    VARCHAR(128) NOT NULL,
    channel_id      VARCHAR(128) NOT NULL,
    channel_name    VARCHAR(256),
    user_id         VARCHAR(128) NOT NULL,
    username        VARCHAR(256),
    text            TEXT         NOT NULL DEFAULT '',
    thread_ts       VARCHAR(64),
    reply_count     INTEGER      NOT NULL DEFAULT 0,
    reactions       JSONB        NOT NULL DEFAULT '[]',
    attachments     JSONB        NOT NULL DEFAULT '[]',
    posted_at       TIMESTAMPTZ  NOT NULL,
    captured_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    PRIMARY KEY (platform, id)
);

CREATE INDEX IF NOT EXISTS idx_platform_messages_channel
    ON platform_messages (platform, channel_id, posted_at DESC);

CREATE INDEX IF NOT EXISTS idx_platform_messages_thread
    ON platform_messages (platform, thread_ts, posted_at)
    WHERE thread_ts IS NOT NULL;

-- ---------------------------------------------------------------------------
-- extraction_jobs
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS extraction_jobs (
    id               UUID         NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    conversation_id  VARCHAR(256) NOT NULL,
    platform         VARCHAR(20)  NOT NULL CHECK (platform IN ('slack', 'teams', 'discord')),
    channel_id       VARCHAR(128) NOT NULL,
    message_count    INTEGER      NOT NULL DEFAULT 0,
    status           VARCHAR(20)  NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'skipped')),
    error_message    TEXT,
    started_at       TIMESTAMPTZ,
    finished_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_extraction_jobs_status
    ON extraction_jobs (status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_extraction_jobs_conversation
    ON extraction_jobs (conversation_id);

-- ---------------------------------------------------------------------------
-- extracted_knowledge
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS extracted_knowledge (
    id               UUID         NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    job_id           UUID         NOT NULL REFERENCES extraction_jobs (id) ON DELETE CASCADE,
    conversation_id  VARCHAR(256) NOT NULL,
    platform         VARCHAR(20)  NOT NULL CHECK (platform IN ('slack', 'teams', 'discord')),
    title            TEXT         NOT NULL DEFAULT '',
    summary          TEXT         NOT NULL DEFAULT '',
    insights         JSONB        NOT NULL DEFAULT '[]',
    problem_solved   TEXT,
    technologies     JSONB        NOT NULL DEFAULT '[]',
    relevant_for     JSONB        NOT NULL DEFAULT '[]',
    confidence       REAL         NOT NULL DEFAULT 0.0 CHECK (confidence BETWEEN 0.0 AND 1.0),
    confirmed        BOOLEAN      NOT NULL DEFAULT FALSE,
    confirmed_by     INTEGER      REFERENCES users (id) ON DELETE SET NULL,
    confirmed_at     TIMESTAMPTZ,
    -- Markdown representation cached for quick display
    markdown_cache   TEXT,
    extracted_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_extracted_knowledge_confidence
    ON extracted_knowledge (confidence DESC)
    WHERE confirmed = FALSE;

CREATE INDEX IF NOT EXISTS idx_extracted_knowledge_confirmed
    ON extracted_knowledge (confirmed, extracted_at DESC);

CREATE INDEX IF NOT EXISTS idx_extracted_knowledge_platform
    ON extracted_knowledge (platform, extracted_at DESC);

-- ---------------------------------------------------------------------------
-- Trigger: auto-update updated_at columns
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'trg_platform_configs_updated_at'
    ) THEN
        CREATE TRIGGER trg_platform_configs_updated_at
            BEFORE UPDATE ON platform_configs
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'trg_extraction_jobs_updated_at'
    ) THEN
        CREATE TRIGGER trg_extraction_jobs_updated_at
            BEFORE UPDATE ON extraction_jobs
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'trg_extracted_knowledge_updated_at'
    ) THEN
        CREATE TRIGGER trg_extracted_knowledge_updated_at
            BEFORE UPDATE ON extracted_knowledge
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;
END;
$$;
