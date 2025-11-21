use crate::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Render keybindings help popup
pub fn render_keybindings_help(frame: &mut Frame, app: &App, area: Rect) {
    // Create centered popup (80% width, 85% height)
    let popup_area = centered_rect(80, 85, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Get theme colors
    let bg_color = Color::Rgb(
        app.current_theme.colors.background.r,
        app.current_theme.colors.background.g,
        app.current_theme.colors.background.b,
    );
    let fg_color = Color::Rgb(
        app.current_theme.colors.foreground.r,
        app.current_theme.colors.foreground.g,
        app.current_theme.colors.foreground.b,
    );
    let primary_color = Color::Rgb(
        app.current_theme.colors.primary.r,
        app.current_theme.colors.primary.g,
        app.current_theme.colors.primary.b,
    );
    let accent_color = Color::Rgb(
        app.current_theme.colors.accent.r,
        app.current_theme.colors.accent.g,
        app.current_theme.colors.accent.b,
    );
    let muted_color = Color::Rgb(
        app.current_theme.colors.muted.r,
        app.current_theme.colors.muted.g,
        app.current_theme.colors.muted.b,
    );

    let keybindings = get_keybindings_content(primary_color, accent_color, fg_color, muted_color);

    let help_text = Paragraph::new(keybindings)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Keybindings Help ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(primary_color))
                .style(Style::default().bg(bg_color)),
        )
        .style(Style::default().fg(fg_color).bg(bg_color))
        .alignment(Alignment::Left);

    frame.render_widget(help_text, popup_area);

    // Scrollbar (visual indicator)
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(100).position(0);

    let scrollbar_area = Rect {
        x: popup_area.x + popup_area.width - 1,
        y: popup_area.y + 1,
        width: 1,
        height: popup_area.height - 2,
    };
    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

    // Help text at the very bottom
    let help_area = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + popup_area.height - 1,
        width: popup_area.width - 2,
        height: 1,
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(muted_color)),
        Span::styled(
            "? ",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("or ", Style::default().fg(muted_color)),
        Span::styled(
            "ESC ",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("to close", Style::default().fg(muted_color)),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().bg(bg_color));

    frame.render_widget(footer, help_area);
}

/// Get all keybindings content as styled lines
fn get_keybindings_content(
    primary: Color,
    accent: Color,
    fg: Color,
    muted: Color,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Helper to create a section header
    let section = |title: &str| -> Line<'static> {
        Line::from(vec![Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(Color::Black)
                .bg(primary)
                .add_modifier(Modifier::BOLD),
        )])
    };

    // Helper to create a keybinding line
    let key = |k: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("  {:12}", k),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc.to_string(), Style::default().fg(fg)),
        ])
    };

    // Global Keybindings
    lines.push(section("Global"));
    lines.push(Line::from(""));
    lines.push(key("q", "Quit application"));
    lines.push(key("?", "Toggle this help"));
    lines.push(key("M", "Cycle search mode (Repository > Code > Trending > Notifications > Semantic > Portfolio > Discovery)"));
    lines.push(key("T", "Open theme selector"));
    lines.push(key("Ctrl+R", "Open search history"));
    lines.push(key("Ctrl+S", "Open settings/token manager"));
    lines.push(key("ESC", "Close popup / Clear error / Exit mode"));
    lines.push(Line::from(""));

    // Navigation
    lines.push(section("Navigation"));
    lines.push(Line::from(""));
    lines.push(key("j / Down", "Navigate down / Scroll down"));
    lines.push(key("k / Up", "Navigate up / Scroll up"));
    lines.push(key("TAB", "Cycle preview tabs / Next option"));
    lines.push(key("Shift+TAB", "Previous preview tab"));
    lines.push(key("ENTER", "Confirm / Open in browser / Execute"));
    lines.push(Line::from(""));

    // Repository Search Mode
    lines.push(section("Repository Search"));
    lines.push(Line::from(""));
    lines.push(key("/", "Enter search mode"));
    lines.push(key("f", "Toggle fuzzy search filter"));
    lines.push(key("F", "Toggle filter panel"));
    lines.push(key("b", "Bookmark current repository"));
    lines.push(key("B", "Toggle bookmarks-only view"));
    lines.push(key("r / R", "Fetch and display README"));
    lines.push(key("d", "Fetch dependency information"));
    lines.push(key("c", "Copy package install command (Package tab)"));
    lines.push(key("N", "Create new portfolio"));
    lines.push(key("+", "Add repository to portfolio"));
    lines.push(key("-", "Remove repository from portfolio"));
    lines.push(Line::from(""));

    // Code Search Mode
    lines.push(section("Code Search"));
    lines.push(Line::from(""));
    lines.push(key("/", "Enter search mode"));
    lines.push(key("F", "Toggle code filters"));
    lines.push(key("n", "Navigate to next match in file"));
    lines.push(key("N", "Navigate to previous match in file"));
    lines.push(key("TAB", "Toggle Code/Raw preview modes"));
    lines.push(Line::from(""));

    // Trending Mode
    lines.push(section("Trending"));
    lines.push(Line::from(""));
    lines.push(key("o / O", "Toggle trending options panel"));
    lines.push(key("Space", "Toggle period/velocity option"));
    lines.push(key("+ / =", "Increase minimum stars"));
    lines.push(key("- / _", "Decrease minimum stars"));
    lines.push(key("ENTER", "Execute trending search"));
    lines.push(Line::from(""));

    // Notifications Mode
    lines.push(section("Notifications"));
    lines.push(Line::from(""));
    lines.push(key("m", "Mark selected notification as read"));
    lines.push(key("a", "Mark all notifications as read"));
    lines.push(key("f", "Toggle all/unread filter"));
    lines.push(key("p", "Toggle participating filter"));
    lines.push(Line::from(""));

    // Discovery Mode
    lines.push(section("Discovery"));
    lines.push(Line::from(""));
    lines.push(key("TAB / l", "Next discovery category"));
    lines.push(key("h", "Previous discovery category"));
    lines.push(key("1", "Quick search: New & Notable (7 days)"));
    lines.push(key("2", "Quick search: New & Notable (30 days)"));
    lines.push(key("3", "Quick search: New & Notable (90 days)"));
    lines.push(key("D", "Switch to Discovery mode"));
    lines.push(key("Backspace", "Return to Discovery mode"));
    lines.push(Line::from(""));

    // Portfolio Mode
    lines.push(section("Portfolio"));
    lines.push(Line::from(""));
    lines.push(key("N", "Create new portfolio"));
    lines.push(key("+", "Add repository to selected portfolio"));
    lines.push(key("-", "Remove repository from selected portfolio"));
    lines.push(Line::from(""));

    // Filter/Edit Modes
    lines.push(section("Filter & Edit Modes"));
    lines.push(Line::from(""));
    lines.push(key("ENTER", "Save/confirm value"));
    lines.push(key("ESC", "Cancel/exit mode"));
    lines.push(key("DEL / d", "Clear current filter"));
    lines.push(key("s", "Cycle sort options (in filter mode)"));
    lines.push(key("Backspace", "Delete character"));
    lines.push(Line::from(""));

    // Theme Selector
    lines.push(section("Theme Selector"));
    lines.push(Line::from(""));
    lines.push(key("j / k", "Navigate themes"));
    lines.push(key("ENTER", "Apply selected theme"));
    lines.push(key("ESC", "Close without applying"));
    lines.push(Line::from(""));

    // History Popup
    lines.push(section("History Popup"));
    lines.push(Line::from(""));
    lines.push(key("j / k", "Navigate history entries"));
    lines.push(key("ENTER", "Execute selected query"));
    lines.push(key("ESC", "Close popup"));
    lines.push(Line::from(""));

    // Settings
    lines.push(section("Settings"));
    lines.push(Line::from(""));
    lines.push(key("j / k", "Navigate settings"));
    lines.push(key("ENTER", "Select platform to configure"));
    lines.push(key("ESC", "Close settings"));
    lines.push(Line::from(""));

    // Footer note
    lines.push(Line::from(vec![Span::styled(
        "  Tip: Context-sensitive help is shown in the status bar at the bottom",
        Style::default().fg(muted).add_modifier(Modifier::ITALIC),
    )]));
    lines.push(Line::from(""));

    lines
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
