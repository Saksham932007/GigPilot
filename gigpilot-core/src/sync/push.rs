use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::str::FromStr;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::sync_change::SyncOperation;
use crate::sync::conflict::{has_conflict, resolve_conflict};
use crate::sync::types::{ConflictStrategy, PushChange, PushRequest, PushResponse};

/// Applies changes from the client to the server (Push synchronization).
/// 
/// This function implements the "Push" part of the sync protocol. It applies
/// changes transactionally, handling conflicts and recording changes in the
/// sync_changes table.
/// 
/// # Arguments
/// 
/// * `pool` - PostgreSQL connection pool
/// * `user_id` - ID of the user making the changes
/// * `request` - Push request containing array of changes
/// 
/// # Returns
/// 
/// Returns a `Result<PushResponse>` containing the number of applied changes
/// and conflicts, or an error if the operation fails.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Database transaction fails
/// - Conflict resolution fails
/// - Change application fails
pub async fn push_changes(
    pool: &PgPool,
    user_id: Uuid,
    request: PushRequest,
) -> Result<PushResponse, anyhow::Error> {
    info!(
        "Push sync requested for user {} with {} changes",
        user_id,
        request.changes.len()
    );
    
    let device_id = request.device_id.unwrap_or_else(|| "unknown".to_string());
    let mut applied_count = 0;
    let mut conflict_count = 0;
    let mut conflicted_ids = Vec::new();
    
    // Start a transaction for atomicity
    let mut tx = pool.begin().await?;
    
    for change in request.changes {
        match apply_change(
            &mut tx,
            user_id,
            &change,
            &device_id,
            ConflictStrategy::ServerWins, // Default strategy
        )
        .await
        {
            Ok(was_conflict) => {
                if was_conflict {
                    conflict_count += 1;
                    conflicted_ids.push(change.id);
                    warn!(
                        "Conflict detected and resolved for {}:{}",
                        change.table, change.id
                    );
                } else {
                    applied_count += 1;
                }
            }
            Err(e) => {
                error!(
                    "Failed to apply change for {}:{}: {}",
                    change.table, change.id, e
                );
                // Continue with other changes (transaction will rollback on final commit if needed)
            }
        }
    }
    
    // Commit the transaction
    tx.commit().await?;
    
    info!(
        "Push sync completed: {} applied, {} conflicts",
        applied_count, conflict_count
    );
    
    Ok(PushResponse {
        applied: applied_count,
        conflicts: conflict_count,
        conflicted_ids,
        timestamp: Utc::now(),
    })
}

/// Applies a single change to the database.
/// 
/// Handles INSERT, UPDATE, and DELETE operations with conflict detection
/// and resolution.
/// 
/// # Arguments
/// 
/// * `tx` - Database transaction
/// * `user_id` - ID of the user
/// * `change` - The change to apply
/// * `device_id` - Device ID making the change
/// * `strategy` - Conflict resolution strategy
/// 
/// # Returns
/// 
/// Returns `Ok(true)` if a conflict occurred and was resolved,
/// `Ok(false)` if no conflict occurred, or an error.
async fn apply_change(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    change: &PushChange,
    device_id: &str,
    strategy: ConflictStrategy,
) -> Result<bool, anyhow::Error> {
    let operation = if change.deleted {
        SyncOperation::Delete
    } else if change.data.is_some() {
        // Check if record exists to determine INSERT vs UPDATE
        let exists = record_exists(tx, &change.table, change.id, user_id).await?;
        if exists {
            SyncOperation::Update
        } else {
            SyncOperation::Insert
        }
    } else {
        return Err(anyhow::anyhow!("Change has no data and is not a delete"));
    };
    
    // Extract last_modified and version_vector from data if present
    let (client_last_modified, client_version_vector) = if let Some(ref data) = change.data {
        let last_mod = data.get("last_modified")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));
        
        let vv = data.get("version_vector").cloned();
        (last_mod, vv)
    } else {
        (None, None)
    };
    
    // Check for conflicts (only for UPDATE operations)
    let has_conf = if operation == SyncOperation::Update {
        has_conflict(
            tx,
            user_id,
            &change.table,
            change.id,
            client_version_vector.as_ref(),
            client_last_modified,
        )
        .await?
    } else {
        false
    };
    
    // Apply the change based on operation type
    match operation {
        SyncOperation::Insert => {
            apply_insert(tx, user_id, change, device_id).await?;
        }
        SyncOperation::Update => {
            if has_conf {
                // Resolve conflict
                let resolved_data = resolve_conflict(
                    tx,
                    user_id,
                    &change.table,
                    change.id,
                    change.data.as_ref().unwrap(),
                    strategy,
                )
                .await?;
                
                // Apply resolved data
                apply_update(tx, user_id, change.id, &change.table, &resolved_data, device_id).await?;
            } else {
                // No conflict, apply client data
                apply_update(
                    tx,
                    user_id,
                    change.id,
                    &change.table,
                    change.data.as_ref().unwrap(),
                    device_id,
                )
                .await?;
            }
        }
        SyncOperation::Delete => {
            apply_delete(tx, user_id, change.id, &change.table, device_id).await?;
        }
    }
    
    // Record the change in sync_changes table
    record_sync_change(
        tx,
        user_id,
        &change.table,
        change.id,
        operation,
        change.data.as_ref(),
        device_id,
        change.version_vector.as_ref(),
    )
    .await?;
    
    Ok(has_conf)
}

/// Checks if a record exists in the database.
async fn record_exists(
    tx: &mut Transaction<'_, Postgres>,
    table_name: &str,
    record_id: Uuid,
    user_id: Uuid,
) -> Result<bool, anyhow::Error> {
    match table_name {
        "invoices" => {
            let result = sqlx::query!(
                "SELECT 1 FROM invoices WHERE id = $1 AND user_id = $2 AND is_deleted = false",
                record_id,
                user_id
            )
            .fetch_optional(&mut **tx)
            .await?;
            Ok(result.is_some())
        }
        _ => {
            warn!("Record existence check not implemented for table: {}", table_name);
            Ok(false)
        }
    }
}

/// Applies an INSERT operation.
async fn apply_insert(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    change: &PushChange,
    device_id: &str,
) -> Result<(), anyhow::Error> {
    let data = change.data.as_ref().ok_or_else(|| {
        anyhow::anyhow!("INSERT operation requires data")
    })?;
    
    match change.table.as_str() {
        "invoices" => {
            let invoice_number = data.get("invoice_number")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing invoice_number"))?;
            
            let client_name = data.get("client_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing client_name"))?;
            
            let amount = data.get("amount")
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        rust_decimal::Decimal::from_str_exact(s).ok()
                    } else if let Some(n) = v.as_f64() {
                        rust_decimal::Decimal::try_from(n).ok()
                    } else {
                        None
                    }
                })
                .ok_or_else(|| anyhow::anyhow!("Invalid amount"))?;
            
            let currency = data.get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("USD");
            
            let status = data.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("draft");
            
            sqlx::query!(
                r#"
                INSERT INTO invoices (
                    id, user_id, invoice_number, client_name, client_email,
                    amount, currency, status, due_date, issue_date,
                    description, line_items, metadata, last_modified, version_vector
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW(), $14
                )
                "#,
                change.id,
                user_id,
                invoice_number,
                client_name,
                data.get("client_email").and_then(|v| v.as_str()),
                amount,
                currency,
                status,
                data.get("due_date").and_then(|v| v.as_str()).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                data.get("issue_date").and_then(|v| v.as_str()).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()).or_else(|| Some(chrono::Utc::now().date_naive())),
                data.get("description").and_then(|v| v.as_str()),
                data.get("line_items"),
                data.get("metadata"),
                change.version_vector.as_ref(),
            )
            .execute(&mut **tx)
            .await?;
        }
        _ => {
            return Err(anyhow::anyhow!("INSERT not implemented for table: {}", change.table));
        }
    }
    
    Ok(())
}

/// Applies an UPDATE operation (upsert logic).
async fn apply_update(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    record_id: Uuid,
    table_name: &str,
    data: &Value,
    device_id: &str,
) -> Result<(), anyhow::Error> {
    match table_name {
        "invoices" => {
            let amount = data.get("amount")
                .and_then(|v| {
                    if let Some(s) = v.as_str() {
                        rust_decimal::Decimal::from_str_exact(s).ok()
                    } else if let Some(n) = v.as_f64() {
                        rust_decimal::Decimal::try_from(n).ok()
                    } else {
                        None
                    }
                });
            
            sqlx::query!(
                r#"
                UPDATE invoices
                SET
                    invoice_number = COALESCE($3, invoice_number),
                    client_name = COALESCE($4, client_name),
                    client_email = $5,
                    amount = COALESCE($6, amount),
                    currency = COALESCE($7, currency),
                    status = COALESCE($8, status),
                    due_date = $9,
                    issue_date = COALESCE($10, issue_date),
                    description = $11,
                    line_items = $12,
                    metadata = $13,
                    last_modified = NOW(),
                    version_vector = $14,
                    updated_at = NOW()
                WHERE id = $1 AND user_id = $2 AND is_deleted = false
                "#,
                record_id,
                user_id,
                data.get("invoice_number").and_then(|v| v.as_str()),
                data.get("client_name").and_then(|v| v.as_str()),
                data.get("client_email").and_then(|v| v.as_str()),
                amount,
                data.get("currency").and_then(|v| v.as_str()),
                data.get("status").and_then(|v| v.as_str()),
                data.get("due_date").and_then(|v| v.as_str()).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                data.get("issue_date").and_then(|v| v.as_str()).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                data.get("description").and_then(|v| v.as_str()),
                data.get("line_items"),
                data.get("metadata"),
                data.get("version_vector"),
            )
            .execute(&mut **tx)
            .await?;
        }
        _ => {
            return Err(anyhow::anyhow!("UPDATE not implemented for table: {}", table_name));
        }
    }
    
    Ok(())
}

/// Applies a DELETE operation (soft delete).
async fn apply_delete(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    record_id: Uuid,
    table_name: &str,
    device_id: &str,
) -> Result<(), anyhow::Error> {
    match table_name {
        "invoices" => {
            sqlx::query!(
                r#"
                UPDATE invoices
                SET is_deleted = true, last_modified = NOW(), updated_at = NOW()
                WHERE id = $1 AND user_id = $2
                "#,
                record_id,
                user_id
            )
            .execute(&mut **tx)
            .await?;
        }
        _ => {
            return Err(anyhow::anyhow!("DELETE not implemented for table: {}", table_name));
        }
    }
    
    Ok(())
}

/// Records a change in the sync_changes table.
async fn record_sync_change(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    table_name: &str,
    record_id: Uuid,
    operation: SyncOperation,
    new_data: Option<&Value>,
    device_id: &str,
    version_vector: Option<&Value>,
) -> Result<(), anyhow::Error> {
    let operation_str = match operation {
        SyncOperation::Insert => "INSERT",
        SyncOperation::Update => "UPDATE",
        SyncOperation::Delete => "DELETE",
    };
    
    sqlx::query!(
        r#"
        INSERT INTO sync_changes (
            user_id, table_name, record_id, operation,
            new_data, device_id, vector_clock, is_applied
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, true
        )
        "#,
        user_id,
        table_name,
        record_id,
        operation_str,
        new_data,
        device_id,
        version_vector,
    )
    .execute(&mut **tx)
    .await?;
    
    Ok(())
}

