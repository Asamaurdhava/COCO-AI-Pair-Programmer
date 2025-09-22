use clap::{Parser, Subcommand};
use anyhow::Result;

mod app;
mod ui;
mod ai;
mod watcher;
mod session;
mod config;

use app::App;

#[derive(Parser)]
#[command(name = "coco")]
#[command(about = "CoCo v2.0 - AI pair programmer that shows what AI thinks")]
#[command(version = "2.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start watching (default)
    Start,
    /// Record session
    Record,
    /// Replay session
    Replay { id: String },
    /// List sessions
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Start) => start_coco().await?,
        Some(Commands::Record) => start_recording().await?,
        Some(Commands::Replay { id }) => replay_session(&id).await?,
        Some(Commands::List) => list_sessions()?,
    }

    Ok(())
}

async fn start_coco() -> Result<()> {
    tracing::info!("Starting CoCo v2.0...");

    // Initialize application
    let mut app = App::new().await?;

    // Validate configuration
    app.config.validate().await?;

    // Start main application loop
    app.run().await?;

    Ok(())
}

async fn start_recording() -> Result<()> {
    tracing::info!("Starting CoCo v2.0 with session recording...");

    // Initialize application with recording enabled
    let mut app = App::new_with_recording().await?;

    // Validate configuration
    app.config.validate().await?;

    // Start main application loop
    app.run().await?;

    Ok(())
}

async fn replay_session(id: &str) -> Result<()> {
    tracing::info!("Replaying session: {}", id);

    // Load and replay session
    let session = session::load_session(id)?;
    session::replay(session).await?;

    Ok(())
}

fn list_sessions() -> Result<()> {
    println!("📝 Recorded Sessions:");

    let sessions = session::list_sessions()?;

    if sessions.is_empty() {
        println!("   No sessions found. Use 'coco record' to start recording.");
        return Ok(());
    }

    for session in sessions {
        println!("   🎥 {} - {} events ({})",
            session.id,
            session.events.len(),
            session.started_at.format("%Y-%m-%d %H:%M")
        );
    }

    Ok(())
}