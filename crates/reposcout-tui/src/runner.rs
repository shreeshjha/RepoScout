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

                                // Perform search
                                match on_search(&app.search_input).await {
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
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('/') => {
                            app.enter_search_mode();
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
