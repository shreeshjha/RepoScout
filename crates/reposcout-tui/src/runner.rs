// TUI event loop and terminal management
use crate::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub async fn run_tui<F>(mut app: App, mut on_search: F) -> anyhow::Result<()>
where
    F: FnMut(&str) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<reposcout_core::models::Repository>>> + '_>>,
{
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
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('/') => {
                            app.enter_search_mode();
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            app.toggle_filters();
                            if app.show_filters {
                                app.enter_filter_mode();
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            use crate::PreviewMode;

                            // If toggling to README mode and we don't have content, show mock
                            if app.preview_mode == PreviewMode::Stats && app.readme_content.is_none() {
                                // Mock README for demo - in real impl, we'd fetch from API
                                let mock_readme = "# Repository README\n\n\
                                    ## Overview\n\
                                    This is a sample README preview.\n\n\
                                    ### Features\n\
                                    - Feature 1\n\
                                    - Feature 2\n\
                                    - Feature 3\n\n\
                                    ### Installation\n\
                                    ```bash\n\
                                    cargo install repo\n\
                                    ```\n\n\
                                    Press 'r' to toggle back to stats view.\n\n\
                                    **Note**: Real README fetching coming soon!";
                                app.set_readme(mock_readme.to_string());
                            }
                            app.toggle_preview_mode();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next_result();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous_result();
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
