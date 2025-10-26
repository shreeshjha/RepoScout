use clap::Parser;
use reposcout_cache::{BookmarkEntry, CacheManager};
use reposcout_core::{providers::{BitbucketProvider, GitHubProvider, GitLabProvider}, CachedSearchEngine};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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
            show_repository(&name, cli.github_token, cli.gitlab_token, cli.bitbucket_username, cli.bitbucket_app_password).await?;
        }
        Some(Commands::Cache { action }) => {
            handle_cache_command(action).await?;
        }
        Some(Commands::Bookmark { action }) => {
            handle_bookmark_command(action, cli.github_token, cli.gitlab_token, cli.bitbucket_username, cli.bitbucket_app_password).await?;
        }
        Some(Commands::Tui) => {
            run_tui_mode(cli.github_token, cli.gitlab_token, cli.bitbucket_username, cli.bitbucket_app_password).await?;
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
    bitbucket_username: Option<String>,
    bitbucket_app_password: Option<String>,
) -> anyhow::Result<()> {
    // Build GitHub search query with filters
    let search_query = build_github_query(query, language, min_stars, max_stars, pushed);
    tracing::info!("Searching for: {}", search_query);

    // Initialize cache
    let cache_path = get_cache_path()?;
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    let mut engine = CachedSearchEngine::with_cache(cache);
    // Add all providers - search across all platforms
    engine.add_provider(Box::new(GitHubProvider::new(github_token)));
    engine.add_provider(Box::new(GitLabProvider::new(gitlab_token)));
    engine.add_provider(Box::new(BitbucketProvider::new(bitbucket_username, bitbucket_app_password)));

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

async fn show_repository(full_name: &str, github_token: Option<String>, gitlab_token: Option<String>, bitbucket_username: Option<String>, bitbucket_app_password: Option<String>) -> anyhow::Result<()> {
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
    engine.add_provider(Box::new(BitbucketProvider::new(bitbucket_username, bitbucket_app_password)));

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

async fn handle_bookmark_command(action: BookmarkAction, github_token: Option<String>, gitlab_token: Option<String>, bitbucket_username: Option<String>, bitbucket_app_password: Option<String>) -> anyhow::Result<()> {
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
                println!("   ⭐ {} | 🍴 {} | {}",
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
            engine.add_provider(Box::new(BitbucketProvider::new(bitbucket_username, bitbucket_app_password)));

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

fn export_bookmarks_csv(bookmarks: &[BookmarkEntry], output: &str) -> anyhow::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(output)?;

    // Write CSV header
    writeln!(file, "Platform,Repository,Stars,Forks,Language,Description,URL,Bookmarked At,Tags,Notes")?;

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

/// Sort repository results based on user preference
fn sort_results(results: &mut [reposcout_core::models::Repository], sort_by: &str) {
    match sort_by {
        "stars" => results.sort_by(|a, b| b.stars.cmp(&a.stars)),
        "forks" => results.sort_by(|a, b| b.forks.cmp(&a.forks)),
        "updated" => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        _ => {} // Already sorted by relevance from API
    }
}

async fn run_tui_mode(github_token: Option<String>, gitlab_token: Option<String>, bitbucket_username: Option<String>, bitbucket_app_password: Option<String>) -> anyhow::Result<()> {
    use reposcout_tui::{App, run_tui};
    use reposcout_api::{BitbucketClient, GitHubClient, GitLabClient};

    let app = App::new();
    let cache_path = get_cache_path()?;
    let cache_path_str = cache_path.to_str().unwrap().to_string();

    // Create API clients for README fetching
    let github_client = GitHubClient::new(github_token.clone());
    let gitlab_client = GitLabClient::new(gitlab_token.clone());
    let bitbucket_client = BitbucketClient::new(bitbucket_username.clone(), bitbucket_app_password.clone());

    // Create cache manager for bookmarks
    let cache = CacheManager::new(cache_path.to_str().unwrap(), 24)?;

    run_tui(app, move |query| {
        let github_token_clone = github_token.clone();
        let gitlab_token_clone = gitlab_token.clone();
        let bitbucket_username_clone = bitbucket_username.clone();
        let bitbucket_app_password_clone = bitbucket_app_password.clone();
        let cache_path_clone = cache_path_str.clone();

        Box::pin(async move {
            let cache = CacheManager::new(&cache_path_clone, 24)?;
            let mut engine = CachedSearchEngine::with_cache(cache);
            // Search across all platforms
            engine.add_provider(Box::new(GitHubProvider::new(github_token_clone)));
            engine.add_provider(Box::new(GitLabProvider::new(gitlab_token_clone)));
            engine.add_provider(Box::new(BitbucketProvider::new(bitbucket_username_clone, bitbucket_app_password_clone)));
            engine.search(query).await.map_err(|e| e.into())
        })
    }, github_client, gitlab_client, bitbucket_client, cache)
    .await
}

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
                        language: item.repository.language.clone(),
                        file_url: item.html_url.clone(),
                        repository_url: item.repository.html_url.clone(),
                        matches,
                        repository_stars: item.repository.stargazers_count,
                    });
                }
                tracing::info!("Found {} results from GitHub", all_results.len());
            }
            Err(e) => {
                tracing::warn!("GitHub code search failed: {}", e);
            }
        }
    } else {
        println!("⚠️  GitHub token not provided. Set GITHUB_TOKEN or use --github-token");
        println!("   Code search requires authentication on GitHub.\n");
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
                tracing::info!("Found {} total results (including GitLab)", all_results.len());
            }
            Err(e) => {
                tracing::warn!("GitLab code search failed: {}", e);
            }
        }
    }

    // Search Bitbucket
    if bitbucket_username.is_some() && bitbucket_app_password.is_some() {
        // Note: Bitbucket code search is limited and requires workspace/repo context
        // For now, we'll skip it in multi-platform search
        tracing::info!("Bitbucket code search requires workspace/repo context - skipping in multi-platform search");
    }

    // Display results
    if all_results.is_empty() {
        println!("No code matches found for '{}'", query);
        return Ok(());
    }

    // Sort by repository stars
    all_results.sort_by(|a, b| b.repository_stars.cmp(&a.repository_stars));

    println!("\n🔍 Found {} code matches:\n", all_results.len());

    for (i, result) in all_results.iter().take(limit).enumerate() {
        println!(
            "{}. {} ({})",
            i + 1,
            result.file_path,
            result.repository
        );
        println!("   Platform: {} | ⭐ {}", result.platform, result.repository_stars);
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
