use crate::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render portfolio list
pub fn render_portfolio_list(frame: &mut Frame, app: &App, area: Rect) {
    let portfolios = app.get_portfolios();

    let items: Vec<ListItem> = if portfolios.is_empty() {
        vec![
            ListItem::new(Line::from(vec![
                Span::styled("No portfolios yet", Style::default().fg(Color::Gray)),
            ])),
            ListItem::new(Line::from("")),
            ListItem::new(Line::from(vec![
                Span::styled("Press 'N' to create your first portfolio!", Style::default().fg(Color::Yellow)),
            ])),
        ]
    } else {
        portfolios
            .iter()
            .enumerate()
            .map(|(idx, portfolio)| {
                let is_selected = idx == app.portfolio_cursor;
                let style = if is_selected {
                    Style::default().bg(Color::Rgb(68, 71, 90))
                } else {
                    Style::default()
                };

                let icon = portfolio.icon.as_emoji();
                let name = &portfolio.name;
                let repo_count = portfolio.repo_count();
                let total_stars = portfolio.total_stars();

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("{} ", icon), style),
                        Span::styled(name, style.fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("  {} repos  •  ", repo_count), style.fg(Color::Gray)),
                        Span::styled("⭐", style.fg(Color::Yellow)),
                        Span::styled(format!(" {}", total_stars), style.fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                ])
                .style(style)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📁 Portfolios (N: new, +: add repo)")
                .border_style(Style::default().fg(Color::Rgb(249, 226, 175))),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

/// Render portfolio details
pub fn render_portfolio_detail(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    if let Some(portfolio) = app.get_selected_portfolio() {
        // Portfolio header
        lines.push(Line::from(vec![
            Span::styled(portfolio.icon.as_emoji(), Style::default()),
            Span::styled(" ", Style::default()),
            Span::styled(&portfolio.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));

        if let Some(desc) = &portfolio.description {
            lines.push(Line::from(Span::styled(desc, Style::default().fg(Color::Gray))));
            lines.push(Line::from(""));
        }

        // Stats
        lines.push(Line::from(vec![
            Span::styled("Repositories: ", Style::default().fg(Color::Gray)),
            Span::styled(portfolio.repo_count().to_string(), Style::default().fg(Color::Green)),
            Span::styled("  •  ", Style::default().fg(Color::Gray)),
            Span::styled("Total Stars: ", Style::default().fg(Color::Gray)),
            Span::styled(portfolio.total_stars().to_string(), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));

        // Repository list
        if portfolio.repos.is_empty() {
            lines.push(Line::from(Span::styled(
                "No repositories in this portfolio yet",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Navigate to a repository and press '+' to add it",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "📚 Repositories:",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            for watched in &portfolio.repos {
                let repo = &watched.repo;

                // Repo name
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(&repo.full_name, Style::default().fg(Color::Cyan)),
                ]));

                // Stats
                lines.push(Line::from(vec![
                    Span::styled("    ⭐ ", Style::default().fg(Color::Yellow)),
                    Span::styled(repo.stars.to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled("  🍴 ", Style::default().fg(Color::Green)),
                    Span::styled(repo.forks.to_string(), Style::default().fg(Color::Green)),
                ]));

                // Tags if any
                if !watched.tags.is_empty() {
                    let tag_str = watched.tags.join(", ");
                    lines.push(Line::from(vec![
                        Span::styled("    Tags: ", Style::default().fg(Color::Gray)),
                        Span::styled(tag_str, Style::default().fg(Color::Magenta)),
                    ]));
                }

                // Notes if any
                if let Some(notes) = &watched.notes {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(notes, Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
                    ]));
                }

                lines.push(Line::from(""));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Select a portfolio to view details",
            Style::default().fg(Color::Gray),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Portfolio Details")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
