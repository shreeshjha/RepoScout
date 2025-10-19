use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "reposcout")]
#[command(version, about = "Terminal-based Git repository discovery platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Search for repositories
    Search {
        /// Search query
        query: String,
    },
    /// Show repository details
    Show {
        /// Repository name (owner/repo)
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging - helps when things go sideways
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reposcout=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Search { query }) => {
            tracing::info!("Searching for: {}", query);
            println!("Search functionality coming soon!");
        }
        Some(Commands::Show { name }) => {
            tracing::info!("Showing repository: {}", name);
            println!("Show functionality coming soon!");
        }
        None => {
            println!("No command specified. Try --help");
        }
    }

    Ok(())
}
