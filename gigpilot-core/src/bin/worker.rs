use dotenv::dotenv;
use gigpilot_core::db::Database;
use gigpilot_core::worker::JobScheduler;
use tokio::signal;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Worker binary entry point for the invoice chasing agent.
/// 
/// This binary runs as a background worker that:
/// - Polls for overdue invoices
/// - Processes them through the state machine
/// - Sends chase emails
/// - Updates invoice states
/// 
/// The worker survives server restarts by storing state in the database.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(LevelFilter::INFO.into());
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
    
    info!("Starting GigPilot Invoice Chasing Worker...");
    
    // Initialize database connection pool
    let db_pool = Database::new().await?;
    
    // Get poll interval from environment (default: 60 seconds)
    let poll_interval = std::env::var("WORKER_POLL_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    
    // Create scheduler
    let mut scheduler = JobScheduler::new(db_pool, Some(poll_interval));
    
    // Handle shutdown signals gracefully (cross-platform)
    let mut ctrl_c = signal::ctrl_c();
    
    // Spawn the scheduler in a task
    let scheduler_handle = tokio::spawn(async move {
        if let Err(e) = scheduler.start().await {
            tracing::error!("Scheduler error: {}", e);
        }
    });
    
    // Wait for shutdown signal
    tokio::select! {
        _ = ctrl_c.recv() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = scheduler_handle => {
            info!("Scheduler task completed");
        }
    }
    
    // Note: The scheduler will check the running flag on each iteration
    // For a more immediate shutdown, we could use a channel or shared state
    
    info!("GigPilot Invoice Chasing Worker stopped");
    Ok(())
}

