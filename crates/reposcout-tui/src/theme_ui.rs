use crate::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Render theme selector popup
pub fn render_theme_selector(frame: &mut Frame, app: &App, area: Rect) {
    // Create centered popup (60% width, 70% height)
    let popup_area = centered_rect(60, 70, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Split popup into list area and preview area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(popup_area);

    let list_area = chunks[0];

    let themes = reposcout_core::Theme::all_themes();
    let current_theme_name = &app.current_theme.name;

    let items: Vec<ListItem> = themes
        .iter()
        .map(|theme| {
            let is_current = &theme.name == current_theme_name;

            let indicator = if is_current { "● " } else { "  " };

            // Show theme name and color preview
            let preview = format!(
                "{}{} {}",
                indicator,
                theme.name,
                if is_current { "(active)" } else { "" }
            );

            // Create color preview boxes
            let color_preview = format!(
                "  Colors: {}",
                create_color_boxes(&theme.colors)
            );

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(preview, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(color_preview, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Theme Selector")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(68, 71, 90))
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▶ ");

    // Create list state with current selection
    let mut list_state = ListState::default();
    list_state.select(Some(app.theme_selector_index));

    // Render with stateful widget to enable scrolling
    frame.render_stateful_widget(list, list_area, &mut list_state);

    // Render preview of selected theme colors at bottom
    if let Some(selected_theme) = themes.get(app.theme_selector_index) {
        render_theme_preview(frame, selected_theme, chunks[1]);
    }

    // Help text at the very bottom
    let help_area = Rect {
        y: popup_area.y + popup_area.height - 1,
        height: 1,
        ..popup_area
    };

    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k: navigate | ", Style::default().fg(Color::Gray)),
        Span::styled("ENTER: apply | ", Style::default().fg(Color::Yellow)),
        Span::styled("ESC: cancel", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);

    frame.render_widget(help, help_area);
}

/// Render theme color preview
fn render_theme_preview(frame: &mut Frame, theme: &reposcout_core::Theme, area: Rect) {
    let colors = &theme.colors;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Success ", Style::default().bg(to_ratatui_color(&colors.success))),
            Span::styled(" Warning ", Style::default().bg(to_ratatui_color(&colors.warning))),
            Span::styled(" Error ", Style::default().bg(to_ratatui_color(&colors.error))),
            Span::styled(" Info ", Style::default().bg(to_ratatui_color(&colors.info))),
        ]),
        Line::from(vec![
            Span::styled("  Primary ", Style::default().bg(to_ratatui_color(&colors.primary))),
            Span::styled(" Accent ", Style::default().bg(to_ratatui_color(&colors.accent))),
            Span::styled(" Selected ", Style::default().bg(to_ratatui_color(&colors.selected))),
        ]),
    ];

    let preview = Paragraph::new(lines)
        .block(Block::default().borders(Borders::TOP))
        .alignment(Alignment::Center);

    frame.render_widget(preview, area);
}

/// Create color preview boxes as a string
fn create_color_boxes(_colors: &reposcout_core::ThemeColors) -> String {
    // Simple text representation of colors
    format!(
        "■ Primary ■ Success ■ Warning ■ Error"
    )
}

/// Convert our Color to ratatui Color
fn to_ratatui_color(color: &reposcout_core::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
