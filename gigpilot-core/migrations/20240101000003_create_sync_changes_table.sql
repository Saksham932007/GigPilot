-- Migration: Create sync_changes table for CRDT/Sync logic
-- This table stores changesets for offline-first synchronization

CREATE TABLE sync_changes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Change metadata
    table_name VARCHAR(100) NOT NULL, -- 'invoices', 'users', etc.
    record_id UUID NOT NULL, -- ID of the changed record
    operation VARCHAR(20) NOT NULL, -- 'INSERT', 'UPDATE', 'DELETE'
    
    -- Change data
    old_data JSONB, -- Previous state (for UPDATE/DELETE)
    new_data JSONB, -- New state (for INSERT/UPDATE)
    
    -- Sync metadata
    device_id VARCHAR(255) NOT NULL, -- Identifier for the device/client
    change_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    vector_clock JSONB, -- Vector clock at time of change
    
    -- Sync status
    is_applied BOOLEAN NOT NULL DEFAULT false, -- Whether change has been applied
    is_conflict BOOLEAN NOT NULL DEFAULT false, -- Whether this change conflicts
    conflict_resolution JSONB, -- Resolution strategy if conflict occurred
    
    -- Ordering
    sequence_number BIGSERIAL, -- Monotonically increasing sequence number
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for sync operations
CREATE INDEX idx_sync_changes_user_id ON sync_changes(user_id);
CREATE INDEX idx_sync_changes_table_record ON sync_changes(table_name, record_id);
CREATE INDEX idx_sync_changes_device ON sync_changes(device_id);
CREATE INDEX idx_sync_changes_timestamp ON sync_changes(change_timestamp DESC);
CREATE INDEX idx_sync_changes_unapplied ON sync_changes(user_id, is_applied) WHERE is_applied = false;
CREATE INDEX idx_sync_changes_sequence ON sync_changes(user_id, sequence_number);

-- GIN indexes for JSONB queries
CREATE INDEX idx_sync_changes_vector_clock ON sync_changes USING GIN (vector_clock);
CREATE INDEX idx_sync_changes_old_data ON sync_changes USING GIN (old_data);
CREATE INDEX idx_sync_changes_new_data ON sync_changes USING GIN (new_data);

-- Row Level Security: Enable RLS
ALTER TABLE sync_changes ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only view their own sync changes
CREATE POLICY sync_changes_select_own ON sync_changes
    FOR SELECT
    USING (user_id = auth.uid());

-- RLS Policy: Users can insert their own sync changes
CREATE POLICY sync_changes_insert_own ON sync_changes
    FOR INSERT
    WITH CHECK (user_id = auth.uid());

-- RLS Policy: Users can update their own sync changes (for marking as applied)
CREATE POLICY sync_changes_update_own ON sync_changes
    FOR UPDATE
    USING (user_id = auth.uid());

-- Composite index for efficient sync queries
-- Used to fetch unapplied changes for a user, ordered by sequence
CREATE INDEX idx_sync_changes_user_unapplied_sequence 
    ON sync_changes(user_id, sequence_number) 
    WHERE is_applied = false;

