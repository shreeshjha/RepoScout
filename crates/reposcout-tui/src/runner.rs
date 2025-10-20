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
        terminal.draw(|f| crate::ui::render(f, &app))?;

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
                            // Edit current filter - open a simple input mode
                            handle_filter_edit(&mut app).await;
                        }
                        KeyCode::Char('s') if app.filter_cursor == 4 => {
                            // Cycle sort options with 's' key
                            app.cycle_sort();
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

// Simple inline filter editing
async fn handle_filter_edit(app: &mut App) {
    let mut input = String::new();

    // Get current value if it exists
    match app.filter_cursor {
        0 => input = app.filters.language.clone().unwrap_or_default(),
        1 => input = app.filters.min_stars.map(|s| s.to_string()).unwrap_or_default(),
        2 => input = app.filters.max_stars.map(|s| s.to_string()).unwrap_or_default(),
        3 => input = app.filters.pushed.clone().unwrap_or_default(),
        4 => {
            // Cycle sort for simplicity
            app.cycle_sort();
            return;
        }
        _ => return,
    }

    // Simple input loop
    loop {
        if let Ok(Event::Key(key)) = event::read() {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Enter => {
                        // Save the input
                        match app.filter_cursor {
                            0 => {
                                app.filters.language = if input.is_empty() {
                                    None
                                } else {
                                    Some(input)
                                };
                            }
                            1 => {
                                app.filters.min_stars = input.parse().ok();
                            }
                            2 => {
                                app.filters.max_stars = input.parse().ok();
                            }
                            3 => {
                                app.filters.pushed = if input.is_empty() {
                                    None
                                } else {
                                    Some(input)
                                };
                            }
                            _ => {}
                        }
                        break;
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}
