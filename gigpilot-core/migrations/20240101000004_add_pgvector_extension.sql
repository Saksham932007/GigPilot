-- Migration: Add pgvector extension for vector similarity search
-- This enables storing and searching embeddings for the Contextual Estimator

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create embeddings table for storing project/invoice embeddings
CREATE TABLE embeddings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Embedding data
    text_content TEXT NOT NULL, -- Original text that was embedded
    embedding vector(1536), -- OpenAI ada-002 embedding dimension (1536)
    
    -- Metadata
    entity_type VARCHAR(50) NOT NULL, -- 'invoice', 'project', 'client', etc.
    entity_id UUID, -- ID of the related entity (invoice_id, project_id, etc.)
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for vector similarity search
CREATE INDEX idx_embeddings_vector ON embeddings 
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Index for user and entity lookups
CREATE INDEX idx_embeddings_user_id ON embeddings(user_id);
CREATE INDEX idx_embeddings_entity ON embeddings(entity_type, entity_id);

-- Row Level Security: Enable RLS
ALTER TABLE embeddings ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only view their own embeddings
CREATE POLICY embeddings_select_own ON embeddings
    FOR SELECT
    USING (user_id = auth.uid());

-- RLS Policy: Users can insert their own embeddings
CREATE POLICY embeddings_insert_own ON embeddings
    FOR INSERT
    WITH CHECK (user_id = auth.uid());

-- RLS Policy: Users can update their own embeddings
CREATE POLICY embeddings_update_own ON embeddings
    FOR UPDATE
    USING (user_id = auth.uid());

-- RLS Policy: Users can delete their own embeddings
CREATE POLICY embeddings_delete_own ON embeddings
    FOR DELETE
    USING (user_id = auth.uid());

-- Function to update updated_at timestamp
CREATE TRIGGER update_embeddings_updated_at
    BEFORE UPDATE ON embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
