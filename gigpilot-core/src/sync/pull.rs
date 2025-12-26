use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::models::sync_change::SyncChange;
use crate::sync::types::{PullRequest, PullResponse};

/// Retrieves changes from the database for pull synchronization.
/// 
/// This function implements the "Pull" part of the sync protocol, compatible
/// with WatermelonDB. It queries the sync_changes table for all changes
/// that occurred after the last_pulled_at timestamp.
/// 
/// # Arguments
/// 
/// * `pool` - PostgreSQL connection pool
/// * `user_id` - ID of the user requesting sync
/// * `request` - Pull request with last_pulled_at timestamp
/// 
/// # Returns
/// 
/// Returns a `Result<PullResponse>` containing changes grouped by table,
/// or an error if the query fails.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Database query fails
/// - JSON serialization fails
pub async fn get_changes(
    pool: &PgPool,
    user_id: Uuid,
    request: PullRequest,
) -> Result<PullResponse, anyhow::Error> {
    info!(
        "Pull sync requested for user {} with last_pulled_at: {:?}",
        user_id, request.last_pulled_at
    );
    
    // Query sync_changes table for changes after last_pulled_at
    let changes = if let Some(last_pulled) = request.last_pulled_at {
        // Incremental sync: get changes after last_pulled_at
        sqlx::query_as::<_, SyncChange>(
            r#"
            SELECT 
                id, user_id, table_name, record_id, operation,
                old_data, new_data, device_id, change_timestamp,
                vector_clock, is_applied, is_conflict, conflict_resolution,
                sequence_number, created_at
            FROM sync_changes
            WHERE user_id = $1
                AND change_timestamp > $2
                AND is_applied = true
            ORDER BY change_timestamp ASC, sequence_number ASC
            "#,
        )
        .bind(user_id)
        .bind(last_pulled)
        .fetch_all(pool)
        .await?
    } else {
        // Full sync: get all changes (for first sync)
        sqlx::query_as::<_, SyncChange>(
            r#"
            SELECT 
                id, user_id, table_name, record_id, operation,
                old_data, new_data, device_id, change_timestamp,
                vector_clock, is_applied, is_conflict, conflict_resolution,
                sequence_number, created_at
            FROM sync_changes
            WHERE user_id = $1
                AND is_applied = true
            ORDER BY change_timestamp ASC, sequence_number ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?
    };
    
    info!("Found {} changes for user {}", changes.len(), user_id);
    
    // Group changes by table name and operation type
    let mut changes_by_table: std::collections::HashMap<String, std::collections::HashMap<String, Vec<Value>>> = 
        std::collections::HashMap::new();
    
    for change in changes {
        let table_name = change.table_name.clone();
        let operation_type = match change.operation {
            crate::models::sync_change::SyncOperation::Insert => "created",
            crate::models::sync_change::SyncOperation::Update => "updated",
            crate::models::sync_change::SyncOperation::Delete => "deleted",
        };
        
        // Get the record data (new_data for INSERT/UPDATE, old_data for DELETE)
        let record_data = match change.operation {
            crate::models::sync_change::SyncOperation::Insert | 
            crate::models::sync_change::SyncOperation::Update => {
                change.new_data.clone()
            }
            crate::models::sync_change::SyncOperation::Delete => {
                change.old_data.clone()
            }
        };
        
        if let Some(data) = record_data {
            // Add record ID to the data
            let mut record = data.clone();
            if let Some(obj) = record.as_object_mut() {
                obj.insert("id".to_string(), json!(change.record_id));
            }
            
            changes_by_table
                .entry(table_name)
                .or_insert_with(std::collections::HashMap::new)
                .entry(operation_type.to_string())
                .or_insert_with(Vec::new)
                .push(record);
        }
    }
    
    // Convert to WatermelonDB-compatible format
    let mut changes_json = json!({});
    for (table, operations) in changes_by_table {
        let mut table_changes = json!({});
        for (op_type, records) in operations {
            table_changes[op_type] = json!(records);
        }
        changes_json[table] = table_changes;
    }
    
    let timestamp = Utc::now();
    
    Ok(PullResponse {
        changes: changes_json,
        timestamp,
    })
}

