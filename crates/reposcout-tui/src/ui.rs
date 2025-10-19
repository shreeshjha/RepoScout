// UI rendering logic
use crate::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search input
            Constraint::Min(10),     // Main content
            Constraint::Length(1),   // Status bar
        ])
        .split(frame.area());

    // Render search input
    render_search_input(frame, app, chunks[0]);

    // Split main content into results and preview
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Results list
            Constraint::Percentage(60),  // Preview pane
        ])
        .split(chunks[1]);

    // Render results list
    render_results_list(frame, app, content_chunks[0]);

    // Render preview pane
    render_preview(frame, app, content_chunks[1]);

    // Render status bar
    render_status_bar(frame, app, chunks[2]);
}

fn render_search_input(frame: &mut Frame, app: &App, area: Rect) {
    let input_style = match app.input_mode {
        InputMode::Searching => Style::default().fg(Color::Yellow),
        InputMode::Normal => Style::default(),
    };

    let input = Paragraph::new(app.search_input.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search (ESC to navigate, / to search)")
                .border_style(input_style),
        );

    frame.render_widget(input, area);

    // Show cursor when in search mode
    if app.input_mode == InputMode::Searching {
        frame.set_cursor_position((
            area.x + app.search_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn render_results_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let style = if i == app.selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("⭐ {} ", repo.stars),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(&repo.full_name, style),
            ])];

            ListItem::new(content)
        })
        .collect();

    let title = if app.loading {
        "Results (Loading...)"
    } else {
        &format!("Results ({})", app.results.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(repo) = app.selected_repository() {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                &repo.full_name,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if let Some(desc) = &repo.description {
            lines.push(Line::from(desc.as_str()));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(vec![
            Span::raw("⭐ "),
            Span::styled(
                repo.stars.to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  🍴 "),
            Span::styled(repo.forks.to_string(), Style::default().fg(Color::Blue)),
            Span::raw("  👀 "),
            Span::styled(
                repo.watchers.to_string(),
                Style::default().fg(Color::Green),
            ),
        ]));

        lines.push(Line::from(""));

        if let Some(lang) = &repo.language {
            lines.push(Line::from(vec![
                Span::raw("Language: "),
                Span::styled(lang, Style::default().fg(Color::Magenta)),
            ]));
        }

        if !repo.topics.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("Topics: "),
                Span::styled(
                    repo.topics.join(", "),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("Updated: "),
            Span::styled(
                repo.updated_at.format("%Y-%m-%d").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));

        lines
    } else {
        vec![Line::from("No repository selected")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(error) = &app.error_message {
        Span::styled(error, Style::default().fg(Color::Red))
    } else {
        match app.input_mode {
            InputMode::Searching => {
                Span::styled("SEARCH MODE | ESC: normal mode | ENTER: search", Style::default().fg(Color::Yellow))
            }
            InputMode::Normal => {
                Span::raw("j/k: navigate | /: search | q: quit | ENTER: open in browser")
            }
        }
    };

    let paragraph = Paragraph::new(Line::from(vec![status]));
    frame.render_widget(paragraph, area);
}
