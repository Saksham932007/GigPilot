import { appSchema, tableSchema } from '@nozbe/watermelondb';

/**
 * WatermelonDB Schema for GigPilot
 * 
 * This schema matches the PostgreSQL database schema for offline-first sync.
 * All tables support CRDT-based synchronization with version vectors.
 */
export const schema = appSchema({
  version: 1,
  tables: [
    // Users table
    tableSchema({
      name: 'users',
      columns: [
        { name: 'email', type: 'string', isIndexed: true },
        { name: 'password_hash', type: 'string', isOptional: true }, // Not synced to client
        { name: 'full_name', type: 'string', isOptional: true },
        { name: 'created_at', type: 'number' },
        { name: 'updated_at', type: 'number' },
        { name: 'last_login_at', type: 'number', isOptional: true },
        { name: 'is_active', type: 'boolean' },
        // Sync metadata
        { name: 'last_modified', type: 'number' },
        { name: 'version_vector', type: 'string', isOptional: true }, // JSON string
        { name: 'is_deleted', type: 'boolean' },
      ],
    }),

    // Invoices table
    tableSchema({
      name: 'invoices',
      columns: [
        { name: 'user_id', type: 'string', isIndexed: true },
        { name: 'invoice_number', type: 'string', isIndexed: true },
        { name: 'client_name', type: 'string' },
        { name: 'client_email', type: 'string', isOptional: true },
        { name: 'amount', type: 'number' },
        { name: 'currency', type: 'string' },
        { name: 'status', type: 'string', isIndexed: true }, // draft, sent, paid, overdue, cancelled
        { name: 'due_date', type: 'number', isOptional: true }, // Unix timestamp
        { name: 'issue_date', type: 'number' },
        { name: 'description', type: 'string', isOptional: true },
        { name: 'line_items', type: 'string', isOptional: true }, // JSON string
        { name: 'metadata', type: 'string', isOptional: true }, // JSON string
        { name: 'created_at', type: 'number' },
        { name: 'updated_at', type: 'number' },
        // Sync metadata
        { name: 'last_modified', type: 'number', isIndexed: true },
        { name: 'version_vector', type: 'string', isOptional: true }, // JSON string
        { name: 'is_deleted', type: 'boolean', isIndexed: true },
      ],
    }),

    // Sync changes table (for tracking local changes)
    tableSchema({
      name: 'sync_changes',
      columns: [
        { name: 'user_id', type: 'string', isIndexed: true },
        { name: 'table_name', type: 'string', isIndexed: true },
        { name: 'record_id', type: 'string', isIndexed: true },
        { name: 'operation', type: 'string' }, // INSERT, UPDATE, DELETE
        { name: 'old_data', type: 'string', isOptional: true }, // JSON string
        { name: 'new_data', type: 'string', isOptional: true }, // JSON string
        { name: 'device_id', type: 'string', isIndexed: true },
        { name: 'change_timestamp', type: 'number' },
        { name: 'vector_clock', type: 'string', isOptional: true }, // JSON string
        { name: 'is_applied', type: 'boolean', isIndexed: true },
        { name: 'is_conflict', type: 'boolean' },
        { name: 'conflict_resolution', type: 'string', isOptional: true }, // JSON string
        { name: 'sequence_number', type: 'number', isOptional: true },
        { name: 'created_at', type: 'number' },
      ],
    }),
  ],
});

