// TUI event loop and terminal management
use crate::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use reposcout_api::{GitHubClient, GitLabClient};
use reposcout_cache::CacheManager;

pub async fn run_tui<F>(
    mut app: App,
    mut on_search: F,
    github_client: GitHubClient,
    gitlab_client: GitLabClient,
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
        terminal.draw(|f| crate::ui::render(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.input_mode {
                    InputMode::Searching => match key.code {
                        KeyCode::Enter => {
                            if !app.search_input.is_empty() {
                                app.loading = true;
                                app.enter_normal_mode();

                                // Perform search with filters applied
                                let query = app.get_search_query();
                                match on_search(&query).await {
                                    Ok(results) => {
                                        app.set_results(results);
                                        app.loading = false;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(format!("Search failed: {}", e));
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
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('/') => {
                            app.enter_search_mode();
                        }
                        KeyCode::Char('f') => {
                            // Enter fuzzy search mode
                            if !app.results.is_empty() {
                                app.enter_fuzzy_mode();
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
                        KeyCode::Char('F') => {
                            app.toggle_filters();
                            if app.show_filters {
                                app.enter_filter_mode();
                            }
                        }
                        KeyCode::Tab => {
                            // Tab cycles through preview tabs
                            app.next_preview_tab();
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
                                            _ => Err(anyhow::anyhow!("Platform not supported")),
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

                            // Fetch dependencies for current repository
                            if let Some(repo) = app.selected_repository() {
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
                                                _ => Ok(None),
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
                                                _ => Ok(None),
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
                                                _ => Ok(None),
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
                        KeyCode::Char('j') | KeyCode::Down => {
                            use crate::PreviewMode;
                            // If in README preview mode, scroll instead of navigating
                            if app.preview_mode == PreviewMode::Readme {
                                app.scroll_readme_down();
                            } else {
                                app.next_result();
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            use crate::PreviewMode;
                            // If in README preview mode, scroll instead of navigating
                            if app.preview_mode == PreviewMode::Readme {
                                app.scroll_readme_up();
                            } else {
                                app.previous_result();
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(repo) = app.selected_repository() {
                                // Open in browser
                                let url = repo.url.clone();
                                if let Err(e) = open::that(&url) {
                                    app.error_message = Some(format!("Failed to open browser: {}", e));
                                }
                            }
                        }
                        _ => {}
                    },
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
