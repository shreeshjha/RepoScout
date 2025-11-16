// TUI event loop and terminal management
use crate::{App, InputMode, SearchMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use reposcout_api::{BitbucketClient, GitHubClient, GitLabClient};
use reposcout_cache::CacheManager;

pub async fn run_tui<F>(
    mut app: App,
    mut on_search: F,
    github_client: GitHubClient,
    gitlab_client: GitLabClient,
    bitbucket_client: BitbucketClient,
    cache: CacheManager,
) -> anyhow::Result<()>
where
    F: FnMut(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<reposcout_core::models::Repository>>> + '_>>,
{
    // Load existing bookmarks
    if let Ok(bookmarks) = cache.get_bookmarks::<reposcout_core::models::Repository>() {
        for repo in bookmarks {
            let key = App::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            app.bookmarked.insert(key);
        }
    }
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        // Clear expired temporary errors
        app.clear_expired_error();

        // Clear and redraw terminal
        terminal.draw(|f| crate::ui::render(f, &mut app))?;

        // Poll for events with timeout to allow periodic error clearing
        if event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::Searching => match key.code {
                        KeyCode::Enter => {
                            if !app.search_input.is_empty() {
                                // Clear any stale state from previous searches
                                app.all_results.clear();
                                app.fuzzy_input.clear();
                                app.results.clear();
                                app.code_results.clear();
                                // Exit bookmarks-only mode when performing a new search
                                app.show_bookmarks_only = false;

                                app.loading = true;
                                app.enter_normal_mode();
                                // Clear terminal before search
                                terminal.clear()?;
                                // Immediately draw loading state
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                match app.search_mode {
                                    SearchMode::Repository | SearchMode::Trending => {
                                        // Perform repository search with filters applied
                                        // (Trending is handled separately via Enter key)
                                        let query = app.get_search_query();
                                        match on_search(&query).await {
                                            Ok(results) => {
                                                // Record search in history
                                                let result_count = results.len();

                                                // Auto-index results for semantic search (in background)
                                                let results_for_indexing = results.clone();
                                                tokio::spawn(async move {
                                                    use reposcout_semantic::{SemanticSearchEngine, SemanticConfig};

                                                    // Get semantic index path (same pattern as CLI)
                                                    if let Some(cache_dir) = dirs_next::cache_dir() {
                                                        let cache_path = cache_dir.join("reposcout").join("reposcout.db");
                                                        let semantic_path = cache_path.join("semantic");

                                                        let config = SemanticConfig {
                                                            cache_path: semantic_path.to_string_lossy().to_string(),
                                                            ..Default::default()
                                                        };

                                                        if let Ok(engine) = SemanticSearchEngine::new(config) {
                                                            if engine.initialize().await.is_ok() {
                                                                let repos_to_index: Vec<(reposcout_core::models::Repository, Option<String>)> =
                                                                    results_for_indexing.into_iter().map(|r| (r, None)).collect();
                                                                let _ = engine.index_repositories(repos_to_index).await;
                                                                tracing::debug!("Auto-indexed {} repositories for semantic search", result_count);
                                                            }
                                                        }
                                                    }
                                                });

                                                app.set_results(results);
                                                app.loading = false;
                                                app.error_message = None;

                                                // Save to search history
                                                if let Err(e) = cache.add_search_history(&app.search_input, None, Some(result_count as i64)) {
                                                    tracing::warn!("Failed to save search history: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                let error_str = e.to_string();
                                                let error_message = if error_str.contains("Network") || error_str.contains("network") {
                                                    "Network error. Check your connection.".to_string()
                                                } else if error_str.len() > 100 {
                                                    format!("{}...", &error_str[..100])
                                                } else {
                                                    error_str
                                                };
                                                app.error_message = Some(error_message);
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    SearchMode::Notifications => {
                                        // Notifications don't have a search box - fetched automatically
                                        app.loading = false;
                                    }
                                    SearchMode::Code => {
                                        // Perform code search
                                        let query = app.get_code_search_query();

                                        // Search GitHub and GitLab for code
                                        let mut all_results = Vec::new();

                                        // Search GitHub
                                        match github_client.search_code(&query, 30).await {
                                            Ok(items) => {
                                                for item in items {
                                                    use reposcout_core::models::{CodeMatch, CodeSearchResult, Platform};

                                                    let matches: Vec<CodeMatch> = item
                                                        .text_matches
                                                        .iter()
                                                        .map(|tm| CodeMatch {
                                                            content: tm.fragment.clone(),
                                                            line_number: 1,
                                                            context_before: vec![],
                                                            context_after: vec![],
                                                        })
                                                        .collect();

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
                                            }
                                            Err(e) => {
                                                let error_str = e.to_string();
                                                let error_message = if error_str.contains("Authentication required") || error_str.contains("401") || error_str.contains("Unauthorized") {
                                                    "Code search requires authentication. Set GITHUB_TOKEN environment variable.".to_string()
                                                } else if error_str.contains("Rate limit") {
                                                    "Rate limit exceeded. Wait a moment and try again.".to_string()
                                                } else if error_str.contains("Network") || error_str.contains("network") {
                                                    "Network error. Check your connection and try again.".to_string()
                                                } else if error_str.contains("decode") || error_str.contains("parse") {
                                                    "API response error. Try again later.".to_string()
                                                } else {
                                                    // Truncate long error messages
                                                    let short_msg = if error_str.len() > 100 {
                                                        format!("{}...", &error_str[..100])
                                                    } else {
                                                        error_str
                                                    };
                                                    format!("Search failed: {}", short_msg)
                                                };
                                                app.error_message = Some(error_message);
                                                app.loading = false;
                                                tracing::warn!("GitHub code search failed: {}", e);
                                                // Don't add any results on error
                                            }
                                        }

                                        // Sort by stars
                                        all_results.sort_by(|a, b| b.repository_stars.cmp(&a.repository_stars));

                                        if all_results.is_empty() {
                                            app.error_message = Some("No code matches found. Try a different search query.".to_string());
                                        }

                                        app.set_code_results(all_results);
                                        app.loading = false;
                                    }
                                    SearchMode::Semantic => {
                                        // Perform hybrid semantic search (keyword + semantic)
                                        let query = app.get_search_query();

                                        // First, do keyword search to get candidates
                                        match on_search(&query).await {
                                            Ok(keyword_results) => {
                                                if keyword_results.is_empty() {
                                                    app.error_message = Some("No repositories found. Try a different query.".to_string());
                                                    app.loading = false;
                                                } else {
                                                    // Now perform hybrid semantic search
                                                    use reposcout_semantic::{SemanticSearchEngine, SemanticConfig};
                                                    let config = SemanticConfig::default();

                                                    match SemanticSearchEngine::new(config) {
                                                        Ok(engine) => {
                                                            match engine.initialize().await {
                                                                Ok(_) => {
                                                                    // Convert to format expected by hybrid_search
                                                                    let keyword_pairs: Vec<(reposcout_core::models::Repository, f32)> = keyword_results
                                                                        .into_iter()
                                                                        .enumerate()
                                                                        .map(|(i, repo)| {
                                                                            let score = 1.0 - (i as f32 / 100.0).min(0.9);
                                                                            (repo, score)
                                                                        })
                                                                        .collect();

                                                                    match engine.hybrid_search(&query, keyword_pairs, 30).await {
                                                                        Ok(results) => {
                                                                            let result_count = results.len();

                                                                            // Convert semantic results to regular repositories
                                                                            let repos: Vec<reposcout_core::models::Repository> =
                                                                                results.into_iter().map(|r| r.repository).collect();

                                                                            app.set_results(repos);
                                                                            app.loading = false;
                                                                            app.error_message = None;

                                                                            // Save to search history
                                                                            if let Err(e) = cache.add_search_history(&app.search_input, None, Some(result_count as i64)) {
                                                                                tracing::warn!("Failed to save search history: {}", e);
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            app.error_message = Some(format!("Semantic search failed: {}", e));
                                                                            app.loading = false;
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    app.error_message = Some(format!("Failed to initialize semantic search: {}", e));
                                                                    app.loading = false;
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            app.error_message = Some(format!("Failed to create semantic engine: {}", e));
                                                            app.loading = false;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Search failed: {}", e));
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    SearchMode::Portfolio => {
                                        // Portfolio mode doesn't perform searches
                                        app.loading = false;
                                    }
                                    SearchMode::Discovery => {
                                        // Discovery mode uses special queries - handled by Enter key
                                        app.loading = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.search_input.pop();
                        }
                        KeyCode::Esc => {
                            app.enter_normal_mode();
                        }
                        _ => {}
                    },
                    InputMode::Filtering => match key.code {
                        KeyCode::Esc => {
                            app.enter_normal_mode();
                        }
                        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                            app.next_filter();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous_filter();
                        }
                        KeyCode::Delete | KeyCode::Char('d') => {
                            app.clear_current_filter();
                        }
                        KeyCode::Enter => {
                            // Enter edit mode for this filter
                            app.enter_editing_filter_mode();
                        }
                        KeyCode::Char('s') if app.filter_cursor == 4 => {
                            // Cycle sort options with 's' key
                            app.cycle_sort();
                        }
                        _ => {}
                    },
                    InputMode::EditingFilter => match key.code {
                        KeyCode::Enter => {
                            app.save_filter_edit();
                        }
                        KeyCode::Esc => {
                            app.cancel_filter_edit();
                        }
                        KeyCode::Char(c) => {
                            app.filter_edit_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.filter_edit_buffer.pop();
                        }
                        _ => {}
                    },
                    InputMode::FuzzySearch => match key.code {
                        KeyCode::Esc => {
                            app.exit_fuzzy_mode();
                        }
                        KeyCode::Char(c) => {
                            app.fuzzy_input.push(c);
                            app.apply_fuzzy_filter();
                        }
                        KeyCode::Backspace => {
                            app.fuzzy_input.pop();
                            app.apply_fuzzy_filter();
                        }
                        _ => {}
                    },
                    InputMode::HistoryPopup => match key.code {
                        KeyCode::Esc => {
                            app.exit_history_popup();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next_history_entry();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous_history_entry();
                        }
                        KeyCode::Enter => {
                            // Apply selected history entry and trigger search
                            if let Some(query) = app.apply_selected_history() {
                                app.exit_history_popup();

                                // Clear any stale state from previous searches
                                app.all_results.clear();
                                app.fuzzy_input.clear();
                                app.results.clear();
                                app.code_results.clear();
                                // Exit bookmarks-only mode when performing a new search
                                app.show_bookmarks_only = false;

                                app.loading = true;
                                app.enter_normal_mode();
                                terminal.clear()?;
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                match app.search_mode {
                                    SearchMode::Repository | SearchMode::Trending => {
                                        let query_str = app.get_search_query();
                                        match on_search(&query_str).await {
                                            Ok(results) => {
                                                // Record search in history
                                                let result_count = results.len();
                                                app.set_results(results);
                                                app.loading = false;
                                                app.error_message = None;

                                                // Save to search history
                                                if let Err(e) = cache.add_search_history(&app.search_input, None, Some(result_count as i64)) {
                                                    tracing::warn!("Failed to save search history: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Search failed: {}", e));
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    SearchMode::Code => {
                                        // Code search not implemented in history yet
                                        app.error_message = Some("Code search history not yet supported".to_string());
                                        app.loading = false;
                                    }
                                    SearchMode::Notifications => {
                                        // Notifications not in search history
                                        app.loading = false;
                                    }
                                    SearchMode::Semantic => {
                                        // Hybrid semantic search from history
                                        let query_str = app.get_search_query();

                                        match on_search(&query_str).await {
                                            Ok(keyword_results) => {
                                                if keyword_results.is_empty() {
                                                    app.error_message = Some("No repositories found".to_string());
                                                    app.loading = false;
                                                } else {
                                                    use reposcout_semantic::{SemanticSearchEngine, SemanticConfig};
                                                    let config = SemanticConfig::default();

                                                    match SemanticSearchEngine::new(config) {
                                                        Ok(engine) => {
                                                            match engine.initialize().await {
                                                                Ok(_) => {
                                                                    let keyword_pairs: Vec<(reposcout_core::models::Repository, f32)> = keyword_results
                                                                        .into_iter()
                                                                        .enumerate()
                                                                        .map(|(i, repo)| {
                                                                            let score = 1.0 - (i as f32 / 100.0).min(0.9);
                                                                            (repo, score)
                                                                        })
                                                                        .collect();

                                                                    match engine.hybrid_search(&query_str, keyword_pairs, 30).await {
                                                                        Ok(results) => {
                                                                            let repos: Vec<reposcout_core::models::Repository> =
                                                                                results.into_iter().map(|r| r.repository).collect();
                                                                            app.set_results(repos);
                                                                            app.loading = false;
                                                                            app.error_message = None;
                                                                        }
                                                                        Err(e) => {
                                                                            app.error_message = Some(format!("Semantic search failed: {}", e));
                                                                            app.loading = false;
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    app.error_message = Some(format!("Failed to initialize: {}", e));
                                                                    app.loading = false;
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            app.error_message = Some(format!("Failed to create engine: {}", e));
                                                            app.loading = false;
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Search failed: {}", e));
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    SearchMode::Portfolio => {
                                        // Portfolio mode doesn't use search history
                                        app.loading = false;
                                    }
                                    SearchMode::Discovery => {
                                        // Discovery mode doesn't use search history
                                        app.loading = false;
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    InputMode::Normal => {
                        // Special handling when theme selector is open
                        if app.show_theme_selector {
                            match key.code {
                                KeyCode::Esc => {
                                    app.show_theme_selector = false;
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    let themes = reposcout_core::Theme::all_themes();
                                    if app.theme_selector_index < themes.len() - 1 {
                                        app.theme_selector_index += 1;
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if app.theme_selector_index > 0 {
                                        app.theme_selector_index -= 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    // Apply selected theme
                                    let themes = reposcout_core::Theme::all_themes();
                                    if let Some(theme) = themes.get(app.theme_selector_index) {
                                        app.set_theme(theme.clone());
                                        app.show_theme_selector = false;
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // Special handling when trending options panel is open
                        if app.show_trending_options && app.search_mode == SearchMode::Trending {
                            match key.code {
                                KeyCode::Esc => {
                                    app.toggle_trending_options(); // Close panel
                                }
                                KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                                    app.next_trending_option();
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.previous_trending_option();
                                }
                                KeyCode::Char(' ') => {
                                    // Toggle based on current option
                                    match app.trending_option_cursor {
                                        0 => app.toggle_trending_period(),
                                        4 => app.toggle_trending_velocity(),
                                        _ => {}
                                    }
                                }
                                KeyCode::Char('+') | KeyCode::Char('=') => {
                                    if app.trending_option_cursor == 2 {
                                        app.increase_trending_min_stars();
                                    }
                                }
                                KeyCode::Char('-') | KeyCode::Char('_') => {
                                    if app.trending_option_cursor == 2 {
                                        app.decrease_trending_min_stars();
                                    }
                                }
                                KeyCode::Char(c) if c.is_alphanumeric() || c == '.' || c == '-' => {
                                    // Edit language or topic
                                    if app.trending_option_cursor == 1 {
                                        // Language
                                        let mut lang = app.trending_filters.language.take().unwrap_or_default();
                                        lang.push(c);
                                        app.trending_filters.language = Some(lang);
                                    } else if app.trending_option_cursor == 3 {
                                        // Topic
                                        let mut topic = app.trending_filters.topic.take().unwrap_or_default();
                                        topic.push(c);
                                        app.trending_filters.topic = Some(topic);
                                    }
                                }
                                KeyCode::Backspace => {
                                    // Clear language or topic
                                    if app.trending_option_cursor == 1 {
                                        if let Some(ref mut lang) = app.trending_filters.language {
                                            lang.pop();
                                            if lang.is_empty() {
                                                app.trending_filters.language = None;
                                            }
                                        }
                                    } else if app.trending_option_cursor == 3 {
                                        if let Some(ref mut topic) = app.trending_filters.topic {
                                            topic.pop();
                                            if topic.is_empty() {
                                                app.trending_filters.topic = None;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    // Trigger trending search
                                    app.toggle_trending_options(); // Close panel
                                    // Fall through to execute search below
                                }
                                _ => {}
                            }
                            // If Enter was pressed, continue to search execution
                            if key.code != KeyCode::Enter {
                                continue;
                            }
                        }

                        // Handle Ctrl+R for history popup
                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
                            // Load search history
                            if let Ok(history) = cache.get_search_history(20) {
                                if !history.is_empty() {
                                    app.load_search_history(history);
                                    app.enter_history_popup();
                                } else {
                                    app.set_temp_error("No search history available (Press Esc to dismiss)".to_string());
                                }
                            } else {
                                app.set_temp_error("Failed to load search history (Press Esc to dismiss)".to_string());
                            }
                            continue;
                        }

                        // Handle Ctrl+S for settings popup
                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                            app.toggle_settings();
                            continue;
                        }

                        match key.code {
                        KeyCode::Esc => {
                            // Clear error message if present
                            if app.error_message.is_some() {
                                app.clear_error();
                            }
                        }
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('M') => {
                            // Toggle between repository, code, trending, and notifications modes
                            app.toggle_search_mode();

                            // Fetch notifications when entering notification mode
                            if app.search_mode == SearchMode::Notifications {
                                app.notifications_loading = true;
                                terminal.clear()?;
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                match github_client.get_notifications(
                                    app.notifications_show_all,
                                    app.notifications_participating,
                                    50
                                ).await {
                                    Ok(notifications) => {
                                        app.notifications = notifications;
                                        app.notifications_selected_index = 0;
                                        app.notifications_loading = false;
                                        app.error_message = None;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to fetch notifications: {}", e));
                                        app.notifications_loading = false;
                                    }
                                }
                            }

                            // Force full redraw
                            terminal.clear()?;
                        }
                        KeyCode::Char('m') => {
                            // Mark selected notification as read (only in notification mode)
                            if app.search_mode == SearchMode::Notifications {
                                if let Some(notif) = app.get_selected_notification() {
                                    let notif_id = notif.id.clone();
                                    match github_client.mark_notification_read(&notif_id).await {
                                        Ok(_) => {
                                            // Refresh notifications
                                            app.notifications_loading = true;
                                            terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                            match github_client.get_notifications(
                                                app.notifications_show_all,
                                                app.notifications_participating,
                                                50
                                            ).await {
                                                Ok(notifications) => {
                                                    app.notifications = notifications;
                                                    app.notifications_loading = false;
                                                    app.error_message = None;
                                                }
                                                Err(e) => {
                                                    app.error_message = Some(format!("Failed to refresh: {}", e));
                                                    app.notifications_loading = false;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            app.error_message = Some(format!("Failed to mark as read: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            // Mark all notifications as read (only in notification mode)
                            if app.search_mode == SearchMode::Notifications {
                                match github_client.mark_all_notifications_read().await {
                                    Ok(_) => {
                                        // Refresh notifications
                                        app.notifications_loading = true;
                                        terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                        match github_client.get_notifications(
                                            app.notifications_show_all,
                                            app.notifications_participating,
                                            50
                                        ).await {
                                            Ok(notifications) => {
                                                app.notifications = notifications;
                                                app.notifications_loading = false;
                                                app.error_message = None;
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Failed to refresh: {}", e));
                                                app.notifications_loading = false;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to mark all as read: {}", e));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            // Toggle participating filter (only in notification mode)
                            if app.search_mode == SearchMode::Notifications {
                                app.toggle_participating_filter();

                                // Refresh notifications with new filter
                                app.notifications_loading = true;
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                match github_client.get_notifications(
                                    app.notifications_show_all,
                                    app.notifications_participating,
                                    50
                                ).await {
                                    Ok(notifications) => {
                                        app.notifications = notifications;
                                        app.notifications_selected_index = 0;
                                        app.notifications_loading = false;
                                        app.error_message = None;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to fetch notifications: {}", e));
                                        app.notifications_loading = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('/') => {
                            // Enter search mode unless in trending/notification mode
                            if app.search_mode != SearchMode::Trending && app.search_mode != SearchMode::Notifications {
                                app.enter_search_mode();
                            }
                        }
                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            // Toggle trending options (only in trending mode)
                            if app.search_mode == SearchMode::Trending {
                                app.toggle_trending_options();
                            }
                        }
                        KeyCode::Enter => {
                            // Trigger trending search when in trending mode
                            if app.search_mode == SearchMode::Trending {
                                app.loading = true;
                                terminal.clear()?;
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                // Execute trending search
                                use reposcout_core::TrendingPeriod as CorePeriod;

                                // Convert TUI period to core period
                                let period = match app.trending_filters.period {
                                    crate::app::TrendingPeriod::Daily => CorePeriod::Daily,
                                    crate::app::TrendingPeriod::Weekly => CorePeriod::Weekly,
                                    crate::app::TrendingPeriod::Monthly => CorePeriod::Monthly,
                                };

                                // Create providers (this is a bit awkward, we need tokens)
                                // For now, we'll use the existing on_search closure approach
                                // But construct a query that triggers trending logic

                                // Build trending query
                                let mut query_parts = vec!["stars:>100".to_string()];

                                // Add date filter based on period
                                let date_filter = match period {
                                    CorePeriod::Daily => "created:>=".to_string() + &(chrono::Utc::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string(),
                                    CorePeriod::Weekly => "created:>=".to_string() + &(chrono::Utc::now() - chrono::Duration::weeks(1)).format("%Y-%m-%d").to_string(),
                                    CorePeriod::Monthly => "created:>=".to_string() + &(chrono::Utc::now() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string(),
                                };
                                query_parts.push(date_filter);

                                if let Some(ref lang) = app.trending_filters.language {
                                    query_parts.push(format!("language:{}", lang));
                                }

                                if app.trending_filters.min_stars > 0 {
                                    query_parts.push(format!("stars:>={}", app.trending_filters.min_stars));
                                }

                                if let Some(ref topic) = app.trending_filters.topic {
                                    query_parts.push(format!("topic:{}", topic));
                                }

                                let query = query_parts.join(" ");

                                match on_search(&query).await {
                                    Ok(mut results) => {
                                        // Sort by velocity if requested
                                        if app.trending_filters.sort_by_velocity {
                                            results.sort_by(|a, b| {
                                                let age_a = (chrono::Utc::now() - a.created_at).num_days().max(1) as f64;
                                                let age_b = (chrono::Utc::now() - b.created_at).num_days().max(1) as f64;
                                                let velocity_a = a.stars as f64 / age_a;
                                                let velocity_b = b.stars as f64 / age_b;
                                                velocity_b.partial_cmp(&velocity_a).unwrap_or(std::cmp::Ordering::Equal)
                                            });
                                        }

                                        app.set_results(results);
                                        app.loading = false;
                                        app.error_message = None;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Trending search failed: {}", e));
                                        app.loading = false;
                                    }
                                }
                            } else if app.search_mode == SearchMode::Discovery {
                                app.set_error("DEBUG: Discovery Enter pressed".to_string());
                                // Trigger search based on discovery category
                                match app.discovery_category {
                                    crate::DiscoveryCategory::NewAndNotable => {
                                        let query = reposcout_core::discovery::new_and_notable_query(None, 30);
                                        app.search_input = query.clone();
                                        app.search_mode = SearchMode::Repository;
                                        app.loading = true;
                                        app.set_error(format!("DEBUG: Searching: {}", query));

                                        match on_search(&query).await {
                                            Ok(results) => {
                                                let count = results.len();
                                                app.set_results(results);
                                                app.selected_index = 0;
                                                app.list_state.select(Some(0));
                                                app.loading = false;
                                                app.set_error(format!("DEBUG: Found {} repos", count));
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Search failed: {}", e));
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    crate::DiscoveryCategory::HiddenGems => {
                                        let query = reposcout_core::discovery::hidden_gems_query(None, 100);
                                        app.search_input = query.clone();
                                        app.search_mode = SearchMode::Repository;
                                        app.loading = true;
                                        app.set_error(format!("DEBUG: Searching gems: {}", query));

                                        match on_search(&query).await {
                                            Ok(results) => {
                                                let count = results.len();
                                                app.set_results(results);
                                                app.selected_index = 0;
                                                app.list_state.select(Some(0));
                                                app.loading = false;
                                                app.set_error(format!("DEBUG: Found {} gems", count));
                                            }
                                            Err(e) => {
                                                app.error_message = Some(format!("Search failed: {}", e));
                                                app.loading = false;
                                            }
                                        }
                                    }
                                    crate::DiscoveryCategory::Topics => {
                                        let topics = reposcout_core::discovery::popular_topics();
                                        if let Some((topic, name)) = topics.get(app.discovery_cursor) {
                                            let query = reposcout_core::discovery::topic_query(topic, 10);
                                            app.search_input = query.clone();
                                            app.search_mode = SearchMode::Repository;
                                            app.loading = true;
                                            app.set_error(format!("DEBUG: Searching topic {}: {}", name, query));

                                            match on_search(&query).await {
                                                Ok(results) => {
                                                    let count = results.len();
                                                    app.set_results(results);
                                                    app.selected_index = 0;
                                                    app.list_state.select(Some(0));
                                                    app.loading = false;
                                                    app.set_error(format!("DEBUG: Found {} for {}", count, name));
                                                }
                                                Err(e) => {
                                                    app.error_message = Some(format!("Search failed: {}", e));
                                                    app.loading = false;
                                                }
                                            }
                                        } else {
                                            app.set_error("DEBUG: No topic selected!".to_string());
                                        }
                                    }
                                    crate::DiscoveryCategory::AwesomeLists => {
                                        let awesome_lists = reposcout_core::discovery::awesome_lists();
                                        if let Some((repo, name)) = awesome_lists.get(app.discovery_cursor) {
                                            let url = format!("https://github.com/{}", repo);
                                            app.set_error(format!("DEBUG: Opening {}", name));
                                            if let Err(e) = open::that(&url) {
                                                app.error_message = Some(format!("Failed to open browser: {}", e));
                                            }
                                        } else {
                                            app.set_error("DEBUG: No list selected!".to_string());
                                        }
                                    }
                                }
                            } else {
                                // Handle opening repos/code/notifications in browser
                                match app.search_mode {
                                    SearchMode::Code => {
                                        if let Some(result) = app.selected_code_result() {
                                            let url = result.file_url.clone();
                                            app.set_error(format!("DEBUG: Opening code at {}", url));
                                            if let Err(e) = open::that(&url) {
                                                app.error_message = Some(format!("Failed to open browser: {}", e));
                                            }
                                        }
                                    }
                                    SearchMode::Repository | SearchMode::Semantic | SearchMode::Portfolio => {
                                        app.set_error(format!("DEBUG: Repo Enter - idx:{} len:{}", app.selected_index, app.results.len()));
                                        if app.preview_mode == crate::PreviewMode::Package {
                                            if let Err(e) = app.open_package_registry() {
                                                app.set_error(e);
                                            }
                                        } else if let Some(repo) = app.selected_repository() {
                                            let url = repo.url.clone();
                                            let repo_name = repo.full_name.clone();
                                            app.set_error(format!("DEBUG: Opening {} at {}", repo_name, url));
                                            if let Err(e) = open::that(&url) {
                                                app.error_message = Some(format!("Failed to open browser: {}", e));
                                            }
                                        } else {
                                            app.set_error(format!("DEBUG ERROR: No repo! idx={} len={}", app.selected_index, app.results.len()));
                                        }
                                    }
                                    SearchMode::Notifications => {
                                        if let Some(notif) = app.get_selected_notification() {
                                            let url = notif.repository.html_url.clone();
                                            app.set_error(format!("DEBUG: Opening notification at {}", url));
                                            if let Err(e) = open::that(&url) {
                                                app.error_message = Some(format!("Failed to open browser: {}", e));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        KeyCode::Char('f') => {
                            if app.search_mode == SearchMode::Notifications {
                                // Toggle all/unread filter in notification mode
                                app.toggle_notification_filter();

                                // Refresh notifications with new filter
                                app.notifications_loading = true;
                                terminal.draw(|f| crate::ui::render(f, &mut app))?;

                                match github_client.get_notifications(
                                    app.notifications_show_all,
                                    app.notifications_participating,
                                    50
                                ).await {
                                    Ok(notifications) => {
                                        app.notifications = notifications;
                                        app.notifications_selected_index = 0;
                                        app.notifications_loading = false;
                                        app.error_message = None;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to fetch notifications: {}", e));
                                        app.notifications_loading = false;
                                    }
                                }
                            } else {
                                // Enter fuzzy search mode in other modes
                                if !app.results.is_empty() {
                                    app.enter_fuzzy_mode();
                                }
                            }
                        }
                        KeyCode::Char('b') => {
                            // Toggle bookmark for current repository
                            if let Some(repo) = app.selected_repository() {
                                let platform = repo.platform.to_string().to_lowercase();
                                let full_name = repo.full_name.clone();
                                let repo_clone = repo.clone();

                                app.toggle_current_bookmark();

                                // Persist to database
                                if app.is_current_bookmarked() {
                                    if let Err(e) = cache.add_bookmark(&platform, &full_name, &repo_clone, None, None) {
                                        app.error_message = Some(format!("Failed to bookmark: {}", e));
                                    }
                                } else {
                                    if let Err(e) = cache.remove_bookmark(&platform, &full_name) {
                                        app.error_message = Some(format!("Failed to remove bookmark: {}", e));
                                    }
                                }
                            }
                        }
                        KeyCode::Char('B') => {
                            // Toggle bookmarks view
                            app.toggle_bookmarks_view();

                            if app.show_bookmarks_only {
                                // Load bookmarks
                                if let Ok(bookmarks) = cache.get_bookmarks::<reposcout_core::models::Repository>() {
                                    app.set_results(bookmarks);
                                }
                            }
                        }
                        KeyCode::Char('T') => {
                            // Toggle theme selector
                            app.show_theme_selector = !app.show_theme_selector;
                            if app.show_theme_selector {
                                // Reset selector index to current theme
                                let themes = reposcout_core::Theme::all_themes();
                                app.theme_selector_index = themes.iter()
                                    .position(|t| t.name == app.current_theme.name)
                                    .unwrap_or(0);
                            }
                        }
                        KeyCode::Char('N') => {
                            if app.search_mode == SearchMode::Code {
                                // Navigate to previous match within current code result
                                app.previous_code_match();
                            } else {
                                // Create new portfolio with default settings
                                let portfolio = app.create_portfolio(
                                    format!("Portfolio {}", app.get_portfolios().len() + 1),
                                    None,
                                    reposcout_core::PortfolioColor::Blue,
                                    reposcout_core::PortfolioIcon::Work,
                                );
                                app.selected_portfolio_id = Some(portfolio.id.clone());
                                app.set_temp_error(format!("Created portfolio: {}", portfolio.name));
                            }
                        }
                        KeyCode::Char('+') => {
                            // Add current repository to selected portfolio
                            if let Some(_repo) = app.selected_repository() {
                                if let Some(portfolio_id) = &app.selected_portfolio_id.clone() {
                                    match app.add_to_portfolio(portfolio_id, None, vec![]) {
                                        Ok(_) => {
                                            app.set_temp_error("Added repository to portfolio".to_string());
                                        }
                                        Err(e) => {
                                            app.set_temp_error(format!("Failed to add: {}", e));
                                        }
                                    }
                                } else {
                                    app.set_temp_error("No portfolio selected. Press N to create one.".to_string());
                                }
                            } else {
                                app.set_temp_error("No repository selected".to_string());
                            }
                        }
                        KeyCode::Char('-') => {
                            // Remove current repository from selected portfolio
                            if let Some(_repo) = app.selected_repository() {
                                if let Some(portfolio_id) = &app.selected_portfolio_id.clone() {
                                    match app.remove_from_portfolio(portfolio_id) {
                                        Ok(_) => {
                                            app.set_temp_error("Removed repository from portfolio".to_string());
                                        }
                                        Err(e) => {
                                            app.set_temp_error(format!("Failed to remove: {}", e));
                                        }
                                    }
                                } else {
                                    app.set_temp_error("No portfolio selected".to_string());
                                }
                            } else {
                                app.set_temp_error("No repository selected".to_string());
                            }
                        }
                        KeyCode::Char('c') => {
                            // Copy install command when in Package preview mode
                            if app.search_mode == SearchMode::Repository ||
                               app.search_mode == SearchMode::Trending ||
                               app.search_mode == SearchMode::Semantic {
                                if app.preview_mode == crate::PreviewMode::Package {
                                    match app.copy_package_install_command() {
                                        Ok(()) => {
                                            app.set_temp_error("Install command copied to clipboard!".to_string());
                                        }
                                        Err(e) => {
                                            app.set_temp_error(e);
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('F') => {
                            // Toggle filters based on search mode
                            if app.search_mode == SearchMode::Code {
                                app.toggle_code_filters();
                            } else {
                                app.toggle_filters();
                                if app.show_filters {
                                    app.enter_filter_mode();
                                }
                            }
                        }
                        KeyCode::Tab => {
                            // Tab cycles through preview tabs/modes based on search mode
                            if app.search_mode == SearchMode::Discovery {
                                // In Discovery mode, Tab switches to next category
                                app.next_discovery_category();
                                app.discovery_cursor = 0; // Reset cursor when switching categories
                            } else if app.search_mode == SearchMode::Code {
                                app.toggle_code_preview_mode();
                            } else {
                                app.next_preview_tab();

                                // If we switched to Package tab, fetch metadata if needed
                                if app.preview_mode == crate::PreviewMode::Package {
                                    if let Some(packages) = app.get_cached_package_info().cloned() {
                                        // Check if we need to fetch metadata
                                        let needs_fetch = packages.iter().any(|pkg| pkg.latest_version.is_none());

                                        if needs_fetch {
                                            app.start_package_loading();

                                            // Spawn task to fetch metadata
                                            let registry_client = reposcout_core::RegistryClient::new();
                                            let mut packages_clone = packages.clone();

                                            tokio::spawn(async move {
                                                for pkg in &mut packages_clone {
                                                    let _ = registry_client.fetch_metadata(pkg).await;
                                                }
                                                packages_clone
                                            });

                                            // Note: We'd need to handle the result and update app state
                                            // For now, this is a basic implementation
                                            app.stop_package_loading();
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::BackTab => {
                            // Shift+Tab cycles backward through preview tabs
                            app.previous_preview_tab();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            use crate::PreviewMode;

                            // If toggling to README mode, fetch if needed
                            if app.preview_mode == PreviewMode::Stats {
                                // Reset scroll position when entering README view
                                app.reset_readme_scroll();

                                if let Some(repo) = app.selected_repository() {
                                    let repo_name = repo.full_name.clone();
                                    let platform = repo.platform;

                                    // Check if already cached
                                    if !app.readme_cache.contains_key(&repo_name) {
                                        // Mark as loading
                                        app.start_readme_loading();
                                        app.toggle_preview_mode();

                                        // Fetch README based on platform
                                        let readme_result: anyhow::Result<String> = match platform {
                                            reposcout_core::models::Platform::GitHub => {
                                                let parts: Vec<&str> = repo_name.split('/').collect();
                                                if parts.len() == 2 {
                                                    github_client.get_readme(parts[0], parts[1]).await.map_err(|e| anyhow::anyhow!("{}", e))
                                                } else {
                                                    Err(anyhow::anyhow!("Invalid repository name format"))
                                                }
                                            }
                                            reposcout_core::models::Platform::GitLab => {
                                                gitlab_client.get_readme(&repo_name).await.map_err(|e| anyhow::anyhow!("{}", e))
                                            }
                                            reposcout_core::models::Platform::Bitbucket => {
                                                let parts: Vec<&str> = repo_name.split('/').collect();
                                                if parts.len() == 2 {
                                                    bitbucket_client.get_readme(parts[0], parts[1]).await.map_err(|e| anyhow::anyhow!("{}", e))
                                                } else {
                                                    Err(anyhow::anyhow!("Invalid repository name format"))
                                                }
                                            }
                                        };

                                        match readme_result {
                                            Ok(readme) => {
                                                app.cache_readme(repo_name, readme.clone());
                                                app.set_readme(readme);
                                            }
                                            Err(e) => {
                                                let error_msg = format!("# README Not Available\n\nFailed to fetch README: {}", e);
                                                app.cache_readme(repo_name, error_msg.clone());
                                                app.set_readme(error_msg);
                                            }
                                        }
                                    } else {
                                        // Load from cache
                                        app.load_readme_for_current();
                                        app.toggle_preview_mode();
                                    }
                                } else {
                                    app.toggle_preview_mode();
                                }
                            } else {
                                // Just toggle back to stats
                                app.toggle_preview_mode();
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            use crate::PreviewMode;

                            // Shift+D: Quick shortcut to Discovery mode
                            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT)
                               && app.search_mode != SearchMode::Discovery {
                                app.search_mode = SearchMode::Discovery;
                                app.results.clear();
                                app.error_message = None;
                                app.discovery_cursor = 0; // Reset cursor
                            } else if let Some(repo) = app.selected_repository() {
                                // Regular 'd': Fetch dependencies for current repository
                                let repo_name = repo.full_name.clone();
                                let platform = repo.platform;
                                let language = repo.language.clone();

                                // Check if already cached
                                if !app.dependencies_cache.contains_key(&repo_name) {
                                    // Switch to dependencies view
                                    app.preview_mode = PreviewMode::Dependencies;
                                    app.start_dependencies_loading();

                                    // Determine which dependency file to fetch based on language
                                    let deps_result: anyhow::Result<Option<reposcout_deps::DependencyInfo>> = match language.as_deref() {
                                        Some("Rust") => {
                                            match platform {
                                                reposcout_core::models::Platform::GitHub => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match github_client.get_cargo_toml(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_cargo_toml(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                                reposcout_core::models::Platform::GitLab => {
                                                    match gitlab_client.get_cargo_toml(&repo_name).await {
                                                        Ok(content) => {
                                                            reposcout_deps::parse_cargo_toml(&content)
                                                                .map(Some)
                                                                .map_err(|e| anyhow::anyhow!("{}", e))
                                                        }
                                                        Err(_) => Ok(None),
                                                    }
                                                }
                                                reposcout_core::models::Platform::Bitbucket => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match bitbucket_client.get_cargo_toml(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_cargo_toml(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                            }
                                        }
                                        Some("JavaScript") | Some("TypeScript") => {
                                            match platform {
                                                reposcout_core::models::Platform::GitHub => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match github_client.get_package_json(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_package_json(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                                reposcout_core::models::Platform::GitLab => {
                                                    match gitlab_client.get_package_json(&repo_name).await {
                                                        Ok(content) => {
                                                            reposcout_deps::parse_package_json(&content)
                                                                .map(Some)
                                                                .map_err(|e| anyhow::anyhow!("{}", e))
                                                        }
                                                        Err(_) => Ok(None),
                                                    }
                                                }
                                                reposcout_core::models::Platform::Bitbucket => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match bitbucket_client.get_package_json(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_package_json(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                            }
                                        }
                                        Some("Python") => {
                                            match platform {
                                                reposcout_core::models::Platform::GitHub => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match github_client.get_requirements_txt(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_requirements_txt(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                                reposcout_core::models::Platform::GitLab => {
                                                    match gitlab_client.get_requirements_txt(&repo_name).await {
                                                        Ok(content) => {
                                                            reposcout_deps::parse_requirements_txt(&content)
                                                                .map(Some)
                                                                .map_err(|e| anyhow::anyhow!("{}", e))
                                                        }
                                                        Err(_) => Ok(None),
                                                    }
                                                }
                                                reposcout_core::models::Platform::Bitbucket => {
                                                    let parts: Vec<&str> = repo_name.split('/').collect();
                                                    if parts.len() == 2 {
                                                        match bitbucket_client.get_requirements_txt(parts[0], parts[1]).await {
                                                            Ok(content) => {
                                                                reposcout_deps::parse_requirements_txt(&content)
                                                                    .map(Some)
                                                                    .map_err(|e| anyhow::anyhow!("{}", e))
                                                            }
                                                            Err(_) => Ok(None),
                                                        }
                                                    } else {
                                                        Err(anyhow::anyhow!("Invalid repository name format"))
                                                    }
                                                }
                                            }
                                        }
                                        _ => Ok(None),
                                    };

                                    match deps_result {
                                        Ok(deps) => {
                                            app.cache_dependencies(repo_name, deps);
                                        }
                                        Err(e) => {
                                            app.error_message = Some(format!("Failed to fetch dependencies: {}", e));
                                            app.cache_dependencies(repo_name, None);
                                        }
                                    }

                                    app.stop_dependencies_loading();
                                } else {
                                    // Already cached, just switch to dependencies view
                                    app.preview_mode = PreviewMode::Dependencies;
                                }
                            }
                        }
                        KeyCode::Char('h') => {
                            // In Discovery mode, go to previous category
                            if app.search_mode == SearchMode::Discovery {
                                app.previous_discovery_category();
                                app.discovery_cursor = 0; // Reset cursor when switching categories
                            }
                        }
                        KeyCode::Char('l') => {
                            // In Discovery mode, go to next category
                            if app.search_mode == SearchMode::Discovery {
                                app.next_discovery_category();
                                app.discovery_cursor = 0; // Reset cursor when switching categories
                            }
                        }
                        KeyCode::Backspace => {
                            // Quick shortcut to return to Discovery mode
                            if app.search_mode != SearchMode::Discovery {
                                app.search_mode = SearchMode::Discovery;
                                app.results.clear();
                                app.error_message = None;
                                app.discovery_cursor = 0; // Reset cursor
                            }
                        }
                        KeyCode::Char('1') => {
                            // In New & Notable, search last 7 days
                            if app.search_mode == SearchMode::Discovery
                               && app.discovery_category == crate::DiscoveryCategory::NewAndNotable {
                                let query = reposcout_core::discovery::new_and_notable_query(None, 7);
                                app.search_input = query.clone();
                                app.search_mode = SearchMode::Repository;
                                app.loading = true;

                                match on_search(&query).await {
                                    Ok(results) => {
                                        app.set_results(results);
                                        app.selected_index = 0;
                                        app.list_state.select(Some(0));
                                        app.loading = false;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Search failed: {}", e));
                                        app.loading = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('2') => {
                            // In New & Notable, search last 30 days
                            if app.search_mode == SearchMode::Discovery
                               && app.discovery_category == crate::DiscoveryCategory::NewAndNotable {
                                let query = reposcout_core::discovery::new_and_notable_query(None, 30);
                                app.search_input = query.clone();
                                app.search_mode = SearchMode::Repository;
                                app.loading = true;

                                match on_search(&query).await {
                                    Ok(results) => {
                                        app.set_results(results);
                                        app.selected_index = 0;
                                        app.list_state.select(Some(0));
                                        app.loading = false;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Search failed: {}", e));
                                        app.loading = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('3') => {
                            // In New & Notable, search last 90 days
                            if app.search_mode == SearchMode::Discovery
                               && app.discovery_category == crate::DiscoveryCategory::NewAndNotable {
                                let query = reposcout_core::discovery::new_and_notable_query(None, 90);
                                app.search_input = query.clone();
                                app.search_mode = SearchMode::Repository;
                                app.loading = true;

                                match on_search(&query).await {
                                    Ok(results) => {
                                        app.set_results(results);
                                        app.selected_index = 0;
                                        app.list_state.select(Some(0));
                                        app.loading = false;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Search failed: {}", e));
                                        app.loading = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            use crate::PreviewMode;
                            match app.search_mode {
                                SearchMode::Code => {
                                    // Scroll code preview or navigate results
                                    if key.code == KeyCode::Down {
                                        app.next_code_result();
                                        app.reset_code_scroll();
                                        app.reset_code_match_index();
                                    } else {
                                        app.scroll_code_down();
                                    }
                                }
                                SearchMode::Repository | SearchMode::Trending | SearchMode::Semantic | SearchMode::Portfolio => {
                                    // If in README preview mode, scroll instead of navigating
                                    if app.preview_mode == PreviewMode::Readme {
                                        app.scroll_readme_down();
                                    } else {
                                        app.next_result();
                                    }
                                }
                                SearchMode::Notifications => {
                                    app.next_notification();
                                }
                                SearchMode::Discovery => {
                                    // Navigate within discovery category items
                                    match app.discovery_category {
                                        crate::DiscoveryCategory::Topics => {
                                            let max = reposcout_core::discovery::popular_topics().len();
                                            if app.discovery_cursor < max.saturating_sub(1) {
                                                app.discovery_cursor += 1;
                                            }
                                        }
                                        crate::DiscoveryCategory::AwesomeLists => {
                                            let max = reposcout_core::discovery::awesome_lists().len();
                                            if app.discovery_cursor < max.saturating_sub(1) {
                                                app.discovery_cursor += 1;
                                            }
                                        }
                                        _ => {} // New & Notable and Hidden Gems don't have navigation
                                    }
                                }
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            use crate::PreviewMode;
                            match app.search_mode {
                                SearchMode::Code => {
                                    // Scroll code preview or navigate results
                                    if key.code == KeyCode::Up {
                                        app.previous_code_result();
                                        app.reset_code_scroll();
                                        app.reset_code_match_index();
                                    } else {
                                        app.scroll_code_up();
                                    }
                                }
                                SearchMode::Repository | SearchMode::Trending | SearchMode::Semantic | SearchMode::Portfolio => {
                                    // If in README preview mode, scroll instead of navigating
                                    if app.preview_mode == PreviewMode::Readme {
                                        app.scroll_readme_up();
                                    } else {
                                        app.previous_result();
                                    }
                                }
                                SearchMode::Notifications => {
                                    app.previous_notification();
                                }
                                SearchMode::Discovery => {
                                    // Navigate within discovery category items
                                    match app.discovery_category {
                                        crate::DiscoveryCategory::Topics | crate::DiscoveryCategory::AwesomeLists => {
                                            if app.discovery_cursor > 0 {
                                                app.discovery_cursor -= 1;
                                            }
                                        }
                                        _ => {} // New & Notable and Hidden Gems don't have navigation
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            // Navigate to next match within current code result
                            if app.search_mode == SearchMode::Code {
                                app.next_code_match();
                            }
                        }
                        _ => {}
                        }
                    },
                    InputMode::Settings => match key.code {
                        KeyCode::Esc => {
                            app.toggle_settings();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous_setting();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next_setting();
                        }
                        KeyCode::Enter => {
                            match app.settings_cursor {
                                0 => app.start_token_input("github"),
                                1 => app.start_token_input("gitlab"),
                                2 => app.start_token_input("bitbucket"),
                                3 => app.toggle_settings(), // Close
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                    InputMode::TokenInput => match key.code {
                        KeyCode::Esc => {
                            app.cancel_token_input();
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.save_token() {
                                app.error_message = Some(format!("Failed to save token: {}", e));
                                app.error_timestamp = Some(std::time::SystemTime::now());
                            }
                        }
                        KeyCode::Char(c) => {
                            app.token_input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.token_input_buffer.pop();
                        }
                        _ => {}
                    },
                }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
