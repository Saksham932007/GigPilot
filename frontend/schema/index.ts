import { appSchema, tableSchema } from '@nozbe/watermelondb';

/**
 * WatermelonDB Schema Definition
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
        { name: 'id', type: 'string', isIndexed: true },
        { name: 'email', type: 'string', isIndexed: true },
        { name: 'full_name', type: 'string', isOptional: true },
        { name: 'created_at', type: 'number' },
        { name: 'updated_at', type: 'number' },
        { name: 'last_login_at', type: 'number', isOptional: true },
        { name: 'is_active', type: 'boolean' },
        // Sync metadata
        { name: '_status', type: 'string', isOptional: true },
        { name: '_changed', type: 'string', isOptional: true },
      ],
    }),

    // Invoices table
    tableSchema({
      name: 'invoices',
      columns: [
        { name: 'id', type: 'string', isIndexed: true },
        { name: 'user_id', type: 'string', isIndexed: true },
        { name: 'invoice_number', type: 'string', isIndexed: true },
        { name: 'client_name', type: 'string' },
        { name: 'client_email', type: 'string', isOptional: true },
        { name: 'amount', type: 'string' }, // Decimal as string
        { name: 'currency', type: 'string' },
        { name: 'status', type: 'string' }, // draft, sent, paid, overdue, cancelled
        { name: 'due_date', type: 'number', isOptional: true }, // Unix timestamp
        { name: 'issue_date', type: 'number' }, // Unix timestamp
        { name: 'description', type: 'string', isOptional: true },
        { name: 'line_items', type: 'string', isOptional: true }, // JSON string
        { name: 'metadata', type: 'string', isOptional: true }, // JSON string (includes chase_state)
        // Sync metadata
        { name: 'last_modified', type: 'number' },
        { name: 'version_vector', type: 'string', isOptional: true }, // JSON string
        { name: 'is_deleted', type: 'boolean' },
        { name: '_status', type: 'string', isOptional: true },
        { name: '_changed', type: 'string', isOptional: true },
      ],
    }),

    // Sync changes table (for local tracking)
    tableSchema({
      name: 'sync_changes',
      columns: [
        { name: 'id', type: 'string', isIndexed: true },
        { name: 'user_id', type: 'string', isIndexed: true },
        { name: 'table_name', type: 'string', isIndexed: true },
        { name: 'record_id', type: 'string', isIndexed: true },
        { name: 'operation', type: 'string' }, // INSERT, UPDATE, DELETE
        { name: 'old_data', type: 'string', isOptional: true }, // JSON string
        { name: 'new_data', type: 'string', isOptional: true }, // JSON string
        { name: 'device_id', type: 'string' },
        { name: 'change_timestamp', type: 'number' },
        { name: 'vector_clock', type: 'string', isOptional: true }, // JSON string
        { name: 'is_applied', type: 'boolean' },
        { name: 'is_conflict', type: 'boolean' },
        { name: 'conflict_resolution', type: 'string', isOptional: true }, // JSON string
        { name: 'sequence_number', type: 'number', isOptional: true },
        { name: 'created_at', type: 'number' },
        { name: '_status', type: 'string', isOptional: true },
        { name: '_changed', type: 'string', isOptional: true },
      ],
    }),
  ],
});

