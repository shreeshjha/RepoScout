use clap::Parser;
use reposcout_cache::CacheManager;
use reposcout_core::{providers::{GitHubProvider, GitLabProvider}, CachedSearchEngine};
use std::path::PathBuf;
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

    /// GitLab personal access token (or set GITLAB_TOKEN env var)
    #[arg(long, env)]
    gitlab_token: Option<String>,
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

        /// Filter by programming language (e.g., rust, python, go)
        #[arg(short = 'l', long)]
        language: Option<String>,

        /// Minimum number of stars
        #[arg(long)]
        min_stars: Option<u32>,

        /// Maximum number of stars
        #[arg(long)]
        max_stars: Option<u32>,

        /// Filter by pushed date (e.g., >2024-01-01, <2023-12-31)
        #[arg(long)]
        pushed: Option<String>,

        /// Sort by: stars, forks, updated (default: stars)
        #[arg(short = 's', long, default_value = "stars")]
        sort: String,
    },
    /// Show repository details
    Show {
        /// Repository name (owner/repo)
        name: String,
    },
    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Launch interactive TUI
    Tui,
}

#[derive(clap::Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cached data
    Clear,
    /// Clean up expired entries
    Cleanup,
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
        Some(Commands::Search {
            query,
            limit,
            language,
            min_stars,
            max_stars,
            pushed,
            sort,
        }) => {
            search_repositories(
                &query,
                limit,
                language,
                min_stars,
                max_stars,
                pushed,
                &sort,
                cli.github_token,
                cli.gitlab_token,
            )
            .await?;
        }
        Some(Commands::Show { name }) => {
            show_repository(&name, cli.github_token, cli.gitlab_token).await?;
        }
        Some(Commands::Cache { action }) => {
            handle_cache_command(action).await?;
        }
        Some(Commands::Tui) => {
            run_tui_mode(cli.github_token, cli.gitlab_token).await?;
        }
        None => {
            println!("No command specified. Try --help");
        }
    }

    Ok(())
}

async fn search_repositories(
    query: &str,
    limit: usize,
    language: Option<String>,
    min_stars: Option<u32>,
    max_stars: Option<u32>,
    pushed: Option<String>,
    sort: &str,
    github_token: Option<String>,
    gitlab_token: Option<String>,
) -> anyhow::Result<()> {
    // Build GitHub search query with filters
    let search_query = build_github_query(query, language, min_stars, max_stars, pushed);
    tracing::info!("Searching for: {}", search_query);

    // Initialize cache
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    let mut engine = CachedSearchEngine::with_cache(cache);
    // Add both providers - search across both platforms
    engine.add_provider(Box::new(GitHubProvider::new(github_token)));
    engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));

    let mut results = engine.search(&search_query).await?;

    // Sort results based on user preference
    sort_results(&mut results, sort);

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

async fn show_repository(full_name: &str, github_token: Option<String>, gitlab_token: Option<String>) -> anyhow::Result<()> {
    // Parse owner/repo format
    let parts: Vec<&str> = full_name.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Repository name must be in 'owner/repo' format");
    }

    let (owner, repo) = (parts[0], parts[1]);
    tracing::info!("Fetching repository: {}/{}", owner, repo);

    // Initialize cache
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    let mut engine = CachedSearchEngine::with_cache(cache);
    // Add both providers - will try both platforms
    engine.add_provider(Box::new(GitHubProvider::new(github_token)));
    engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));

    let repository = engine.get_repository(owner, repo).await?;

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

async fn handle_cache_command(action: CacheAction) -> anyhow::Result<()> {
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    match action {
        CacheAction::Stats => {
            let stats = cache.stats()?;
            println!("\nCache Statistics:\n");
            println!("Total entries:   {}", stats.total_entries);
            println!("Valid entries:   {}", stats.valid_entries);
            println!("Expired entries: {}", stats.expired_entries);
            println!("Cache size:      {} KB", stats.size_bytes / 1024);
            println!("\nCache location:  {}", cache_path.display());
        }
        CacheAction::Clear => {
            cache.clear()?;
            println!("✅ Cache cleared successfully");
        }
        CacheAction::Cleanup => {
            let deleted = cache.cleanup_expired()?;
            println!("✅ Cleaned up {} expired entries", deleted);
        }
    }

    Ok(())
}

/// Build GitHub search query with filters
///
/// GitHub uses special syntax like "language:rust stars:>1000"
/// We build this query string based on user filters
fn build_github_query(
    query: &str,
    language: Option<String>,
    min_stars: Option<u32>,
    max_stars: Option<u32>,
    pushed: Option<String>,
) -> String {
    let mut parts = vec![query.to_string()];

    if let Some(lang) = language {
        parts.push(format!("language:{}", lang));
    }

    // GitHub stars filter syntax
    match (min_stars, max_stars) {
        (Some(min), Some(max)) => parts.push(format!("stars:{}..{}", min, max)),
        (Some(min), None) => parts.push(format!("stars:>={}", min)),
        (None, Some(max)) => parts.push(format!("stars:<={}", max)),
        (None, None) => {}
    }

    if let Some(pushed_date) = pushed {
        parts.push(format!("pushed:{}", pushed_date));
    }

    parts.join(" ")
}

/// Sort repository results based on user preference
fn sort_results(results: &mut [reposcout_core::models::Repository], sort_by: &str) {
    match sort_by {
        "stars" => results.sort_by(|a, b| b.stars.cmp(&a.stars)),
        "forks" => results.sort_by(|a, b| b.forks.cmp(&a.forks)),
        "updated" => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        _ => {} // Already sorted by relevance from API
    }
}

async fn run_tui_mode(github_token: Option<String>, gitlab_token: Option<String>) -> anyhow::Result<()> {
    use reposcout_tui::{App, run_tui};
    use reposcout_api::{GitHubClient, GitLabClient};

    let app = App::new();
    let cache_path = get_cache_path()?;
    let cache_path_str = cache_path.to_str().unwrap().to_string();

    // Create API clients for README fetching
    let github_client = GitHubClient::new(github_token.clone());
    let gitlab_client = GitLabClient::new(gitlab_token.clone());

    run_tui(app, move |query| {
        let github_token_clone = github_token.clone();
        let gitlab_token_clone = gitlab_token.clone();
        let cache_path_clone = cache_path_str.clone();

        Box::pin(async move {
            let cache = CacheManager::new(&cache_path_clone, 24)?;
            let mut engine = CachedSearchEngine::with_cache(cache);
            // Search both GitHub and GitLab
            engine.add_provider(Box::new(GitHubProvider::new(github_token_clone)));
            engine.add_provider(Box::new(GitLabProvider::new(gitlab_token_clone)));
            engine.search(query).await.map_err(|e| e.into())
        })
    }, github_client, gitlab_client)
    .await
}

fn get_cache_path() -> anyhow::Result<PathBuf> {
    let cache_dir = if cfg!(target_os = "windows") {
        dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
            .join("reposcout")
    } else {
        dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?
            .join("reposcout")
    };

    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("reposcout.db"))
}
