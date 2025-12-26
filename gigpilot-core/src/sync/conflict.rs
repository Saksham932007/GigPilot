use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::sync_change::SyncOperation;
use crate::sync::types::ConflictStrategy;

/// Checks if a conflict exists between client and server versions.
/// 
/// A conflict occurs when:
/// - The record exists on the server with a different version vector
/// - The server version was modified after the client's last_modified timestamp
/// 
/// # Arguments
/// 
/// * `executor` - Database executor (pool or transaction)
/// * `user_id` - ID of the user
/// * `table_name` - Name of the table
/// * `record_id` - ID of the record
/// * `client_version_vector` - Client's version vector
/// * `client_last_modified` - Client's last_modified timestamp
/// 
/// # Returns
/// 
/// Returns `true` if a conflict exists, `false` otherwise.
pub async fn has_conflict<'a, E>(
    executor: E,
    user_id: Uuid,
    table_name: &str,
    record_id: Uuid,
    client_version_vector: Option<&Value>,
    client_last_modified: Option<DateTime<Utc>>,
) -> Result<bool, anyhow::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    // Check if record exists and get its current state
    match table_name {
        "invoices" => {
            let result = sqlx::query!(
                r#"
                SELECT last_modified, version_vector
                FROM invoices
                WHERE id = $1 AND user_id = $2 AND is_deleted = false
                "#,
                record_id,
                user_id
            )
            .fetch_optional(executor)
            .await?;
            
            if let Some(row) = result {
                // Check if server version is newer
                if let Some(server_last_modified) = row.last_modified {
                    if let Some(client_modified) = client_last_modified {
                        if server_last_modified > client_modified {
                            info!(
                                "Conflict detected: server version is newer (server: {:?}, client: {:?})",
                                server_last_modified, client_modified
                            );
                            return Ok(true);
                        }
                    }
                }
                
                // Check version vectors if provided
                if let (Some(client_vv), Some(server_vv)) = (client_version_vector, row.version_vector.as_ref()) {
                    if client_vv != server_vv {
                        info!("Conflict detected: version vectors differ");
                        return Ok(true);
                    }
                }
            }
        }
        _ => {
            warn!("Conflict check not implemented for table: {}", table_name);
        }
    }
    
    Ok(false)
}

/// Resolves a conflict between client and server versions.
/// 
/// Uses the specified conflict strategy to determine which version wins.
/// 
/// # Arguments
/// 
/// * `executor` - Database executor (pool or transaction)
/// * `user_id` - ID of the user
/// * `table_name` - Name of the table
/// * `record_id` - ID of the record
/// * `client_data` - Client's version of the data
/// * `strategy` - Conflict resolution strategy
/// 
/// # Returns
/// 
/// Returns the resolved data (either client or server version).
pub async fn resolve_conflict<'a, E>(
    executor: E,
    user_id: Uuid,
    table_name: &str,
    record_id: Uuid,
    client_data: &Value,
    strategy: ConflictStrategy,
) -> Result<Value, anyhow::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    match strategy {
        ConflictStrategy::ServerWins => {
            info!("Resolving conflict: Server wins for {}:{}", table_name, record_id);
            // Get server version
            match table_name {
                "invoices" => {
                    let invoice = sqlx::query!(
                        r#"
                        SELECT 
                            id, user_id, invoice_number, client_name, client_email,
                            amount, currency, status, due_date, issue_date,
                            last_modified, version_vector, is_deleted,
                            description, line_items, metadata, created_at, updated_at
                        FROM invoices
                        WHERE id = $1 AND user_id = $2
                        "#,
                        record_id,
                        user_id
                    )
                    .fetch_optional(executor)
                    .await?;
                    
                    if let Some(inv) = invoice {
                        Ok(serde_json::json!({
                            "id": inv.id,
                            "user_id": inv.user_id,
                            "invoice_number": inv.invoice_number,
                            "client_name": inv.client_name,
                            "client_email": inv.client_email,
                            "amount": inv.amount.to_string(),
                            "currency": inv.currency,
                            "status": inv.status,
                            "due_date": inv.due_date,
                            "issue_date": inv.issue_date,
                            "last_modified": inv.last_modified,
                            "version_vector": inv.version_vector,
                            "is_deleted": inv.is_deleted,
                            "description": inv.description,
                            "line_items": inv.line_items,
                            "metadata": inv.metadata,
                            "created_at": inv.created_at,
                            "updated_at": inv.updated_at,
                        }))
                    } else {
                        // Record doesn't exist on server, use client version
                        Ok(client_data.clone())
                    }
                }
                _ => {
                    warn!("Conflict resolution not implemented for table: {}", table_name);
                    Ok(client_data.clone())
                }
            }
        }
        ConflictStrategy::ClientWins => {
            info!("Resolving conflict: Client wins for {}:{}", table_name, record_id);
            Ok(client_data.clone())
        }
        ConflictStrategy::LastWriteWins => {
            info!("Resolving conflict: Last write wins for {}:{}", table_name, record_id);
            // Compare timestamps - for now, use client version
            // In a full implementation, we'd compare last_modified timestamps
            Ok(client_data.clone())
        }
    }
}

