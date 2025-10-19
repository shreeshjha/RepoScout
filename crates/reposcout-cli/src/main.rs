use clap::Parser;
use reposcout_core::{providers::GitHubProvider, search::{SearchEngine, SearchProvider}};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "reposcout")]
#[command(version, about = "Terminal-based Git repository discovery platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// GitHub personal access token (or set GITHUB_TOKEN env var)
    #[arg(long, env)]
    github_token: Option<String>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Search for repositories
    Search {
        /// Search query
        query: String,

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
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
        Some(Commands::Search { query, limit }) => {
            search_repositories(&query, limit, cli.github_token).await?;
        }
        Some(Commands::Show { name }) => {
            show_repository(&name, cli.github_token).await?;
        }
        None => {
            println!("No command specified. Try --help");
        }
    }

    Ok(())
}

async fn search_repositories(query: &str, limit: usize, token: Option<String>) -> anyhow::Result<()> {
    tracing::info!("Searching for: {}", query);

    let mut engine = SearchEngine::new();
    engine.add_provider(Box::new(GitHubProvider::new(token)));

    let results = engine.search_all(query).await?;

    if results.is_empty() {
        println!("No repositories found for '{}'", query);
        return Ok(());
    }

    println!("\nFound {} repositories:\n", results.len());

    for (i, repo) in results.iter().take(limit).enumerate() {
        println!("{}. {} ({})", i + 1, repo.full_name, repo.platform);
        if let Some(desc) = &repo.description {
            println!("   {}", desc);
        }
        println!("   ⭐ {} | 🍴 {} | {}",
            repo.stars,
            repo.forks,
            repo.language.as_deref().unwrap_or("Unknown")
        );
        println!("   {}\n", repo.url);
    }

    Ok(())
}

async fn show_repository(full_name: &str, token: Option<String>) -> anyhow::Result<()> {
    // Parse owner/repo format
    let parts: Vec<&str> = full_name.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Repository name must be in 'owner/repo' format");
    }

    let (owner, repo) = (parts[0], parts[1]);
    tracing::info!("Fetching repository: {}/{}", owner, repo);

    let provider = GitHubProvider::new(token);
    let repository = provider.get_repository(owner, repo).await?;

    println!("\n{}\n", "=".repeat(60));
    println!("📦 {}", repository.full_name);
    println!("{}\n", "=".repeat(60));

    if let Some(desc) = &repository.description {
        println!("{}\n", desc);
    }

    println!("Platform:      {}", repository.platform);
    println!("Language:      {}", repository.language.as_deref().unwrap_or("Unknown"));
    println!("Stars:         ⭐ {}", repository.stars);
    println!("Forks:         🍴 {}", repository.forks);
    println!("Open Issues:   {}", repository.open_issues);
    println!("License:       {}", repository.license.as_deref().unwrap_or("None"));
    println!("Created:       {}", repository.created_at.format("%Y-%m-%d"));
    println!("Last Updated:  {}", repository.updated_at.format("%Y-%m-%d"));
    println!("Last Pushed:   {}", repository.pushed_at.format("%Y-%m-%d"));

    if !repository.topics.is_empty() {
        println!("\nTopics: {}", repository.topics.join(", "));
    }

    if let Some(homepage) = &repository.homepage_url {
        if !homepage.is_empty() {
            println!("Homepage: {}", homepage);
        }
    }

    println!("\n{}", repository.url);

    Ok(())
}
