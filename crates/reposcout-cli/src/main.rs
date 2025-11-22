use clap::Parser;
use reposcout_cache::{BookmarkEntry, CacheManager};
use reposcout_core::{
    providers::{BitbucketProvider, GitHubProvider, GitLabProvider},
    CachedSearchEngine,
};
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

    /// Bitbucket username (or set BITBUCKET_USERNAME env var)
    #[arg(long, env)]
    bitbucket_username: Option<String>,

    /// Bitbucket app password (or set BITBUCKET_APP_PASSWORD env var)
    #[arg(long, env)]
    bitbucket_app_password: Option<String>,
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

        /// Export results to file (format detected from extension: .json, .csv, .md)
        #[arg(short = 'o', long)]
        export: Option<String>,
    },
    /// Search for code within repositories
    Code {
        /// Code search query (e.g., "function auth", "class:User")
        query: String,

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Filter by programming language (e.g., rust, python, go)
        #[arg(short = 'l', long)]
        language: Option<String>,

        /// Filter by repository (e.g., "owner/repo")
        #[arg(short = 'r', long)]
        repo: Option<String>,

        /// Search only in specific path (e.g., "src/")
        #[arg(short = 'p', long)]
        path: Option<String>,

        /// File extension filter (e.g., "rs", "py")
        #[arg(short = 'e', long)]
        extension: Option<String>,
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
    /// Bookmark management
    Bookmark {
        #[command(subcommand)]
        action: BookmarkAction,
    },
    /// Search history management
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
    /// Launch interactive TUI
    Tui,
    /// Show trending repositories
    Trending {
        /// Time period: daily, weekly, monthly
        #[arg(short = 'p', long, default_value = "weekly")]
        period: String,

        /// Filter by programming language (e.g., rust, python, go)
        #[arg(short = 'l', long)]
        language: Option<String>,

        /// Minimum number of stars
        #[arg(long, default_value = "100")]
        min_stars: u32,

        /// Filter by topic
        #[arg(short = 't', long)]
        topic: Option<String>,

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Sort by star velocity (stars/day) instead of total stars
        #[arg(short = 'v', long)]
        velocity: bool,
    },
    /// Semantic search using natural language queries
    Semantic {
        /// Natural language search query (e.g., "logging library for microservices")
        query: String,

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Use hybrid search (combine semantic + keyword scores)
        #[arg(long)]
        hybrid: bool,

        /// Minimum similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        min_similarity: f32,

        /// Export results to file (format detected from extension: .json, .csv, .md)
        #[arg(short = 'o', long)]
        export: Option<String>,
    },
    /// Semantic index management
    SemanticIndex {
        #[command(subcommand)]
        action: SemanticIndexAction,
    },
    /// Manage GitHub notifications
    Notifications {
        #[command(subcommand)]
        action: NotificationAction,
    },
}

#[derive(clap::Subcommand)]
enum NotificationAction {
    /// List notifications
    List {
        /// Show all notifications (not just unread)
        #[arg(short = 'a', long)]
        all: bool,

        /// Show only participating notifications
        #[arg(short = 'p', long)]
        participating: bool,

        /// Number of notifications to show
        #[arg(short = 'n', long, default_value = "50")]
        limit: u32,

        /// Filter by repository (owner/repo)
        #[arg(short = 'r', long)]
        repo: Option<String>,
    },
    /// Mark a notification as read
    MarkRead {
        /// Notification thread ID
        id: String,
    },
    /// Mark all notifications as read
    MarkAllRead,
}

#[derive(clap::Subcommand)]
enum SemanticIndexAction {
    /// Show semantic index statistics
    Stats,
    /// Rebuild the semantic index from cached repositories
    Rebuild {
        /// Force rebuild even if index exists
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Clear the semantic index
    Clear,
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

#[derive(clap::Subcommand)]
enum BookmarkAction {
    /// List all bookmarks
    List,
    /// Add a repository to bookmarks
    Add {
        /// Repository name (owner/repo)
        name: String,
        /// Optional tags (comma-separated)
        #[arg(short = 't', long)]
        tags: Option<String>,
        /// Optional notes
        #[arg(short = 'n', long)]
        notes: Option<String>,
    },
    /// Remove a bookmark
    Remove {
        /// Repository name (owner/repo)
        name: String,
    },
    /// Export bookmarks to file
    Export {
        /// Output file path
        output: String,
        /// Export format: json or csv
        #[arg(short = 'f', long, default_value = "json")]
        format: String,
    },
    /// Import bookmarks from file
    Import {
        /// Input file path
        input: String,
    },
    /// Clear all bookmarks
    Clear,
}

#[derive(clap::Subcommand)]
enum HistoryAction {
    /// List recent search history
    List {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    /// Search within history
    Search {
        /// Search term to filter history
        term: String,
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
    /// Clear all search history
    Clear,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    // Load tokens from secure storage if not provided via env/CLI
    use reposcout_core::TokenStore;
    if let Ok(store) = TokenStore::load() {
        if cli.github_token.is_none() {
            cli.github_token = store.get_token("github");
        }
        if cli.gitlab_token.is_none() {
            cli.gitlab_token = store.get_token("gitlab");
        }
        // Note: Bitbucket uses username+password, not stored in TokenStore yet
    }

    // Only initialize tracing for non-TUI commands to prevent log interference
    let is_tui_mode = matches!(cli.command, Some(Commands::Tui));

    if !is_tui_mode {
        // Initialize logging - helps when things go sideways
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "reposcout=info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    match cli.command {
        Some(Commands::Search {
            query,
            limit,
            language,
            min_stars,
            max_stars,
            pushed,
            sort,
            export,
        }) => {
            search_repositories(
                &query,
                limit,
                language,
                min_stars,
                max_stars,
                pushed,
                &sort,
                export,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::Code {
            query,
            limit,
            language,
            repo,
            path,
            extension,
        }) => {
            search_code(
                &query,
                limit,
                language,
                repo,
                path,
                extension,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::Show { name }) => {
            show_repository(
                &name,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::Cache { action }) => {
            handle_cache_command(action).await?;
        }
        Some(Commands::Bookmark { action }) => {
            handle_bookmark_command(
                action,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::History { action }) => {
            handle_history_command(action).await?;
        }
        Some(Commands::Tui) => {
            run_tui_mode(
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::Trending {
            period,
            language,
            min_stars,
            topic,
            limit,
            velocity,
        }) => {
            show_trending(
                &period,
                language,
                min_stars,
                topic,
                limit,
                velocity,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::Semantic {
            query,
            limit,
            hybrid,
            min_similarity,
            export,
        }) => {
            handle_semantic_search(
                &query,
                limit,
                hybrid,
                min_similarity,
                export,
                cli.github_token,
                cli.gitlab_token,
                cli.bitbucket_username,
                cli.bitbucket_app_password,
            )
            .await?;
        }
        Some(Commands::SemanticIndex { action }) => {
            handle_semantic_index(&action).await?;
        }
        Some(Commands::Notifications { action }) => {
            handle_notifications(action, cli.github_token).await?;
        }
        None => {
            println!("No command specified. Try --help");
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn search_repositories(
    query: &str,
    limit: usize,
    language: Option<String>,
    min_stars: Option<u32>,
    max_stars: Option<u32>,
    pushed: Option<String>,
    sort: &str,
    export: Option<String>,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    // Build GitHub search query with filters
    let search_query = build_github_query(
        query,
        language.clone(),
        min_stars,
        max_stars,
        pushed.clone(),
    );
    tracing::info!("Searching for: {}", search_query);

    // Initialize cache
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    let mut engine = CachedSearchEngine::with_cache(cache);
    // Add all providers - search across all platforms
    engine.add_provider(Box::new(GitHubProvider::new(github_token)));
    engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));
    engine.add_provider(Box::new(BitbucketProvider::new(
        bitbucket_username,
        bitbucket_app_password,
    )));

    let mut results = engine.search(&search_query).await?;

    // Sort results based on user preference
    sort_results(&mut results, sort);

    // Record search in history (create new cache instance to avoid borrow issues)
    let filters = build_filters_string(
        language.as_deref(),
        min_stars,
        max_stars,
        pushed.as_deref(),
        sort,
    );
    let history_cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;
    if let Err(e) =
        history_cache.add_search_history(query, filters.as_deref(), Some(results.len() as i64))
    {
        tracing::warn!("Failed to save search history: {}", e);
    }

    if results.is_empty() {
        println!("No repositories found for '{}'", query);
        return Ok(());
    }

    // Handle export if requested
    if let Some(export_path) = export {
        use reposcout_core::Exporter;

        // Export all results (not limited by display limit)
        Exporter::export_to_file(&results, &export_path)
            .map_err(|e| anyhow::anyhow!("Export failed: {}", e))?;

        println!(
            "✓ Exported {} repositories to {}",
            results.len(),
            export_path
        );
        return Ok(());
    }

    println!("\nFound {} repositories:\n", results.len());

    for (i, repo) in results.iter().take(limit).enumerate() {
        println!("{}. {} ({})", i + 1, repo.full_name, repo.platform);
        if let Some(desc) = &repo.description {
            println!("   {}", desc);
        }

        // Show health indicator if available
        let health_indicator = if let Some(health) = &repo.health {
            format!(" {} {}", health.status.emoji(), health.maintenance.label())
        } else {
            String::new()
        };

        println!(
            "   ⭐ {} | 🍴 {} | {}{}",
            repo.stars,
            repo.forks,
            repo.language.as_deref().unwrap_or("Unknown"),
            health_indicator
        );
        println!("   {}\n", repo.url);
    }

    Ok(())
}

async fn show_repository(
    full_name: &str,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
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
    // Add all providers - will try all platforms
    engine.add_provider(Box::new(GitHubProvider::new(github_token)));
    engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));
    engine.add_provider(Box::new(BitbucketProvider::new(
        bitbucket_username,
        bitbucket_app_password,
    )));

    let repository = engine.get_repository(owner, repo).await?;

    println!("\n{}\n", "=".repeat(60));
    println!("📦 {}", repository.full_name);
    println!("{}\n", "=".repeat(60));

    if let Some(desc) = &repository.description {
        println!("{}\n", desc);
    }

    println!("Platform:      {}", repository.platform);
    println!(
        "Language:      {}",
        repository.language.as_deref().unwrap_or("Unknown")
    );
    println!("Stars:         ⭐ {}", repository.stars);
    println!("Forks:         🍴 {}", repository.forks);
    println!("Open Issues:   {}", repository.open_issues);
    println!(
        "License:       {}",
        repository.license.as_deref().unwrap_or("None")
    );
    println!(
        "Created:       {}",
        repository.created_at.format("%Y-%m-%d")
    );
    println!(
        "Last Updated:  {}",
        repository.updated_at.format("%Y-%m-%d")
    );
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
            println!("\n📊 Cache Statistics:\n");
            println!("Repository Cache:");
            println!("  Total entries:   {}", stats.total_entries);
            println!("  Valid entries:   {}", stats.valid_entries);
            println!("  Expired entries: {}", stats.expired_entries);
            println!("\nQuery Cache:");
            println!("  Cached queries:  {}", stats.query_cache_entries);
            println!("  Expired queries: {}", stats.query_cache_expired);
            println!(
                "  Valid queries:   {}",
                stats.query_cache_entries - stats.query_cache_expired
            );
            println!("\nBookmarks:");
            println!("  Total bookmarks: {}", stats.bookmarks_count);
            println!("\nStorage:");
            println!("  Database size:   {} KB", stats.size_bytes / 1024);
            println!("  Location:        {}", cache_path.display());
        }
        CacheAction::Clear => {
            cache.clear()?;
            cache.clear_query_cache()?;
            println!("✅ Cache cleared successfully");
        }
        CacheAction::Cleanup => {
            let deleted_repos = cache.cleanup_expired()?;
            let deleted_queries = cache.cleanup_expired_query_cache()?;
            println!(
                "✅ Cleaned up {} expired repository entries and {} expired query cache entries",
                deleted_repos, deleted_queries
            );
        }
    }

    Ok(())
}

async fn handle_bookmark_command(
    action: BookmarkAction,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    use reposcout_core::models::Repository;

    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    match action {
        BookmarkAction::List => {
            let bookmarks: Vec<Repository> = cache.get_bookmarks()?;

            if bookmarks.is_empty() {
                println!("No bookmarks found. Use 'reposcout bookmark add <repo>' to add one.");
                return Ok(());
            }

            println!("\n📚 Your Bookmarks ({}):\n", bookmarks.len());
            for (i, repo) in bookmarks.iter().enumerate() {
                println!("{}. {} ({})", i + 1, repo.full_name, repo.platform);
                if let Some(desc) = &repo.description {
                    println!("   {}", desc);
                }
                println!(
                    "   ⭐ {} | 🍴 {} | {}",
                    repo.stars,
                    repo.forks,
                    repo.language.as_deref().unwrap_or("Unknown")
                );
                println!("   {}\n", repo.url);
            }
        }
        BookmarkAction::Add { name, tags, notes } => {
            // Parse owner/repo format
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Repository name must be in 'owner/repo' format");
            }

            let (owner, repo_name) = (parts[0], parts[1]);

            // Fetch repository details
            let cache_manager = CacheManager::new(cache_path.to_str().unwrap(), 24)?;
            let mut engine = CachedSearchEngine::with_cache(cache_manager);
            engine.add_provider(Box::new(GitHubProvider::new(github_token)));
            engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));
            engine.add_provider(Box::new(BitbucketProvider::new(
                bitbucket_username,
                bitbucket_app_password,
            )));

            let repository = engine.get_repository(owner, repo_name).await?;

            // Add to bookmarks
            cache.add_bookmark(
                &repository.platform.to_string().to_lowercase(),
                &repository.full_name,
                &repository,
                tags.as_deref(),
                notes.as_deref(),
            )?;

            println!("✅ Bookmarked: {}", repository.full_name);
        }
        BookmarkAction::Remove { name } => {
            // Try to remove from all platforms
            let removed_github = cache.remove_bookmark("github", &name).is_ok();
            let removed_gitlab = cache.remove_bookmark("gitlab", &name).is_ok();
            let removed_bitbucket = cache.remove_bookmark("bitbucket", &name).is_ok();

            if removed_github || removed_gitlab || removed_bitbucket {
                println!("✅ Removed bookmark: {}", name);
            } else {
                println!("❌ Bookmark not found: {}", name);
            }
        }
        BookmarkAction::Export { output, format } => {
            let bookmarks = cache.get_bookmarks_with_metadata()?;

            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&bookmarks)?;
                    std::fs::write(&output, json)?;
                    println!("✅ Exported {} bookmarks to {}", bookmarks.len(), output);
                }
                "csv" => {
                    export_bookmarks_csv(&bookmarks, &output)?;
                    println!("✅ Exported {} bookmarks to {}", bookmarks.len(), output);
                }
                _ => {
                    anyhow::bail!("Unsupported format: {}. Use 'json' or 'csv'", format);
                }
            }
        }
        BookmarkAction::Import { input } => {
            let content = std::fs::read_to_string(&input)?;
            let bookmarks: Vec<BookmarkEntry> = serde_json::from_str(&content)?;

            for entry in &bookmarks {
                let repo: Repository = serde_json::from_str(&entry.data)?;
                cache.add_bookmark(
                    &entry.platform,
                    &entry.full_name,
                    &repo,
                    entry.tags.as_deref(),
                    entry.notes.as_deref(),
                )?;
            }

            println!("✅ Imported {} bookmarks from {}", bookmarks.len(), input);
        }
        BookmarkAction::Clear => {
            cache.clear_bookmarks()?;
            println!("✅ All bookmarks cleared");
        }
    }

    Ok(())
}

async fn handle_history_command(action: HistoryAction) -> anyhow::Result<()> {
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    match action {
        HistoryAction::List { limit } => {
            let history = cache.get_search_history(limit)?;

            if history.is_empty() {
                println!("No search history found. Start searching to build your history!");
                return Ok(());
            }

            println!("\n📜 Recent Search History ({}):\n", history.len());

            for (i, entry) in history.iter().enumerate() {
                // Format timestamp as relative time
                let timestamp = format_timestamp(entry.searched_at);

                println!("{}. \"{}\"", i + 1, entry.query);
                print!("   {}", timestamp);

                if let Some(count) = entry.result_count {
                    print!(" | {} results", count);
                }

                if let Some(filters) = &entry.filters {
                    if !filters.is_empty() {
                        print!(" | filters: {}", filters);
                    }
                }

                println!("\n");
            }
        }
        HistoryAction::Search { term, limit } => {
            let history = cache.search_history(&term, limit)?;

            if history.is_empty() {
                println!("No search history matching '{}'", term);
                return Ok(());
            }

            println!(
                "\n🔍 Search History matching '{}' ({}):\n",
                term,
                history.len()
            );

            for (i, entry) in history.iter().enumerate() {
                let timestamp = format_timestamp(entry.searched_at);

                println!("{}. \"{}\"", i + 1, entry.query);
                print!("   {}", timestamp);

                if let Some(count) = entry.result_count {
                    print!(" | {} results", count);
                }

                if let Some(filters) = &entry.filters {
                    if !filters.is_empty() {
                        print!(" | filters: {}", filters);
                    }
                }

                println!("\n");
            }
        }
        HistoryAction::Clear => {
            let count = cache.search_history_count()?;
            cache.clear_search_history()?;
            println!("✅ Cleared {} search history entries", count);
        }
    }

    Ok(())
}

/// Format Unix timestamp as relative time (e.g., "2 hours ago")
fn format_timestamp(timestamp: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let diff = now - timestamp;

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        let mins = diff / 60;
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if diff < 86400 {
        let hours = diff / 3600;
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if diff < 604800 {
        let days = diff / 86400;
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if diff < 2592000 {
        let weeks = diff / 604800;
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else if diff < 31536000 {
        let months = diff / 2592000;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = diff / 31536000;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}

fn export_bookmarks_csv(bookmarks: &[BookmarkEntry], output: &str) -> anyhow::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(output)?;

    // Write CSV header
    writeln!(
        file,
        "Platform,Repository,Stars,Forks,Language,Description,URL,Bookmarked At,Tags,Notes"
    )?;

    // Write each bookmark
    for entry in bookmarks {
        let repo: reposcout_core::models::Repository = serde_json::from_str(&entry.data)?;
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{}",
            entry.platform,
            entry.full_name,
            repo.stars,
            repo.forks,
            repo.language.as_deref().unwrap_or(""),
            repo.description.as_deref().unwrap_or("").replace(',', ";"),
            repo.url,
            entry.bookmarked_at,
            entry.tags.as_deref().unwrap_or(""),
            entry.notes.as_deref().unwrap_or("").replace(',', ";"),
        )?;
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

/// Build a human-readable filters string for search history
fn build_filters_string(
    language: Option<&str>,
    min_stars: Option<u32>,
    max_stars: Option<u32>,
    pushed: Option<&str>,
    sort: &str,
) -> Option<String> {
    let mut filters = Vec::new();

    if let Some(lang) = language {
        filters.push(format!("lang:{}", lang));
    }

    match (min_stars, max_stars) {
        (Some(min), Some(max)) => filters.push(format!("stars:{}..{}", min, max)),
        (Some(min), None) => filters.push(format!("stars:≥{}", min)),
        (None, Some(max)) => filters.push(format!("stars:≤{}", max)),
        (None, None) => {}
    }

    if let Some(pushed_date) = pushed {
        filters.push(format!("pushed:{}", pushed_date));
    }

    if sort != "stars" {
        filters.push(format!("sort:{}", sort));
    }

    if filters.is_empty() {
        None
    } else {
        Some(filters.join(", "))
    }
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

async fn run_tui_mode(
    mut github_token: Option<String>,
    mut gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    use reposcout_api::{BitbucketClient, GitHubClient, GitLabClient};
    use reposcout_core::TokenStore;
    use reposcout_tui::{run_tui, App};

    // Load tokens from secure storage if not provided via env/CLI
    if let Ok(store) = TokenStore::load() {
        if github_token.is_none() {
            github_token = store.get_token("github");
            if github_token.is_some() {
                tracing::info!("Loaded GitHub token from secure storage");
            }
        }
        if gitlab_token.is_none() {
            gitlab_token = store.get_token("gitlab");
            if gitlab_token.is_some() {
                tracing::info!("Loaded GitLab token from secure storage");
            }
        }
    }

    let mut app = App::new();
    let cache_path = get_cache_path()?;
    let cache_path_str = cache_path.to_str().unwrap().to_string();

    // Create API clients for README fetching
    let github_client = GitHubClient::new(github_token.clone());
    let gitlab_client = GitLabClient::new(gitlab_token.clone());
    let bitbucket_client =
        BitbucketClient::new(bitbucket_username.clone(), bitbucket_app_password.clone());

    // Set platform status based on provided credentials
    // GitHub and GitLab are always available (public repos don't need auth)
    let bitbucket_configured = bitbucket_username.is_some() && bitbucket_app_password.is_some();
    app.set_platform_status(true, true, bitbucket_configured);

    // Create cache manager for bookmarks
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    run_tui(
        app,
        move |query| {
            let github_token_clone = github_token.clone();
            let gitlab_token_clone = gitlab_token.clone();
            let bitbucket_username_clone = bitbucket_username.clone();
            let bitbucket_app_password_clone = bitbucket_app_password.clone();
            let cache_path_clone = cache_path_str.clone();

            Box::pin(async move {
                // Use query-specific cache for accurate, fast results
                // This avoids FTS5 cross-contamination by caching complete result sets per exact query
                let cache = CacheManager::new(&cache_path_clone, 24)?;
                let mut engine = CachedSearchEngine::with_cache(cache);
                // Search across all platforms
                engine.add_provider(Box::new(GitHubProvider::new(github_token_clone)));
                engine.add_provider(Box::new(GitLabProvider::new(gitlab_token_clone)));
                engine.add_provider(Box::new(BitbucketProvider::new(
                    bitbucket_username_clone,
                    bitbucket_app_password_clone,
                )));
                engine.search(query).await.map_err(|e| e.into())
            })
        },
        github_client,
        gitlab_client,
        bitbucket_client,
        cache,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn search_code(
    query: &str,
    limit: usize,
    language: Option<String>,
    repo: Option<String>,
    path: Option<String>,
    extension: Option<String>,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    use reposcout_api::{GitHubClient, GitLabClient};
    use reposcout_core::models::{CodeMatch, CodeSearchResult, Platform};

    // Build enhanced query with filters
    let mut search_query = query.to_string();

    if let Some(lang) = language {
        search_query.push_str(&format!(" language:{}", lang));
    }

    if let Some(repository) = repo {
        search_query.push_str(&format!(" repo:{}", repository));
    }

    if let Some(path_filter) = path {
        search_query.push_str(&format!(" path:{}", path_filter));
    }

    if let Some(ext) = extension {
        search_query.push_str(&format!(" extension:{}", ext));
    }

    tracing::info!("Searching code for: {}", search_query);

    let mut all_results: Vec<CodeSearchResult> = Vec::new();

    // Search GitHub
    if let Some(ref token) = github_token {
        let github_client = GitHubClient::new(Some(token.clone()));
        match github_client.search_code(&search_query, limit as u32).await {
            Ok(items) => {
                for item in items {
                    // Convert GitHub results to our unified format
                    let matches: Vec<CodeMatch> = item
                        .text_matches
                        .iter()
                        .map(|tm| {
                            // GitHub doesn't provide line numbers in the API response
                            CodeMatch {
                                content: tm.fragment.clone(),
                                line_number: 1,
                                context_before: vec![],
                                context_after: vec![],
                            }
                        })
                        .collect();

                    // If no text matches, create a basic match
                    let matches = if matches.is_empty() {
                        vec![CodeMatch {
                            content: format!("Match found in {}", item.path),
                            line_number: 1,
                            context_before: vec![],
                            context_after: vec![],
                        }]
                    } else {
                        matches
                    };

                    all_results.push(CodeSearchResult {
                        platform: Platform::GitHub,
                        repository: item.repository.full_name.clone(),
                        file_path: item.path.clone(),
                        language: None, // Code search API doesn't return language
                        file_url: item.html_url.clone(),
                        repository_url: item.repository.html_url.clone(),
                        matches,
                        repository_stars: 0, // Code search API doesn't return star count
                    });
                }
                tracing::info!("Found {} results from GitHub", all_results.len());
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("Authentication required") {
                    eprintln!("❌ GitHub code search requires authentication.");
                    eprintln!(
                        "   Set GITHUB_TOKEN environment variable or use --github-token flag."
                    );
                    eprintln!("   Example: export GITHUB_TOKEN=your_token_here\n");
                } else if error_str.contains("Rate limit") {
                    eprintln!("❌ GitHub API rate limit exceeded.");
                    eprintln!("   Please wait a few minutes and try again.\n");
                } else {
                    eprintln!("❌ GitHub code search failed: {}\n", error_str);
                }
                tracing::warn!("GitHub code search failed: {}", e);
            }
        }
    } else {
        eprintln!("⚠️  GitHub token not provided. Set GITHUB_TOKEN or use --github-token");
        eprintln!("   Code search requires authentication on GitHub.");
        eprintln!("   Example: export GITHUB_TOKEN=your_token_here\n");
    }

    // Search GitLab
    if let Some(ref token) = gitlab_token {
        let gitlab_client = GitLabClient::new(Some(token.clone()));
        match gitlab_client.search_code(query, limit as u32).await {
            Ok(items) => {
                // We need to fetch project details for each result
                // For now, create basic results
                for item in items {
                    let matches = vec![CodeMatch {
                        content: item.data.clone(),
                        line_number: item.startline,
                        context_before: vec![],
                        context_after: vec![],
                    }];

                    all_results.push(CodeSearchResult {
                        platform: Platform::GitLab,
                        repository: format!("project-{}", item.project_id),
                        file_path: item.path.clone(),
                        language: None,
                        file_url: format!("https://gitlab.com/projects/{}", item.project_id),
                        repository_url: format!("https://gitlab.com/projects/{}", item.project_id),
                        matches,
                        repository_stars: 0,
                    });
                }
                tracing::info!(
                    "Found {} total results (including GitLab)",
                    all_results.len()
                );
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("Authentication required") {
                    eprintln!("❌ GitLab code search requires authentication.");
                    eprintln!(
                        "   Set GITLAB_TOKEN environment variable or use --gitlab-token flag."
                    );
                    eprintln!("   Example: export GITLAB_TOKEN=your_token_here\n");
                } else if error_str.contains("Rate limit") {
                    eprintln!("❌ GitLab API rate limit exceeded.");
                    eprintln!("   Please wait a few minutes and try again.\n");
                } else {
                    eprintln!("❌ GitLab code search failed: {}\n", error_str);
                }
                tracing::warn!("GitLab code search failed: {}", e);
            }
        }
    } else {
        eprintln!("⚠️  GitLab token not provided. Set GITLAB_TOKEN or use --gitlab-token");
        eprintln!("   Code search on GitLab requires authentication.");
        eprintln!("   Example: export GITLAB_TOKEN=your_token_here\n");
    }

    // Search Bitbucket
    if bitbucket_username.is_some() && bitbucket_app_password.is_some() {
        // Note: Bitbucket code search is limited and requires workspace/repo context
        // For now, we'll skip it in multi-platform search
        tracing::info!("Bitbucket code search requires workspace/repo context - skipping in multi-platform search");
    }

    // Display results
    if all_results.is_empty() {
        if github_token.is_none() && gitlab_token.is_none() {
            eprintln!("❌ No code matches found.");
            eprintln!("   Note: Code search requires authentication. Please provide a GitHub or GitLab token.");
        } else {
            println!("No code matches found for '{}'", query);
            println!("Try adjusting your search query or filters.");
        }
        return Ok(());
    }

    // Sort by repository stars
    all_results.sort_by(|a, b| b.repository_stars.cmp(&a.repository_stars));

    println!("\n🔍 Found {} code matches:\n", all_results.len());

    for (i, result) in all_results.iter().take(limit).enumerate() {
        println!("{}. {} ({})", i + 1, result.file_path, result.repository);
        println!(
            "   Platform: {} | ⭐ {}",
            result.platform, result.repository_stars
        );
        if let Some(lang) = &result.language {
            println!("   Language: {}", lang);
        }

        // Show first match snippet
        if let Some(first_match) = result.matches.first() {
            let snippet = if first_match.content.len() > 150 {
                format!("{}...", &first_match.content[..150])
            } else {
                first_match.content.clone()
            };
            println!("   Preview: {}", snippet.replace('\n', " "));
        }

        println!("   {}\n", result.file_url);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn show_trending(
    period_str: &str,
    language: Option<String>,
    min_stars: u32,
    topic: Option<String>,
    limit: usize,
    velocity: bool,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    use reposcout_core::{TrendingFilters, TrendingFinder, TrendingPeriod};

    // Parse period
    let period = match period_str.to_lowercase().as_str() {
        "daily" | "day" | "today" => TrendingPeriod::Daily,
        "weekly" | "week" => TrendingPeriod::Weekly,
        "monthly" | "month" => TrendingPeriod::Monthly,
        _ => {
            anyhow::bail!("Invalid period. Use: daily, weekly, or monthly");
        }
    };

    println!("\n🔥 Trending Repositories - {}\n", period.display_name());

    // Create providers
    let github_provider = GitHubProvider::new(github_token);
    let gitlab_provider = GitLabProvider::new(gitlab_token);
    let bitbucket_provider = BitbucketProvider::new(bitbucket_username, bitbucket_app_password);

    // Create trending finder
    let mut finder = TrendingFinder::new();
    finder.add_provider(&github_provider);
    finder.add_provider(&gitlab_provider);
    finder.add_provider(&bitbucket_provider);

    // Build filters
    let filters = TrendingFilters {
        language: language.clone(),
        min_stars: Some(min_stars),
        topic: topic.clone(),
    };

    // Find trending repos
    let results = if velocity {
        finder.find_trending_by_velocity(period, &filters).await?
    } else {
        finder.find_trending(period, &filters).await?
    };

    if results.is_empty() {
        println!("No trending repositories found for the specified criteria.");
        return Ok(());
    }

    println!("Found {} trending repositories:\n", results.len());

    // Display filters if any
    let mut filter_parts = Vec::new();
    if let Some(ref lang) = language {
        filter_parts.push(format!("Language: {}", lang));
    }
    if min_stars > 0 {
        filter_parts.push(format!("Min Stars: {}", min_stars));
    }
    if let Some(ref t) = topic {
        filter_parts.push(format!("Topic: {}", t));
    }
    if !filter_parts.is_empty() {
        println!("Filters: {}\n", filter_parts.join(" | "));
    }

    if velocity {
        println!("Sorted by: ⚡ Star Velocity (stars/day)\n");
    } else {
        println!("Sorted by: ⭐ Total Stars\n");
    }

    for (i, repo) in results.iter().take(limit).enumerate() {
        // Calculate velocity for display
        let age_days = (chrono::Utc::now() - repo.created_at).num_days().max(1);
        let star_velocity = repo.stars as f64 / age_days as f64;

        println!("{}. {} ({})", i + 1, repo.full_name, repo.platform);
        if let Some(desc) = &repo.description {
            let short_desc = if desc.len() > 100 {
                format!("{}...", &desc[..100])
            } else {
                desc.clone()
            };
            println!("   {}", short_desc);
        }

        println!(
            "   ⭐ {} | 🍴 {} | {} | ⚡ {:.1} stars/day | 📅 {} days old",
            repo.stars,
            repo.forks,
            repo.language.as_deref().unwrap_or("Unknown"),
            star_velocity,
            age_days
        );
        println!("   {}\n", repo.url);
    }

    Ok(())
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

async fn handle_notifications(
    action: NotificationAction,
    github_token: Option<String>,
) -> anyhow::Result<()> {
    let github_token = github_token
        .ok_or_else(|| anyhow::anyhow!("GitHub token required for notifications. Set GITHUB_TOKEN or use Ctrl+S in TUI to save token."))?;

    let client = reposcout_api::GitHubClient::new(Some(github_token));

    match action {
        NotificationAction::List {
            all,
            participating,
            limit,
            repo,
        } => {
            let notifications = client.get_notifications(all, participating, limit).await?;

            // Filter by repo if specified
            let notifications: Vec<_> = if let Some(repo_filter) = repo {
                notifications
                    .into_iter()
                    .filter(|n| n.repository.full_name == repo_filter)
                    .collect()
            } else {
                notifications
            };

            if notifications.is_empty() {
                println!("No notifications found.");
                return Ok(());
            }

            println!("Found {} notification(s)\n", notifications.len());

            for (i, notif) in notifications.iter().enumerate() {
                let unread_marker = if notif.unread { "🔵" } else { "⚪" };
                let reason = notif.reason.as_str();

                println!(
                    "{}. {} {} - {}",
                    i + 1,
                    unread_marker,
                    notif.subject.title,
                    reason
                );
                println!("   Repository: {}", notif.repository.full_name);
                println!("   Type: {}", notif.subject.subject_type);
                println!(
                    "   Updated: {}",
                    notif.updated_at.format("%Y-%m-%d %H:%M:%S")
                );
                println!("   ID: {}", notif.id);

                if let Some(ref desc) = notif.repository.description {
                    let short_desc = if desc.len() > 80 {
                        format!("{}...", &desc[..80])
                    } else {
                        desc.clone()
                    };
                    println!("   {}", short_desc);
                }

                println!();
            }
        }
        NotificationAction::MarkRead { id } => {
            client.mark_notification_read(&id).await?;
            println!("Marked notification {} as read", id);
        }
        NotificationAction::MarkAllRead => {
            client.mark_all_notifications_read().await?;
            println!("Marked all notifications as read");
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_semantic_search(
    query: &str,
    limit: usize,
    hybrid: bool,
    min_similarity: f32,
    export: Option<String>,
    github_token: Option<String>,
    gitlab_token: Option<String>,
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    use reposcout_semantic::{SemanticConfig, SemanticSearchEngine};

    println!("Initializing semantic search engine...");

    // Initialize semantic search engine
    let cache_path = get_cache_path()?;
    let semantic_cache_path = cache_path.join("semantic");

    let config = SemanticConfig {
        enabled: true,
        cache_path: semantic_cache_path.to_string_lossy().to_string(),
        min_similarity,
        max_results: limit * 2, // Get more results for better ranking
        ..Default::default()
    };

    let engine = SemanticSearchEngine::new(config)?;
    engine.initialize().await?;

    println!("Searching with semantic understanding...");

    let results = if hybrid {
        // Perform keyword search first
        let cache = reposcout_cache::CacheManager::new(cache_path.to_str().unwrap(), 24)?;
        let mut keyword_engine = reposcout_core::CachedSearchEngine::with_cache(cache);
        keyword_engine.add_provider(Box::new(GitHubProvider::new(github_token)));
        keyword_engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));
        keyword_engine.add_provider(Box::new(BitbucketProvider::new(
            bitbucket_username,
            bitbucket_app_password,
        )));

        let keyword_results = keyword_engine.search(query).await?;

        // Score keyword results using BM25
        let keyword_pairs = reposcout_semantic::score_keyword_results(keyword_results, query);

        engine.hybrid_search(query, keyword_pairs, limit).await?
    } else {
        engine.search(query, limit).await?
    };

    if results.is_empty() {
        println!("No repositories found for '{}'", query);
        return Ok(());
    }

    // Handle export if requested
    if let Some(export_path) = export {
        use reposcout_core::Exporter;

        let repos: Vec<_> = results.iter().map(|r| r.repository.clone()).collect();
        Exporter::export_to_file(&repos, &export_path)
            .map_err(|e| anyhow::anyhow!("Export failed: {}", e))?;

        println!("✓ Exported {} repositories to {}", repos.len(), export_path);
        return Ok(());
    }

    println!(
        "\nFound {} repositories (semantic search):\n",
        results.len()
    );

    for (i, result) in results.iter().enumerate() {
        let repo = &result.repository;
        println!(
            "{}. {} ({}) [similarity: {:.2}]",
            i + 1,
            repo.full_name,
            repo.platform,
            result.semantic_score
        );

        if let Some(desc) = &repo.description {
            println!("   {}", desc);
        }

        if hybrid {
            if let Some(keyword_score) = result.keyword_score {
                println!(
                    "   Hybrid score: {:.2} (semantic: {:.2}, keyword: {:.2})",
                    result.hybrid_score, result.semantic_score, keyword_score
                );
            }
        }

        println!(
            "   ⭐ {} stars | 🍴 {} forks | 📝 {}",
            repo.stars,
            repo.forks,
            repo.language.as_deref().unwrap_or("Unknown")
        );
        println!("   {}", repo.url);
        println!();
    }

    Ok(())
}

async fn handle_semantic_index(action: &SemanticIndexAction) -> anyhow::Result<()> {
    use reposcout_semantic::{SemanticConfig, SemanticSearchEngine};

    let cache_path = get_cache_path()?;
    let semantic_cache_path = cache_path.join("semantic");

    let config = SemanticConfig {
        enabled: true,
        cache_path: semantic_cache_path.to_string_lossy().to_string(),
        ..Default::default()
    };

    match action {
        SemanticIndexAction::Stats => {
            let engine = SemanticSearchEngine::new(config)?;
            let stats = engine.stats().await;

            println!("\nSemantic Index Statistics:");
            println!("─────────────────────────────");
            println!("Total repositories: {}", stats.total_repositories);
            println!(
                "Index size: {:.2} MB",
                stats.index_size_bytes as f64 / 1_048_576.0
            );
            println!("Model: {}", stats.model_name);
            println!("Vector dimension: {}", stats.dimension);
            println!(
                "Last updated: {}",
                stats.last_updated.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "Created at: {}",
                stats.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }
        SemanticIndexAction::Rebuild { force } => {
            if !force {
                println!("Warning: This will rebuild the entire semantic index.");
                println!("Use --force to confirm.");
                return Ok(());
            }

            println!("Note: Semantic index rebuild from cache is not yet implemented.");
            println!("The index will be automatically built as you search repositories.");
            println!("Use semantic search commands to populate the index.");
        }
        SemanticIndexAction::Clear => {
            let engine = SemanticSearchEngine::new(config)?;
            engine.clear().await?;

            println!("✓ Semantic index cleared");
        }
    }

    Ok(())
}
