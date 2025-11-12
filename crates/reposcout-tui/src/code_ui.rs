// Enhanced UI rendering for code search
use crate::{App, CodePreviewMode, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Format large numbers with commas
fn format_number(n: u32) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

/// Render enhanced code results list with filter panel
pub fn render_code_results_list(frame: &mut Frame, app: &App, area: Rect) {
    // Split area to accommodate filter panel if shown
    let (list_area, filter_area) = if app.show_code_filters {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Filter panel
                Constraint::Min(0),     // Results list
            ])
            .split(area);
        (chunks[1], Some(chunks[0]))
    } else {
        (area, None)
    };

    // Render filter panel if visible
    if let Some(filter_rect) = filter_area {
        render_code_filter_panel(frame, app, filter_rect);
    }

    // Show loading message if loading
    if app.loading {
        let loading_text = vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("  🔄 Searching code...", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Please wait while we search across platforms", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(loading_text)
            .block(Block::default().borders(Borders::ALL).title(" Code Results (Loading...) "))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, list_area);
        return;
    }

    // Show empty state with helpful message
    if app.code_results.is_empty() {
        let empty_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  No code results found", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Tips:", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("  • Press 'F' to open filters", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  • Try broader search terms", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  • Check your filter settings", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  • Ensure GitHub/GitLab token is configured", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(empty_text)
            .block(Block::default().borders(Borders::ALL).title(" Code Results (0) "))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, list_area);
        return;
    }

    let items: Vec<ListItem> = app
        .code_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let is_selected = i == app.code_selected_index;

            // Platform badge with color
            let platform_bg = match result.platform {
                reposcout_core::models::Platform::GitHub => Color::Rgb(255, 165, 0), // Orange
                reposcout_core::models::Platform::GitLab => Color::Rgb(252, 109, 38), // GitLab orange
                reposcout_core::models::Platform::Bitbucket => Color::Rgb(33, 136, 255), // Blue
            };

            // Line 1: Index + File path (with icon)
            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };

            // Extract filename and directory
            let (dir, filename) = if let Some(pos) = result.file_path.rfind('/') {
                (&result.file_path[..pos], &result.file_path[pos + 1..])
            } else {
                ("", result.file_path.as_str())
            };

            let line1 = Line::from(vec![
                Span::styled(format!("{:>3}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled("📄 ", Style::default().fg(Color::Blue)),
                Span::styled(filename, name_style.clone()),
                Span::raw(" "),
                Span::styled(
                    if !dir.is_empty() { format!("({})", dir) } else { String::new() },
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            // Line 2: Repository + Platform badge + Stars
            let line2 = Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    format!(" {} ", result.platform),
                    Style::default().fg(Color::Black).bg(platform_bg).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(&result.repository, Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(
                    format!("⭐{}", format_number(result.repository_stars)),
                    Style::default().fg(Color::Rgb(255, 215, 0)),
                ),
            ]);

            // Line 3: Language + match count + first match preview
            let lang_display = result.language.as_deref().unwrap_or("Unknown");
            let match_count = result.matches.len();

            // Get preview of first match
            let preview = if let Some(first_match) = result.matches.first() {
                let content = first_match.content.trim();
                let truncated = if content.len() > 60 {
                    format!("{}...", &content[..60])
                } else {
                    content.to_string()
                };
                truncated.replace('\n', " ")
            } else {
                String::new()
            };

            let line3 = Line::from(vec![
                Span::raw("      "),
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{} ", lang_display),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("• {} match{}", match_count, if match_count == 1 { "" } else { "es" }),
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                ),
            ]);

            // Line 4: Preview of first match
            let line4 = if !preview.is_empty() {
                Line::from(vec![
                    Span::raw("      "),
                    Span::styled("↳ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(preview, Style::default().fg(Color::Rgb(180, 180, 180))),
                ])
            } else {
                Line::from("")
            };

            ListItem::new(vec![line1, line2, line3, line4])
                .style(if is_selected {
                    Style::default().bg(Color::Rgb(40, 40, 60))
                } else {
                    Style::default()
                })
        })
        .collect();

    let title = if app.show_code_filters {
        format!(" Code Results ({}) • Filters ON ", app.code_results.len())
    } else {
        format!(" Code Results ({}) ", app.code_results.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 60, 80))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, list_area);
}

/// Render code filter panel
fn render_code_filter_panel(frame: &mut Frame, app: &App, area: Rect) {
    let filter_fields = vec![
        ("Language", app.code_filters.language.as_deref().unwrap_or("")),
        ("Repository", app.code_filters.repo.as_deref().unwrap_or("")),
        ("Path", app.code_filters.path.as_deref().unwrap_or("")),
        ("Extension", app.code_filters.extension.as_deref().unwrap_or("")),
    ];

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Code Search Filters ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("(↑↓: navigate | Enter: edit | Del: clear | F: close)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    for (idx, (label, value)) in filter_fields.iter().enumerate() {
        let is_active = idx == app.code_filter_cursor;
        let is_editing = is_active && app.input_mode == InputMode::EditingFilter;

        let label_style = if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let value_display = if value.is_empty() { "<not set>" } else { value };
        let value_style = if is_editing {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if is_active {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else if value.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Green)
        };

        let cursor = if is_active { "▸ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(Color::Yellow)),
            Span::styled(format!("{:12} ", label), label_style),
            Span::styled(value_display, value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        );

    frame.render_widget(paragraph, area);
}

/// Render enhanced code preview with tabs and syntax highlighting
pub fn render_code_preview(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(result) = app.selected_code_result() {
        // Split area for tabs and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content
            ])
            .split(area);

        // Render tabs
        render_code_preview_tabs(frame, app, chunks[0]);

        // Render content based on selected tab
        match app.code_preview_mode {
            CodePreviewMode::Code => render_code_tab(frame, app, result, chunks[1]),
            CodePreviewMode::Raw => render_raw_tab(frame, app, result, chunks[1]),
            CodePreviewMode::FileInfo => render_file_info_tab(frame, app, result, chunks[1]),
        }
    } else {
        // No result selected
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No code result selected", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigate results with j/k or ↑↓", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Code Preview ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            )
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
    }
}

/// Render code preview tabs
fn render_code_preview_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = vec![
        ("Code", CodePreviewMode::Code),
        ("Raw", CodePreviewMode::Raw),
        ("File Info", CodePreviewMode::FileInfo),
    ];

    let tab_spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (name, mode))| {
            let is_selected = *mode == app.code_preview_mode;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let mut spans = vec![
                Span::raw(" "),
                Span::styled(format!(" {} ", name), style),
                Span::raw(" "),
            ];

            if i < tabs.len() - 1 {
                spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
            }

            spans
        })
        .collect();

    let tabs_line = Line::from(tab_spans);
    let tabs_widget = Paragraph::new(vec![
        Line::from(""),
        tabs_line,
    ])
    .block(Block::default().borders(Borders::ALL).title("Preview Mode (TAB to switch)"));

    frame.render_widget(tabs_widget, area);
}

/// Render code tab with syntax highlighting
fn render_code_tab(frame: &mut Frame, app: &App, result: &reposcout_core::models::CodeSearchResult, area: Rect) {
    let mut preview_lines: Vec<Line> = vec![];

    // File header with breadcrumb
    preview_lines.push(Line::from(vec![
        Span::styled("📁 ", Style::default().fg(Color::Blue)),
        Span::styled(&result.repository, Style::default().fg(Color::Cyan)),
        Span::styled(" / ", Style::default().fg(Color::DarkGray)),
        Span::styled(&result.file_path, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    preview_lines.push(Line::from(""));

    // Show current match indicator if multiple matches
    if result.matches.len() > 1 {
        preview_lines.push(Line::from(vec![
            Span::styled(
                format!("Match {}/{} ", app.code_match_index + 1, result.matches.len()),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled("(n: next match, N: prev match)", Style::default().fg(Color::DarkGray)),
        ]));
        preview_lines.push(Line::from(""));
    }

    // Show matches with syntax highlighting and line numbers
    for (idx, code_match) in result.matches.iter().enumerate() {
        // Only show current match or all if not too many
        let should_show = if result.matches.len() <= 3 {
            true // Show all if 3 or fewer
        } else {
            idx == app.code_match_index // Show only current match
        };

        if !should_show {
            continue;
        }

        if idx > 0 && result.matches.len() <= 3 {
            preview_lines.push(Line::from(""));
            preview_lines.push(Line::from(vec![
                Span::styled("─".repeat(60), Style::default().fg(Color::DarkGray)),
            ]));
            preview_lines.push(Line::from(""));
        }

        // Match header with line number
        let is_current = idx == app.code_match_index;
        let header_style = if is_current {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let marker = if is_current { "▶ " } else { "  " };

        preview_lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("Line {}", code_match.line_number),
                header_style,
            ),
        ]));
        preview_lines.push(Line::from(""));

        // Syntax-highlighted code with line numbers
        let highlighted = highlight_code_with_line_numbers(
            &code_match.content,
            result.language.as_deref(),
            code_match.line_number as usize,
        );
        preview_lines.extend(highlighted);
        preview_lines.push(Line::from(""));
    }

    // Apply scroll
    let paragraph = Paragraph::new(preview_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Code (Syntax Highlighted) ")
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false })
        .scroll((app.code_scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Render raw text tab
fn render_raw_tab(frame: &mut Frame, app: &App, result: &reposcout_core::models::CodeSearchResult, area: Rect) {
    let mut preview_lines: Vec<Line> = vec![];

    preview_lines.push(Line::from(vec![
        Span::styled(&result.file_path, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));
    preview_lines.push(Line::from(""));

    // Show all matches as plain text
    for (idx, code_match) in result.matches.iter().enumerate() {
        if idx > 0 {
            preview_lines.push(Line::from(""));
            preview_lines.push(Line::from(vec![
                Span::styled("─".repeat(50), Style::default().fg(Color::DarkGray)),
            ]));
            preview_lines.push(Line::from(""));
        }

        preview_lines.push(Line::from(vec![
            Span::styled(
                format!("Line {}", code_match.line_number),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        preview_lines.push(Line::from(""));

        // Plain text, no highlighting
        for line in code_match.content.lines() {
            preview_lines.push(Line::from(line.to_string()));
        }
    }

    let paragraph = Paragraph::new(preview_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Raw Text ")
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false })
        .scroll((app.code_scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Render file info tab
fn render_file_info_tab(frame: &mut Frame, _app: &App, result: &reposcout_core::models::CodeSearchResult, area: Rect) {
    let mut info_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("File Information", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("━".repeat(50), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Path:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.file_path, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Repository:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.repository, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Platform:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", result.platform), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
    ];

    if let Some(lang) = &result.language {
        info_lines.push(Line::from(vec![
            Span::styled("Language:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(lang, Style::default().fg(Color::Green)),
        ]));
        info_lines.push(Line::from(""));
    }

    info_lines.extend(vec![
        Line::from(vec![
            Span::styled("Stars:         ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("⭐ {}", format_number(result.repository_stars)),
                Style::default().fg(Color::Rgb(255, 215, 0)),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Matches:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} match{}", result.matches.len(), if result.matches.len() == 1 { "" } else { "es" }),
                Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("━".repeat(50), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Quick Actions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  • Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("ENTER", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" to open in browser", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  • Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("TAB", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" to switch preview mode", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  • Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("F", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" to toggle filters", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("URL: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.file_url, Style::default().fg(Color::Blue)),
        ]),
    ]);

    let paragraph = Paragraph::new(info_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" File Information ")
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Syntax highlight code with line numbers
fn highlight_code_with_line_numbers(code: &str, language: Option<&str>, start_line: usize) -> Vec<Line<'static>> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];

    let syntax = if let Some(lang) = language {
        ps.find_syntax_by_name(lang)
            .or_else(|| ps.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ps.find_syntax_plain_text())
    } else {
        ps.find_syntax_plain_text()
    };

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result_lines = Vec::new();

    for (line_idx, line) in LinesWithEndings::from(code).enumerate() {
        let line_number = start_line + line_idx;
        let ranges: Vec<(SyntectStyle, &str)> = highlighter
            .highlight_line(line, &ps)
            .unwrap_or_default();

        let mut spans = vec![
            // Line number
            Span::styled(
                format!("{:>4} │ ", line_number),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        // Highlighted code
        for (style, text) in ranges {
            let fg_color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(text.to_string(), Style::default().fg(fg_color)));
        }

        result_lines.push(Line::from(spans));
    }

    result_lines
}
