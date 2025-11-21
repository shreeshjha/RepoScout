use crate::{App, DiscoveryCategory};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render discovery categories sidebar
pub fn render_discovery_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let categories = [
        (
            DiscoveryCategory::NewAndNotable,
            "🆕 New & Notable",
            "Recently created repos gaining traction",
        ),
        (
            DiscoveryCategory::HiddenGems,
            "💎 Hidden Gems",
            "Quality repos with low stars",
        ),
        (
            DiscoveryCategory::Topics,
            "🏷️  Topics",
            "Browse by topic categories",
        ),
        (
            DiscoveryCategory::AwesomeLists,
            "⭐ Awesome Lists",
            "Curated awesome-* collections",
        ),
    ];

    let items: Vec<ListItem> = categories
        .iter()
        .map(|(category, name, desc)| {
            let is_selected = app.discovery_category == *category;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let indicator = if is_selected { "▶ " } else { "  " };

            ListItem::new(vec![
                Line::from(vec![Span::styled(format!("{}{}", indicator, name), style)]),
                Line::from(vec![Span::styled(
                    format!("  {}", desc),
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🔍 Discovery Categories")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

/// Render discovery content based on selected category
pub fn render_discovery_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.discovery_category {
        DiscoveryCategory::NewAndNotable => render_new_and_notable(frame, app, area),
        DiscoveryCategory::HiddenGems => render_hidden_gems(frame, app, area),
        DiscoveryCategory::Topics => render_topics(frame, app, area),
        DiscoveryCategory::AwesomeLists => render_awesome_lists(frame, app, area),
    }
}

fn render_new_and_notable(frame: &mut Frame, _app: &App, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "🆕 New & Notable",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Discover recently created repositories gaining traction",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Filter Options:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  • "),
            Span::styled("Last 7 days", Style::default().fg(Color::Green)),
            Span::raw("  (Press "),
            Span::styled(
                "1",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  • "),
            Span::styled("Last 30 days", Style::default().fg(Color::Green)),
            Span::raw(" (Press "),
            Span::styled(
                "2",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  • "),
            Span::styled("Last 90 days", Style::default().fg(Color::Green)),
            Span::raw(" (Press "),
            Span::styled(
                "3",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(")"),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press ENTER to search with current selection",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("New & Notable")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

fn render_hidden_gems(frame: &mut Frame, _app: &App, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "💎 Hidden Gems",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Find quality repositories with low star counts",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Criteria:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::raw("  ✓ Active development (updated recently)")]),
        Line::from(vec![Span::raw("  ✓ Good documentation (README, issues)")]),
        Line::from(vec![Span::raw(
            "  ✓ Community friendly (good-first-issues)",
        )]),
        Line::from(vec![Span::raw("  ✓ Low stars (< 100) but high quality")]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press ENTER to discover hidden gems",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Hidden Gems")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

fn render_topics(frame: &mut Frame, app: &App, area: Rect) {
    let topics = reposcout_core::discovery::popular_topics();

    let mut items: Vec<ListItem> = vec![ListItem::new(vec![
        Line::from(vec![Span::styled(
            "🏷️  Popular Topics",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigate with j/k, press ENTER to explore",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
    ])];

    for (i, (topic, name)) in topics.iter().enumerate() {
        let is_selected = i == app.discovery_cursor;

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let indicator = if is_selected { "▶ " } else { "  " };

        items.push(ListItem::new(vec![Line::from(vec![
            Span::styled(format!("{}{}", indicator, name), style),
            Span::raw(" "),
            Span::styled(format!("({})", topic), Style::default().fg(Color::DarkGray)),
        ])]));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Topics")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_awesome_lists(frame: &mut Frame, app: &App, area: Rect) {
    let awesome_lists = reposcout_core::discovery::awesome_lists();

    let mut items: Vec<ListItem> = vec![ListItem::new(vec![
        Line::from(vec![Span::styled(
            "⭐ Awesome Lists",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Curated lists of awesome resources",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
    ])];

    for (i, (repo, name)) in awesome_lists.iter().enumerate() {
        let is_selected = i == app.discovery_cursor;

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let indicator = if is_selected { "▶ " } else { "  " };

        items.push(ListItem::new(vec![
            Line::from(vec![Span::styled(format!("{}{}", indicator, name), style)]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(*repo, Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
        ]));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Awesome Lists")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}
