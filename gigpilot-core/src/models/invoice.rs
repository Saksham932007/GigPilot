use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Invoice status enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum InvoiceStatus {
    #[sqlx(rename = "draft")]
    Draft,
    #[sqlx(rename = "sent")]
    Sent,
    #[sqlx(rename = "paid")]
    Paid,
    #[sqlx(rename = "overdue")]
    Overdue,
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

/// Invoice model representing an invoice in the system.
/// 
/// This struct maps to the `invoices` table and includes sync metadata
/// for offline-first synchronization with version vectors.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    /// Unique identifier for the invoice
    pub id: Uuid,
    
    /// ID of the user who owns this invoice
    pub user_id: Uuid,
    
    /// Invoice number (unique per user)
    pub invoice_number: String,
    
    /// Client name
    pub client_name: String,
    
    /// Client email address
    pub client_email: Option<String>,
    
    /// Invoice amount
    pub amount: rust_decimal::Decimal,
    
    /// Currency code (ISO 4217)
    pub currency: String,
    
    /// Invoice status
    pub status: InvoiceStatus,
    
    /// Due date for payment
    pub due_date: Option<NaiveDate>,
    
    /// Date when invoice was issued
    pub issue_date: NaiveDate,
    
    /// Last modification timestamp (for sync)
    pub last_modified: DateTime<Utc>,
    
    /// Version vector for CRDT sync (device_id -> timestamp)
    pub version_vector: Option<Value>,
    
    /// Soft delete flag (for sync)
    pub is_deleted: bool,
    
    /// Invoice description
    pub description: Option<String>,
    
    /// Line items (JSON array)
    pub line_items: Option<Value>,
    
    /// Additional metadata (flexible JSON)
    pub metadata: Option<Value>,
    
    /// Timestamp when the invoice was created
    pub created_at: DateTime<Utc>,
    
    /// Timestamp when the invoice was last updated
    pub updated_at: DateTime<Utc>,
}

/// Invoice creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoice {
    pub invoice_number: String,
    pub client_name: String,
    pub client_email: Option<String>,
    pub amount: rust_decimal::Decimal,
    pub currency: Option<String>,
    pub status: Option<InvoiceStatus>,
    pub due_date: Option<NaiveDate>,
    pub issue_date: Option<NaiveDate>,
    pub description: Option<String>,
    pub line_items: Option<Value>,
    pub metadata: Option<Value>,
}

/// Invoice update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInvoice {
    pub invoice_number: Option<String>,
    pub client_name: Option<String>,
    pub client_email: Option<String>,
    pub amount: Option<rust_decimal::Decimal>,
    pub currency: Option<String>,
    pub status: Option<InvoiceStatus>,
    pub due_date: Option<NaiveDate>,
    pub issue_date: Option<NaiveDate>,
    pub description: Option<String>,
    pub line_items: Option<Value>,
    pub metadata: Option<Value>,
    pub version_vector: Option<Value>,
}

/// Invoice response (public representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub invoice_number: String,
    pub client_name: String,
    pub client_email: Option<String>,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub status: InvoiceStatus,
    pub due_date: Option<NaiveDate>,
    pub issue_date: NaiveDate,
    pub last_modified: DateTime<Utc>,
    pub version_vector: Option<Value>,
    pub description: Option<String>,
    pub line_items: Option<Value>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Invoice> for InvoiceResponse {
    fn from(invoice: Invoice) -> Self {
        InvoiceResponse {
            id: invoice.id,
            user_id: invoice.user_id,
            invoice_number: invoice.invoice_number,
            client_name: invoice.client_name,
            client_email: invoice.client_email,
            amount: invoice.amount,
            currency: invoice.currency,
            status: invoice.status,
            due_date: invoice.due_date,
            issue_date: invoice.issue_date,
            last_modified: invoice.last_modified,
            version_vector: invoice.version_vector,
            description: invoice.description,
            line_items: invoice.line_items,
            metadata: invoice.metadata,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
        }
    }
}

