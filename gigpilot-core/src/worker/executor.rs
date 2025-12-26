use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::invoice::Invoice;
use crate::worker::services::{generate_email, send_email};
use crate::worker::state_machine::{ChaseAction, ChaseState, ChaseStateMachine, Transition};

/// Executor for processing invoice chase actions.
/// 
/// Handles the execution of chase actions determined by the state machine,
/// including generating emails, sending them, and updating invoice state.
pub struct ChaseExecutor {
    /// Database connection pool
    pool: PgPool,
}

impl ChaseExecutor {
    /// Creates a new chase executor.
    /// 
    /// # Arguments
    /// 
    /// * `pool` - PostgreSQL connection pool
    /// 
    /// # Returns
    /// 
    /// Returns a new `ChaseExecutor` instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Processes an invoice through the chasing state machine.
    /// 
    /// This function:
    /// 1. Determines the current chase state (from metadata or defaults to Pending)
    /// 2. Calculates days overdue
    /// 3. Transitions to next state using the state machine
    /// 4. Executes the required action (send email, etc.)
    /// 5. Updates the invoice state in the database
    /// 
    /// # Arguments
    /// 
    /// * `invoice` - The invoice to process
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if processing succeeded, or an error.
    pub async fn process_invoice(&self, invoice: &Invoice) -> Result<(), anyhow::Error> {
        info!(
            "Processing invoice {} for chasing",
            invoice.invoice_number
        );
        
        // Get current chase state from metadata or default to Pending
        let current_state = self.get_chase_state(invoice)?;
        
        // Calculate days overdue
        let days_overdue = self.calculate_days_overdue(invoice)?;
        
        // Determine next state and action
        let (next_state, action) = ChaseStateMachine::transition(current_state, days_overdue);
        
        info!(
            "Invoice {}: {} -> {} (action: {})",
            invoice.invoice_number,
            current_state,
            next_state,
            action
        );
        
        // Execute the action
        match action {
            ChaseAction::SendPoliteReminder => {
                self.send_chase_email(invoice, "polite", &next_state).await?;
            }
            ChaseAction::SendFirmReminder => {
                self.send_chase_email(invoice, "firm", &next_state).await?;
            }
            ChaseAction::MarkAsPaid => {
                // Invoice was marked as paid, update state
                self.update_chase_state(invoice.id, next_state).await?;
            }
            ChaseAction::NoAction => {
                info!("No action required for invoice {}", invoice.invoice_number);
                // Still update state if it changed
                if current_state != next_state {
                    self.update_chase_state(invoice.id, next_state).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Gets the current chase state from invoice metadata.
    /// 
    /// # Arguments
    /// 
    /// * `invoice` - The invoice
    /// 
    /// # Returns
    /// 
    /// Returns the current chase state, or Pending if not set.
    fn get_chase_state(&self, invoice: &Invoice) -> Result<ChaseState, anyhow::Error> {
        // Try to get chase_state from metadata
        if let Some(metadata) = &invoice.metadata {
            if let Some(chase_state_str) = metadata.get("chase_state").and_then(|v| v.as_str()) {
                match chase_state_str {
                    "pending" => return Ok(ChaseState::Pending),
                    "overdue" => return Ok(ChaseState::Overdue),
                    "chasing_level_1" => return Ok(ChaseState::ChasingLevel1),
                    "chasing_level_2" => return Ok(ChaseState::ChasingLevel2),
                    "paid" => return Ok(ChaseState::Paid),
                    _ => {
                        warn!("Unknown chase_state in metadata: {}", chase_state_str);
                    }
                }
            }
        }
        
        // Default based on invoice status and due date
        if invoice.status == crate::models::invoice::InvoiceStatus::Paid {
            Ok(ChaseState::Paid)
        } else if let Some(due_date) = invoice.due_date {
            let today = Utc::now().date_naive();
            if due_date < today {
                Ok(ChaseState::Overdue)
            } else {
                Ok(ChaseState::Pending)
            }
        } else {
            Ok(ChaseState::Pending)
        }
    }

    /// Calculates the number of days an invoice is overdue.
    /// 
    /// # Arguments
    /// 
    /// * `invoice` - The invoice
    /// 
    /// # Returns
    /// 
    /// Returns the number of days overdue, or 0 if not overdue.
    fn calculate_days_overdue(&self, invoice: &Invoice) -> Result<i64, anyhow::Error> {
        let today = Utc::now().date_naive();
        
        if let Some(due_date) = invoice.due_date {
            if due_date < today {
                let days = (today - due_date).num_days();
                Ok(days)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    /// Sends a chase email for an invoice.
    /// 
    /// # Arguments
    /// 
    /// * `invoice` - The invoice to chase
    /// * `tone` - Email tone ("polite" or "firm")
    /// * `new_state` - The new chase state after sending
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the email was sent and state updated, or an error.
    async fn send_chase_email(
        &self,
        invoice: &Invoice,
        tone: &str,
        new_state: &ChaseState,
    ) -> Result<(), anyhow::Error> {
        // Get client email
        let client_email = invoice.client_email.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No client email for invoice {}", invoice.invoice_number)
        })?;
        
        // Build context string for LLM
        let context = format!(
            "Invoice {} for {} {:.2} (Due: {:?})",
            invoice.invoice_number,
            invoice.currency,
            invoice.amount,
            invoice.due_date
        );
        
        // Generate email content using LLM
        let (subject, body) = generate_email(tone, &context).await?;
        
        // Send email
        send_email(client_email, &subject, &body).await?;
        
        // Update invoice state
        self.update_chase_state(invoice.id, *new_state).await?;
        
        info!(
            "Sent {} chase email for invoice {} to {}",
            tone, invoice.invoice_number, client_email
        );
        
        Ok(())
    }

    /// Updates the chase state in the invoice metadata.
    /// 
    /// # Arguments
    /// 
    /// * `invoice_id` - ID of the invoice to update
    /// * `state` - New chase state
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the update succeeded, or an error.
    async fn update_chase_state(
        &self,
        invoice_id: Uuid,
        state: ChaseState,
    ) -> Result<(), anyhow::Error> {
        let state_str = state.to_string();
        
        sqlx::query!(
            r#"
            UPDATE invoices
            SET 
                metadata = COALESCE(metadata, '{}'::jsonb) || jsonb_build_object('chase_state', $2),
                updated_at = NOW(),
                last_modified = NOW()
            WHERE id = $1
            "#,
            invoice_id,
            state_str
        )
        .execute(&self.pool)
        .await?;
        
        info!("Updated chase state for invoice {} to {}", invoice_id, state);
        Ok(())
    }
}

