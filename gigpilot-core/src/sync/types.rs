use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::sync_change::SyncOperation;

/// Pull sync request from client.
/// 
/// WatermelonDB-compatible pull request that includes the last
/// synchronization timestamp to fetch incremental changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// Timestamp of the last successful pull (None for first sync)
    pub last_pulled_at: Option<DateTime<Utc>>,
    
    /// Optional device ID for tracking
    pub device_id: Option<String>,
}

/// Pull sync response to client.
/// 
/// Returns all changes that occurred after the last_pulled_at timestamp,
/// organized by table name for WatermelonDB compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResponse {
    /// Changes grouped by table name
    pub changes: Value, // { "invoices": { "created": [...], "updated": [...], "deleted": [...] } }
    
    /// Timestamp of this pull (for next sync)
    pub timestamp: DateTime<Utc>,
}

/// Single change record for push operations.
/// 
/// Represents a single record change (create/update/delete) from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushChange {
    /// Name of the table being changed
    pub table: String,
    
    /// ID of the record being changed
    pub id: Uuid,
    
    /// The record data (for INSERT/UPDATE)
    pub data: Option<Value>,
    
    /// Whether this is a deletion
    pub deleted: bool,
    
    /// Optional device ID that made this change
    pub device_id: Option<String>,
    
    /// Optional version vector for conflict detection
    pub version_vector: Option<Value>,
}

/// Push sync request from client.
/// 
/// Contains an array of changes to be applied on the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRequest {
    /// Array of changes to apply
    pub changes: Vec<PushChange>,
    
    /// Optional device ID
    pub device_id: Option<String>,
}

/// Push sync response to client.
/// 
/// Returns the result of applying changes, including any conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    /// Number of changes successfully applied
    pub applied: usize,
    
    /// Number of changes that conflicted
    pub conflicts: usize,
    
    /// Array of conflicted change IDs
    pub conflicted_ids: Vec<Uuid>,
    
    /// Timestamp of this push
    pub timestamp: DateTime<Utc>,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Server version wins (default)
    ServerWins,
    
    /// Last write wins (based on timestamp)
    LastWriteWins,
    
    /// Client version wins
    ClientWins,
}

/// Internal representation of a change to be applied.
#[derive(Debug, Clone)]
pub struct ChangeToApply {
    pub table_name: String,
    pub record_id: Uuid,
    pub operation: SyncOperation,
    pub data: Option<Value>,
    pub device_id: String,
    pub version_vector: Option<Value>,
}

