use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Sync operation type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum SyncOperation {
    #[sqlx(rename = "INSERT")]
    Insert,
    #[sqlx(rename = "UPDATE")]
    Update,
    #[sqlx(rename = "DELETE")]
    Delete,
}

/// Sync change model representing a changeset in the sync system.
/// 
/// This struct maps to the `sync_changes` table and stores changesets
/// for offline-first synchronization with CRDT support.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncChange {
    /// Unique identifier for the sync change
    pub id: Uuid,
    
    /// ID of the user who owns this change
    pub user_id: Uuid,
    
    /// Name of the table that was changed
    pub table_name: String,
    
    /// ID of the record that was changed
    pub record_id: Uuid,
    
    /// Type of operation (INSERT, UPDATE, DELETE)
    pub operation: SyncOperation,
    
    /// Previous state (for UPDATE/DELETE)
    pub old_data: Option<Value>,
    
    /// New state (for INSERT/UPDATE)
    pub new_data: Option<Value>,
    
    /// Device/client identifier that made this change
    pub device_id: String,
    
    /// Timestamp when the change occurred
    pub change_timestamp: DateTime<Utc>,
    
    /// Vector clock at the time of change
    pub vector_clock: Option<Value>,
    
    /// Whether this change has been applied
    pub is_applied: bool,
    
    /// Whether this change conflicts with another
    pub is_conflict: bool,
    
    /// Conflict resolution strategy (if conflict occurred)
    pub conflict_resolution: Option<Value>,
    
    /// Monotonically increasing sequence number
    pub sequence_number: Option<i64>,
    
    /// Timestamp when the sync change was created
    pub created_at: DateTime<Utc>,
}

/// Sync change creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSyncChange {
    pub table_name: String,
    pub record_id: Uuid,
    pub operation: SyncOperation,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub device_id: String,
    pub vector_clock: Option<Value>,
}

/// Sync change response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChangeResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub table_name: String,
    pub record_id: Uuid,
    pub operation: SyncOperation,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub device_id: String,
    pub change_timestamp: DateTime<Utc>,
    pub vector_clock: Option<Value>,
    pub is_applied: bool,
    pub is_conflict: bool,
    pub conflict_resolution: Option<Value>,
    pub sequence_number: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl From<SyncChange> for SyncChangeResponse {
    fn from(change: SyncChange) -> Self {
        SyncChangeResponse {
            id: change.id,
            user_id: change.user_id,
            table_name: change.table_name,
            record_id: change.record_id,
            operation: change.operation,
            old_data: change.old_data,
            new_data: change.new_data,
            device_id: change.device_id,
            change_timestamp: change.change_timestamp,
            vector_clock: change.vector_clock,
            is_applied: change.is_applied,
            is_conflict: change.is_conflict,
            conflict_resolution: change.conflict_resolution,
            sequence_number: change.sequence_number,
            created_at: change.created_at,
        }
    }
}

