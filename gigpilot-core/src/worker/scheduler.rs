use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::invoice::Invoice;
use crate::worker::executor::ChaseExecutor;

/// Job scheduler for processing overdue invoices.
/// 
/// Polls the database at regular intervals to find invoices that need
/// chasing and processes them through the state machine.
pub struct JobScheduler {
    /// Database connection pool
    pool: PgPool,
    
    /// Polling interval in seconds
    poll_interval_seconds: u64,
    
    /// Whether the scheduler is running (wrapped in Arc for sharing)
    running: Arc<RwLock<bool>>,
}

impl JobScheduler {
    /// Creates a new job scheduler.
    /// 
    /// # Arguments
    /// 
    /// * `pool` - PostgreSQL connection pool
    /// * `poll_interval_seconds` - How often to poll for overdue invoices (default: 60)
    /// 
    /// # Returns
    /// 
    /// Returns a new `JobScheduler` instance.
    pub fn new(pool: PgPool, poll_interval_seconds: Option<u64>) -> Self {
        Self {
            pool,
            poll_interval_seconds: poll_interval_seconds.unwrap_or(60),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Starts the scheduler loop.
    /// 
    /// This function runs indefinitely, polling for overdue invoices
    /// and processing them. It handles errors gracefully and continues
    /// running even if individual invoice processing fails.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the initial database query fails.
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        *self.running.write().await = true;
        info!(
            "JobScheduler started with poll interval: {} seconds",
            self.poll_interval_seconds
        );
        
        while *self.running.read().await {
            match self.poll_and_process().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Processed {} overdue invoice(s)", count);
                    }
                }
                Err(e) => {
                    error!("Error in scheduler loop: {}", e);
                    // Continue running even on error
                }
            }
            
            // Wait before next poll
            sleep(Duration::from_secs(self.poll_interval_seconds)).await;
        }
        
        info!("JobScheduler stopped");
        Ok(())
    }

    /// Stops the scheduler loop.
    /// 
    /// Sets the running flag to false, which will cause the loop
    /// to exit after the current iteration.
    pub async fn stop(&self) {
        info!("Stopping JobScheduler...");
        *self.running.write().await = false;
    }

    /// Polls the database for overdue invoices and processes them.
    /// 
    /// Finds all invoices where:
    /// - due_date < current date
    /// - status != 'paid'
    /// - is_deleted = false
    /// 
    /// # Returns
    /// 
    /// Returns the number of invoices processed, or an error.
    async fn poll_and_process(&self) -> Result<usize, anyhow::Error> {
        let overdue_invoices = self.find_overdue_invoices().await?;
        
        if overdue_invoices.is_empty() {
            return Ok(0);
        }
        
        info!("Found {} overdue invoice(s) to process", overdue_invoices.len());
        
        let mut processed = 0;
        for invoice in overdue_invoices {
            match self.process_invoice(&invoice).await {
                Ok(_) => {
                    processed += 1;
                    info!("Successfully processed invoice: {}", invoice.invoice_number);
                }
                Err(e) => {
                    error!(
                        "Failed to process invoice {}: {}",
                        invoice.invoice_number, e
                    );
                    // Continue with other invoices
                }
            }
        }
        
        Ok(processed)
    }

    /// Finds all overdue invoices that need chasing.
    /// 
    /// Queries the database for invoices where the due date has passed
    /// and the invoice is not yet paid.
    /// 
    /// # Returns
    /// 
    /// Returns a vector of `Invoice` structs, or an error.
    async fn find_overdue_invoices(&self) -> Result<Vec<Invoice>, anyhow::Error> {
        let today = Utc::now().date_naive();
        
        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT 
                id, user_id, invoice_number, client_name, client_email,
                amount, currency, status, due_date, issue_date,
                last_modified, version_vector, is_deleted,
                description, line_items, metadata, created_at, updated_at
            FROM invoices
            WHERE due_date < $1
                AND status != 'paid'
                AND is_deleted = false
            ORDER BY due_date ASC
            LIMIT 100
            "#,
        )
        .bind(today)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(invoices)
    }

    /// Processes a single invoice through the chasing state machine.
    /// 
    /// Uses the ChaseExecutor to handle state transitions and actions.
    /// 
    /// # Arguments
    /// 
    /// * `invoice` - The invoice to process
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if processing succeeded, or an error.
    async fn process_invoice(&self, invoice: &Invoice) -> Result<(), anyhow::Error> {
        let executor = ChaseExecutor::new(self.pool.clone());
        executor.process_invoice(invoice).await
    }
}

