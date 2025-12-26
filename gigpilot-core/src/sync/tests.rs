#[cfg(test)]
mod tests {
    use crate::sync::push::push_changes;
    use crate::sync::types::{PushChange, PushRequest};
    use chrono::Utc;
    use serde_json::json;
    use sqlx::PgPool;
    use std::str::FromStr;
    use uuid::Uuid;

    /// Test helper to create a test database pool.
    /// 
    /// In a real test environment, this would use a test database.
    /// For now, this is a placeholder that would need DATABASE_URL set.
    async fn create_test_pool() -> Result<PgPool, anyhow::Error> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL not set for tests"))?;
        
        let pool = PgPool::connect(&database_url).await?;
        Ok(pool)
    }

    /// Test that pushing a change updates the database.
    /// 
    /// This test verifies that:
    /// 1. A push request with an invoice change is processed
    /// 2. The invoice is inserted/updated in the database
    /// 3. A sync_change record is created
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_push_changes_updates_database() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        
        // Create a test user (this would normally be done in test setup)
        let test_user_id = Uuid::new_v4();
        
        // Create a test invoice change
        let invoice_id = Uuid::new_v4();
        let invoice_data = json!({
            "id": invoice_id,
            "invoice_number": "INV-001",
            "client_name": "Test Client",
            "client_email": "test@example.com",
            "amount": "100.00",
            "currency": "USD",
            "status": "draft",
            "issue_date": "2024-01-01",
            "last_modified": Utc::now().to_rfc3339(),
        });
        
        let push_request = PushRequest {
            changes: vec![PushChange {
                table: "invoices".to_string(),
                id: invoice_id,
                data: Some(invoice_data),
                deleted: false,
                device_id: Some("test-device".to_string()),
                version_vector: None,
            }],
            device_id: Some("test-device".to_string()),
        };
        
        // Push the change
        let response = push_changes(&pool, test_user_id, push_request)
            .await
            .expect("Push should succeed");
        
        // Verify the response
        assert_eq!(response.applied, 1);
        assert_eq!(response.conflicts, 0);
        
        // Verify the invoice was created in the database
        let invoice = sqlx::query!(
            "SELECT invoice_number, client_name FROM invoices WHERE id = $1 AND user_id = $2",
            invoice_id,
            test_user_id
        )
        .fetch_optional(&pool)
        .await
        .expect("Query should succeed");
        
        assert!(invoice.is_some(), "Invoice should be created");
        let inv = invoice.unwrap();
        assert_eq!(inv.invoice_number, "INV-001");
        assert_eq!(inv.client_name, "Test Client");
        
        // Verify sync_change was created
        let sync_change = sqlx::query!(
            "SELECT id FROM sync_changes WHERE record_id = $1 AND user_id = $2",
            invoice_id,
            test_user_id
        )
        .fetch_optional(&pool)
        .await
        .expect("Query should succeed");
        
        assert!(sync_change.is_some(), "Sync change should be recorded");
    }

    /// Test that pushing an update to an existing invoice works.
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_push_update_existing_invoice() {
        let pool = create_test_pool().await.expect("Failed to create test pool");
        let test_user_id = Uuid::new_v4();
        let invoice_id = Uuid::new_v4();
        
        // First, create an invoice
        sqlx::query!(
            r#"
            INSERT INTO invoices (
                id, user_id, invoice_number, client_name, amount, currency, status, issue_date
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            invoice_id,
            test_user_id,
            "INV-001",
            "Original Client",
            rust_decimal::Decimal::from_str_exact("100.00").unwrap(),
            "USD",
            "draft",
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        )
        .execute(&pool)
        .await
        .expect("Should insert invoice");
        
        // Now push an update
        let update_data = json!({
            "id": invoice_id,
            "invoice_number": "INV-001",
            "client_name": "Updated Client",
            "amount": "150.00",
            "currency": "USD",
            "status": "sent",
            "issue_date": "2024-01-01",
            "last_modified": Utc::now().to_rfc3339(),
        });
        
        let push_request = PushRequest {
            changes: vec![PushChange {
                table: "invoices".to_string(),
                id: invoice_id,
                data: Some(update_data),
                deleted: false,
                device_id: Some("test-device".to_string()),
                version_vector: None,
            }],
            device_id: Some("test-device".to_string()),
        };
        
        let response = push_changes(&pool, test_user_id, push_request)
            .await
            .expect("Push should succeed");
        
        assert_eq!(response.applied, 1);
        
        // Verify the update
        let invoice = sqlx::query!(
            "SELECT client_name, amount FROM invoices WHERE id = $1 AND user_id = $2",
            invoice_id,
            test_user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Invoice should exist");
        
        assert_eq!(invoice.client_name, "Updated Client");
    }
}

