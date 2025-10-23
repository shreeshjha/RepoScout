// UI rendering logic
use crate::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use chrono::Datelike;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.show_filters {
            vec![
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Search input
                Constraint::Length(9),  // Filters panel
                Constraint::Min(10),    // Main content
                Constraint::Length(1),  // Status bar
            ]
        } else {
            vec![
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Search input
                Constraint::Min(10),    // Main content
                Constraint::Length(1),  // Status bar
            ]
        })
        .split(frame.area());

    // Render header
    render_header(frame, app, chunks[0]);

    // Render search input
    render_search_input(frame, app, chunks[1]);

    let (content_area, status_area) = if app.show_filters {
        // Render filters panel
        render_filters_panel(frame, app, chunks[2]);
        (chunks[3], chunks[4])
    } else {
        (chunks[2], chunks[3])
    };

    // Split main content into results and preview
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Results list
            Constraint::Percentage(60),  // Preview pane
        ])
        .split(content_area);

    // Render results list (needs mutable app for stateful widget)
    render_results_list(frame, app, content_chunks[0]);

    // Render preview pane
    render_preview(frame, app, content_chunks[1]);

    // Render status bar
    render_status_bar(frame, app, status_area);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    // Split header into three sections: left, center, right
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Left: Logo and version
    let logo = vec![
        Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(Color::Cyan)),
            Span::styled("RepoScout", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" v1.0.0", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let logo_widget = Paragraph::new(logo)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default());
    frame.render_widget(logo_widget, header_chunks[0]);

    // Center: Platform status
    let platforms = vec![
        Line::from(vec![
            Span::raw("Platforms: "),
            Span::styled(" GitHub ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(" GitLab ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let platforms_widget = Paragraph::new(platforms)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(platforms_widget, header_chunks[1]);

    // Right: Stats
    let bookmark_count = app.bookmarked.len();
    let result_count = app.results.len();

    let stats = vec![
        Line::from(vec![
            Span::styled("📚 ", Style::default().fg(Color::Magenta)),
            Span::styled(format!("{} ", bookmark_count), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("📊 ", Style::default().fg(Color::Green)),
            Span::styled(format!("{}", result_count), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
    ];
    let stats_widget = Paragraph::new(stats)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(stats_widget, header_chunks[2]);
}

fn render_search_input(frame: &mut Frame, app: &App, area: Rect) {
    let input_style = match app.input_mode {
        InputMode::Searching => Style::default().fg(Color::Yellow),
        InputMode::Normal | InputMode::Filtering | InputMode::EditingFilter => Style::default(),
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

fn render_results_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = i == app.selected_index;
            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Check if this repo is bookmarked
            let bookmark_key = App::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            let is_bookmarked = app.bookmarked.contains(&bookmark_key);

            // Platform color
            let platform_color = match repo.platform {
                reposcout_core::models::Platform::GitHub => Color::Yellow,
                reposcout_core::models::Platform::GitLab => Color::Magenta,
                reposcout_core::models::Platform::Bitbucket => Color::Blue,
            };

            // Line 1: Bookmark + Stats + Name
            let line1 = Line::from(vec![
                Span::styled(
                    if is_bookmarked { "📚" } else { "  " },
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("⭐{}", format_number(repo.stars)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("🍴{}", format_number(repo.forks)),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw("  "),
                Span::styled(&repo.full_name, name_style),
            ]);

            // Line 2: Language + Platform + Updated
            let lang_display = repo.language.as_deref().unwrap_or("Unknown");
            let days_ago = (chrono::Utc::now() - repo.updated_at).num_days();
            let updated_display = if days_ago == 0 {
                "today".to_string()
            } else if days_ago == 1 {
                "1d ago".to_string()
            } else if days_ago < 30 {
                format!("{}d ago", days_ago)
            } else if days_ago < 365 {
                format!("{}mo ago", days_ago / 30)
            } else {
                format!("{}y ago", days_ago / 365)
            };

            let line2 = Line::from(vec![
                Span::raw("     "), // Indent
                Span::styled("●", Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled(lang_display, Style::default().fg(Color::Magenta)),
                Span::raw("  •  "),
                Span::styled(
                    format!(" {} ", repo.platform),
                    Style::default().fg(Color::Black).bg(platform_color),
                ),
                Span::raw("  •  "),
                Span::styled(updated_display, Style::default().fg(Color::DarkGray)),
            ]);

            // Line 3: Description (truncated)
            let description = if let Some(desc) = &repo.description {
                if desc.len() > 60 {
                    format!("     {}...", &desc[..57])
                } else {
                    format!("     {}", desc)
                }
            } else {
                "     No description".to_string()
            };

            let line3 = Line::from(vec![
                Span::styled(description, Style::default().fg(Color::Gray)),
            ]);

            let content = vec![line1, line2, line3];

            ListItem::new(content)
        })
        .collect();

    let title = if app.loading {
        "Results (Loading...)"
    } else if app.show_bookmarks_only {
        &format!("📚 Bookmarks ({})", app.results.len())
    } else {
        &format!("Results ({})", app.results.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Use stateful rendering for proper scrolling
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    use crate::PreviewMode;

    // Split area to show tabs at the top
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(area);

    // Render tab bar
    render_preview_tabs(frame, app, chunks[0]);

    // Render content based on selected tab
    let (content, scroll_offset) = match app.preview_mode {
        PreviewMode::Stats => (render_stats_preview(app), 0),
        PreviewMode::Readme => (render_readme_preview(app), app.readme_scroll),
        PreviewMode::Activity => (render_activity_preview(app), 0),
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(""))
        .wrap(Wrap { trim: true })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, chunks[1]);
}

fn render_preview_tabs(frame: &mut Frame, app: &App, area: Rect) {
    use crate::PreviewMode;

    let tabs = vec![
        ("Stats", PreviewMode::Stats),
        ("README", PreviewMode::Readme),
        ("Activity", PreviewMode::Activity),
    ];

    let tab_spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (name, mode))| {
            let is_selected = *mode == app.preview_mode;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
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
    .block(Block::default().borders(Borders::ALL).title("Preview"))
    .style(Style::default().fg(Color::White));

    frame.render_widget(tabs_widget, area);
}

fn render_stats_preview(app: &App) -> Vec<Line> {
    if let Some(repo) = app.selected_repository() {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                repo.full_name.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if let Some(desc) = &repo.description {
            lines.push(Line::from(desc.clone()));
            lines.push(Line::from(""));
        }

        // Stats with better formatting
        lines.push(Line::from(vec![
            Span::raw("⭐ Stars:     "),
            Span::styled(
                format_number(repo.stars),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("🍴 Forks:     "),
            Span::styled(
                format_number(repo.forks),
                Style::default().fg(Color::Blue),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("👀 Watchers:  "),
            Span::styled(
                format_number(repo.watchers),
                Style::default().fg(Color::Green),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("🐛 Issues:    "),
            Span::styled(
                format_number(repo.open_issues),
                Style::default().fg(Color::Red),
            ),
        ]));

        lines.push(Line::from(""));

        if let Some(lang) = &repo.language {
            lines.push(Line::from(vec![
                Span::raw("💻 Language:  "),
                Span::styled(lang.clone(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));
        }

        if let Some(license) = &repo.license {
            lines.push(Line::from(vec![
                Span::raw("📜 License:   "),
                Span::styled(license.clone(), Style::default().fg(Color::Cyan)),
            ]));
        }

        lines.push(Line::from(""));

        if !repo.topics.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Topics:", Style::default().fg(Color::Gray)),
            ]));

            // Show topics as tags
            let topic_line: Vec<Span> = repo.topics.iter().map(|topic| {
                Span::styled(
                    format!(" {} ", topic),
                    Style::default().fg(Color::Black).bg(Color::Cyan),
                )
            }).collect();
            lines.push(Line::from(topic_line));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(vec![
            Span::raw("📅 Created:   "),
            Span::styled(
                repo.created_at.format("%Y-%m-%d").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("🔄 Updated:   "),
            Span::styled(
                repo.updated_at.format("%Y-%m-%d").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("📌 Pushed:    "),
            Span::styled(
                repo.pushed_at.format("%Y-%m-%d").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("🔗 "),
            Span::styled(
                repo.url.clone(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
            ),
        ]));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No repository selected",
                Style::default().fg(Color::Gray),
            )]),
        ]
    }
}

// Helper function to format numbers with K/M suffixes
fn format_number(num: u32) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

fn render_readme_preview(app: &App) -> Vec<Line> {
    if app.readme_loading {
        return vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "⏳ Loading README...",
                Style::default().fg(Color::Yellow),
            )]),
        ];
    }

    if let Some(readme) = &app.readme_content {
        // Simple markdown-to-text conversion
        readme
            .lines()
            .map(|line| {
                // Basic markdown styling
                if line.starts_with("# ") {
                    Line::from(Span::styled(
                        line.trim_start_matches("# "),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("## ") {
                    Line::from(Span::styled(
                        line.trim_start_matches("## "),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("### ") {
                    Line::from(Span::styled(
                        line.trim_start_matches("### "),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("```") {
                    Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::DarkGray).bg(Color::Black),
                    ))
                } else if line.starts_with("- ") || line.starts_with("* ") {
                    Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::Blue),
                    ))
                } else {
                    Line::from(line)
                }
            })
            .collect()
    } else {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press 'R' to fetch README",
                Style::default().fg(Color::Gray),
            )]),
        ]
    }
}

fn render_activity_preview(app: &App) -> Vec<Line> {
    if let Some(repo) = app.selected_repository() {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                "Repository Activity",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        // Size information
        lines.push(Line::from(vec![
            Span::raw("📦 Size:        "),
            Span::styled(
                format!("{} KB", repo.size),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        // Archive status
        if repo.is_archived {
            lines.push(Line::from(vec![
                Span::styled(
                    "⚠️  ARCHIVED",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - This repository is read-only"),
            ]));
        }

        // Visibility
        lines.push(Line::from(vec![
            Span::raw("🔒 Visibility:  "),
            Span::styled(
                if repo.is_private { "Private" } else { "Public" },
                Style::default().fg(if repo.is_private { Color::Red } else { Color::Green }),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Default Branch",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  🌿 "),
            Span::styled(
                repo.default_branch.clone(),
                Style::default().fg(Color::Green),
            ),
        ]));

        // Homepage
        if let Some(homepage) = &repo.homepage_url {
            if !homepage.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "Homepage",
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  🏠 "),
                    Span::styled(
                        homepage.clone(),
                        Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Platform Info",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Platform badge
        let platform_color = match repo.platform {
            reposcout_core::models::Platform::GitHub => Color::Yellow,
            reposcout_core::models::Platform::GitLab => Color::Magenta,
            reposcout_core::models::Platform::Bitbucket => Color::Blue,
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(" {} ", repo.platform),
                Style::default().fg(Color::Black).bg(platform_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No repository selected",
                Style::default().fg(Color::Gray),
            )]),
        ]
    }
}

fn render_filters_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.input_mode == InputMode::Filtering || app.input_mode == InputMode::EditingFilter;
    let is_editing = app.input_mode == InputMode::EditingFilter;

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let filters = &app.filters;
    let cursor = app.filter_cursor;

    // Helper to get display value (either from edit buffer or actual filter)
    let get_display_value = |field_idx: usize, default_val: &str| -> String {
        if is_editing && cursor == field_idx {
            format!("{}█", app.filter_edit_buffer) // Show cursor
        } else {
            default_val.to_string()
        }
    };

    // Create filter display lines
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "Language:   ",
                if cursor == 0 && is_active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::styled(
                get_display_value(0, filters.language.as_deref().unwrap_or("<none>")),
                if cursor == 0 && is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Min Stars:  ",
                if cursor == 1 && is_active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::styled(
                get_display_value(1, &filters.min_stars.map(|s| s.to_string()).unwrap_or_else(|| "<none>".to_string())),
                if cursor == 1 && is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Max Stars:  ",
                if cursor == 2 && is_active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::styled(
                get_display_value(2, &filters.max_stars.map(|s| s.to_string()).unwrap_or_else(|| "<none>".to_string())),
                if cursor == 2 && is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Pushed:     ",
                if cursor == 3 && is_active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::styled(
                get_display_value(3, filters.pushed.as_deref().unwrap_or("<none>")),
                if cursor == 3 && is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Sort By:    ",
                if cursor == 4 && is_active {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::styled(
                get_display_value(4, &filters.sort_by),
                if cursor == 4 && is_active {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "TAB/arrows: navigate | ENTER: edit | DEL: clear | ESC: close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filters (F to toggle)")
                .border_style(border_style),
        )
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
            InputMode::Filtering => {
                Span::styled("FILTER MODE | TAB/j/k: navigate | ENTER: edit | DEL: clear | ESC: close", Style::default().fg(Color::Yellow))
            }
            InputMode::EditingFilter => {
                Span::styled("EDITING | Type value | ENTER: save | ESC: cancel", Style::default().fg(Color::Green))
            }
            InputMode::Normal => {
                use crate::PreviewMode;
                if app.preview_mode == PreviewMode::Readme {
                    Span::styled("README | j/k: scroll | TAB: next tab | q: quit", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("j/k: navigate | /: search | F: filters | TAB: tabs | b: bookmark | B: view bookmarks | ENTER: open | q: quit")
                }
            }
        }
    };

    let paragraph = Paragraph::new(Line::from(vec![status]));
    frame.render_widget(paragraph, area);
}
