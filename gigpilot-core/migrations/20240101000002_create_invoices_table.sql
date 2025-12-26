-- Migration: Create invoices table with version vectors for sync
-- This table supports offline-first sync with last_modified timestamps and version vectors

CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Invoice fields
    invoice_number VARCHAR(100) NOT NULL,
    client_name VARCHAR(255) NOT NULL,
    client_email VARCHAR(255),
    amount DECIMAL(15, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    status VARCHAR(50) NOT NULL DEFAULT 'draft', -- draft, sent, paid, overdue, cancelled
    due_date DATE,
    issue_date DATE NOT NULL DEFAULT CURRENT_DATE,
    
    -- Sync metadata (CRDT support)
    last_modified TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version_vector JSONB, -- Stores vector clock: {"device_id": timestamp}
    is_deleted BOOLEAN NOT NULL DEFAULT false, -- Soft delete for sync
    
    -- Additional metadata
    description TEXT,
    line_items JSONB, -- Array of line items
    metadata JSONB, -- Flexible storage for additional data
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure unique invoice numbers per user
    UNIQUE(user_id, invoice_number)
);

-- Indexes for performance
CREATE INDEX idx_invoices_user_id ON invoices(user_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_last_modified ON invoices(last_modified DESC);
CREATE INDEX idx_invoices_user_status ON invoices(user_id, status);
CREATE INDEX idx_invoices_not_deleted ON invoices(user_id, is_deleted) WHERE is_deleted = false;

-- GIN index for JSONB queries
CREATE INDEX idx_invoices_version_vector ON invoices USING GIN (version_vector);
CREATE INDEX idx_invoices_line_items ON invoices USING GIN (line_items);

-- Row Level Security: Enable RLS
ALTER TABLE invoices ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only view their own invoices
CREATE POLICY invoices_select_own ON invoices
    FOR SELECT
    USING (user_id = auth.uid());

-- RLS Policy: Users can insert their own invoices
CREATE POLICY invoices_insert_own ON invoices
    FOR INSERT
    WITH CHECK (user_id = auth.uid());

-- RLS Policy: Users can update their own invoices
CREATE POLICY invoices_update_own ON invoices
    FOR UPDATE
    USING (user_id = auth.uid());

-- RLS Policy: Users can delete their own invoices (soft delete)
CREATE POLICY invoices_delete_own ON invoices
    FOR DELETE
    USING (user_id = auth.uid());

-- Trigger to auto-update updated_at and last_modified
CREATE TRIGGER update_invoices_timestamps
    BEFORE UPDATE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to update last_modified on invoice changes
CREATE OR REPLACE FUNCTION update_invoice_last_modified()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_modified = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_invoices_last_modified
    BEFORE UPDATE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION update_invoice_last_modified();

