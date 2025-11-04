// UI rendering logic
use crate::{App, InputMode, SearchMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use chrono::Datelike;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub fn render(frame: &mut Frame, app: &mut App) {
    let screen_height = frame.area().height;

    // Dynamic header height: 4 if Bitbucket not configured (extra line for warning), else 3
    let header_height = if !app.platform_status.bitbucket_configured { 4 } else { 3 };

    // Make constraints adaptive to screen size
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.show_filters {
            vec![
                Constraint::Length(header_height.min(screen_height / 6)),  // Header (dynamic)
                Constraint::Length(3.min(screen_height / 8)),  // Search input
                Constraint::Length(9.min(screen_height / 4)),  // Filters panel
                Constraint::Min(5),    // Main content (minimum 5 lines)
                Constraint::Length(1),  // Status bar
            ]
        } else {
            vec![
                Constraint::Length(header_height.min(screen_height / 6)),  // Header (dynamic)
                Constraint::Length(3.min(screen_height / 8)),  // Search input
                Constraint::Min(5),    // Main content (minimum 5 lines)
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
    // Adaptive split: on narrow screens, give more space to results
    let screen_width = frame.area().width;
    let (results_pct, preview_pct) = if screen_width < 100 {
        (50, 50)  // Equal split on narrow screens
    } else if screen_width < 150 {
        (45, 55)  // Slightly favor preview on medium screens
    } else {
        (40, 60)  // More preview space on wide screens
    };

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(results_pct),  // Results list
            Constraint::Percentage(preview_pct),  // Preview pane
        ])
        .split(content_area);

    // Render based on search mode
    match app.search_mode {
        SearchMode::Repository => {
            // Render results list (needs mutable app for stateful widget)
            render_results_list(frame, app, content_chunks[0]);
            // Render preview pane
            render_preview(frame, app, content_chunks[1]);
        }
        SearchMode::Code => {
            // Render code search results
            render_code_results_list(frame, app, content_chunks[0]);
            // Render code preview with syntax highlighting
            render_code_preview(frame, app, content_chunks[1]);
        }
        SearchMode::Trending => {
            // Render trending results (reuse repository results list)
            render_results_list(frame, app, content_chunks[0]);
            // Render preview pane
            render_preview(frame, app, content_chunks[1]);
        }
        SearchMode::Notifications => {
            // Render notifications list
            render_notifications_list(frame, app, content_chunks[0]);
            // Render notification details
            render_notification_preview(frame, app, content_chunks[1]);
        }
    }

    // Render fuzzy search overlay if active
    if app.input_mode == InputMode::FuzzySearch {
        render_fuzzy_search_overlay(frame, app, content_chunks[0]);
    }

    // Render history popup if active
    if app.input_mode == InputMode::HistoryPopup {
        render_history_popup(frame, app, frame.area());
    }

    // Render trending options if active
    if app.show_trending_options && app.search_mode == SearchMode::Trending {
        render_trending_options(frame, app, frame.area());
    }

    // Render settings/token popups if active
    if app.show_settings || app.input_mode == InputMode::Settings || app.input_mode == InputMode::TokenInput {
        if app.input_mode == InputMode::TokenInput {
            render_token_input_popup(app, frame, frame.area());
        } else {
            render_settings_popup(app, frame, frame.area());
        }
    }

    // Render status bar
    render_status_bar(frame, app, status_area);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let screen_width = area.width;

    // Adaptive layout based on screen width
    let header_chunks = if screen_width < 100 {
        // Narrow: Stack vertically or use simpler layout
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ])
            .split(area)
    } else {
        // Normal: Three-column layout
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(area)
    };

    // Left: Logo and version (adaptive)
    let logo_text = if screen_width < 80 {
        "🔍 RS"  // Abbreviated on tiny screens
    } else if screen_width < 100 {
        "🔍 RepoScout"  // No version on small screens
    } else {
        "🔍 RepoScout v1.0.0"  // Full on normal screens
    };

    let logo = vec![Line::from(vec![
        Span::styled(logo_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ])];

    let logo_widget = Paragraph::new(logo)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default());
    frame.render_widget(logo_widget, header_chunks[0]);

    // Center: Search mode and platform status (adaptive)
    let mode_text = if screen_width < 100 {
        match app.search_mode {
            SearchMode::Repository => "Repo",
            SearchMode::Code => "Code",
            SearchMode::Trending => "Trend",
            SearchMode::Notifications => "Notif",
        }
    } else {
        match app.search_mode {
            SearchMode::Repository => "Repository Search",
            SearchMode::Code => "Code Search",
            SearchMode::Trending => "Trending Repos",
            SearchMode::Notifications => "Notifications",
        }
    };
    let mode_color = match app.search_mode {
        SearchMode::Repository => Color::Cyan,
        SearchMode::Code => Color::Green,
        SearchMode::Trending => Color::Magenta,
        SearchMode::Notifications => Color::Yellow,
    };

    // Build platform status indicators (adaptive based on width)
    let mut platform_spans = vec![
        Span::styled(mode_text, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
    ];

    // Only show separator if we have room
    if screen_width > 80 {
        platform_spans.push(Span::raw(" | "));
    } else {
        platform_spans.push(Span::raw(" "));
    }

    // Platform badges - abbreviated on narrow screens
    if screen_width < 100 {
        // Compact mode: just initials with checkmarks
        platform_spans.push(Span::styled(" GH✓ ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)));
        platform_spans.push(Span::styled(" GL✓ ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)));
        if app.platform_status.bitbucket_configured {
            platform_spans.push(Span::styled(" BB✓ ", Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)));
        } else {
            platform_spans.push(Span::styled(" BB✗ ", Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD)));
        }
    } else {
        // Full mode: full names
        platform_spans.push(Span::styled(" GitHub ✓ ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)));
        platform_spans.push(Span::raw(" "));
        platform_spans.push(Span::styled(" GitLab ✓ ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)));
        platform_spans.push(Span::raw(" "));
        if app.platform_status.bitbucket_configured {
            platform_spans.push(Span::styled(" Bitbucket ✓ ", Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)));
        } else {
            platform_spans.push(Span::styled(" Bitbucket ✗ ", Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD)));
        }
    }

    let mut platform_lines = vec![Line::from(platform_spans)];

    // Add Bitbucket warning on separate line (adaptive text)
    if !app.platform_status.bitbucket_configured {
        let warning_text = if screen_width < 120 {
            "⚠ Set BB credentials"  // Short version
        } else {
            "⚠ Set BITBUCKET_USERNAME & BITBUCKET_APP_PASSWORD"  // Full version
        };

        platform_lines.push(Line::from(vec![
            Span::styled(warning_text, Style::default().fg(Color::Yellow)),
        ]));
    }

    let platforms_widget = Paragraph::new(platform_lines)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Center);

    // Render in center area (skip stats on narrow screens)
    if screen_width < 100 {
        // Narrow: platforms take remaining space
        frame.render_widget(platforms_widget, header_chunks[1]);
        return; // Skip stats rendering
    }

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
        InputMode::Normal | InputMode::Filtering | InputMode::EditingFilter | InputMode::FuzzySearch | InputMode::HistoryPopup | InputMode::Settings | InputMode::TokenInput => Style::default(),
    };

    // Different title and content based on search mode
    let (title, content) = match app.search_mode {
        SearchMode::Trending => {
            if app.show_trending_options {
                ("🔥 Trending (Options open - adjust filters)", "Press Enter to search with current filters".to_string())
            } else {
                ("🔥 Trending (Press 'o' for options, Enter to search)",
                 format!("{} | {} | {}+ ⭐",
                    app.trending_filters.period.display_name(),
                    app.trending_filters.language.as_deref().unwrap_or("All languages"),
                    app.trending_filters.min_stars))
            }
        }
        SearchMode::Repository => {
            ("Search (ESC to navigate, / to search)", app.search_input.as_str().to_string())
        }
        SearchMode::Code => {
            ("Code Search (ESC to navigate, / to search)", app.search_input.as_str().to_string())
        }
        SearchMode::Notifications => {
            let filter_info = if app.notifications_show_all {
                "All"
            } else {
                "Unread"
            };
            let participating_info = if app.notifications_participating {
                " | Participating"
            } else {
                ""
            };
            ("📬 Notifications", format!("{}{}", filter_info, participating_info))
        }
    };

    let input = Paragraph::new(content)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(input_style),
        );

    frame.render_widget(input, area);

    // Show cursor when in search mode (not trending)
    if app.input_mode == InputMode::Searching && app.search_mode != SearchMode::Trending {
        frame.set_cursor_position((
            area.x + app.search_input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn render_results_list(frame: &mut Frame, app: &mut App, area: Rect) {
    // Calculate adaptive description length based on area width
    let available_width = area.width.saturating_sub(10); // Account for borders and padding
    let desc_max_length = if available_width < 50 {
        30  // Very narrow
    } else if available_width < 80 {
        40  // Narrow
    } else if available_width < 120 {
        60  // Medium (default)
    } else {
        80  // Wide
    };

    // Show loading message if loading
    if app.loading {
        let loading_text = vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("  🔄 Searching...", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Please wait while we fetch results", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(loading_text)
            .block(Block::default().borders(Borders::ALL).title(" Results (Loading...) "))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = i == app.selected_index;

            // Check if this repo is bookmarked
            let bookmark_key = App::bookmark_key(&repo.platform.to_string().to_lowercase(), &repo.full_name);
            let is_bookmarked = app.bookmarked.contains(&bookmark_key);

            // Platform color for background
            let platform_bg_color = match repo.platform {
                reposcout_core::models::Platform::GitHub => Color::Rgb(255, 165, 0), // Orange for GitHub
                reposcout_core::models::Platform::GitLab => Color::Rgb(252, 109, 38), // GitLab orange
                reposcout_core::models::Platform::Bitbucket => Color::Rgb(33, 136, 255), // Bitbucket blue
            };

            // Line 1: Bookmark + Stats + Name (BRIGHT and DISTINCTIVE)
            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) // Cyan makes repo names stand out
            };

            let line1 = Line::from(vec![
                Span::styled(
                    if is_bookmarked { "📚" } else { "  " },
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("⭐{}", format_number(repo.stars)),
                    Style::default().fg(Color::Rgb(255, 215, 0)), // Gold color for stars
                ),
                Span::raw("  "),
                Span::styled(
                    format!("🍴{}", format_number(repo.forks)),
                    Style::default().fg(Color::Rgb(100, 149, 237)), // Cornflower blue for forks
                ),
                Span::raw("  "),
                Span::styled(&repo.full_name, name_style),
            ]);

            // Line 2: Language + Platform + Updated + Health (MUTED secondary info)
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

            let mut line2_spans = vec![
                Span::raw("     "), // Indent
                Span::styled("●", Style::default().fg(Color::Rgb(147, 112, 219))), // Medium purple
                Span::raw(" "),
                Span::styled(lang_display, Style::default().fg(Color::Rgb(147, 112, 219))),
                Span::raw("  •  "),
                Span::styled(
                    format!(" {} ", repo.platform),
                    Style::default().fg(Color::Black).bg(platform_bg_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  •  "),
                Span::styled(updated_display, Style::default().fg(Color::Rgb(128, 128, 128))), // Medium gray
            ];

            // Add health indicator if available
            if let Some(health) = &repo.health {
                let health_color = match health.status {
                    reposcout_core::HealthStatus::Healthy => Color::Green,
                    reposcout_core::HealthStatus::Moderate => Color::Yellow,
                    reposcout_core::HealthStatus::Warning => Color::Rgb(255, 165, 0), // Orange
                    reposcout_core::HealthStatus::Critical => Color::Red,
                };

                line2_spans.push(Span::raw("  •  "));
                line2_spans.push(Span::styled(
                    format!("{} {}", health.status.emoji(), health.maintenance.label()),
                    Style::default().fg(health_color),
                ));
            }

            let line2 = Line::from(line2_spans);

            // Line 3: Description (VERY MUTED so it doesn't compete with name)
            // Use char_indices() to safely truncate at character boundaries - adaptive
            let description = if let Some(desc) = &repo.description {
                let char_count = desc.chars().count();
                if char_count > desc_max_length as usize {
                    let truncated: String = desc.chars().take(desc_max_length as usize - 3).collect();
                    format!("     {}...", truncated)
                } else {
                    format!("     {}", desc)
                }
            } else {
                "     No description".to_string()
            };

            let line3 = Line::from(vec![
                Span::styled(description, Style::default().fg(Color::Rgb(105, 105, 105))), // Dim gray - very muted
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
        PreviewMode::Dependencies => (render_dependencies_preview(app), 0),
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
        ("Dependencies", PreviewMode::Dependencies),
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

        // Health Metrics Section
        if let Some(health) = &repo.health {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("━━━ Health Metrics ━━━", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            // Overall health score
            let health_color = match health.status {
                reposcout_core::HealthStatus::Healthy => Color::Green,
                reposcout_core::HealthStatus::Moderate => Color::Yellow,
                reposcout_core::HealthStatus::Warning => Color::Rgb(255, 165, 0),
                reposcout_core::HealthStatus::Critical => Color::Red,
            };

            lines.push(Line::from(vec![
                Span::raw("💚 Health:    "),
                Span::styled(
                    format!("{} {} ({}/100)", health.status.emoji(), health.status.label(), health.score),
                    Style::default().fg(health_color).add_modifier(Modifier::BOLD),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("🔧 Maintenance: "),
                Span::styled(
                    format!("{} {}", health.maintenance.emoji(), health.maintenance.label()),
                    Style::default().fg(health_color),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {}", health.maintenance.description()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));

            // Detailed scores breakdown
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Detailed Scores:", Style::default().fg(Color::Gray)),
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Activity:      "),
                Span::styled(
                    format!("{}/30", health.metrics.activity_score),
                    Style::default().fg(Color::Cyan),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Community:     "),
                Span::styled(
                    format!("{}/25", health.metrics.community_score),
                    Style::default().fg(Color::Cyan),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Responsiveness:"),
                Span::styled(
                    format!("{}/20", health.metrics.responsiveness_score),
                    Style::default().fg(Color::Cyan),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Maturity:      "),
                Span::styled(
                    format!("{}/15", health.metrics.maturity_score),
                    Style::default().fg(Color::Cyan),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("  Documentation: "),
                Span::styled(
                    format!("{}/10", health.metrics.documentation_score),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

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

        // Activity Heatmap
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "━━━ Activity Heatmap (Last 12 Months) ━━━",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Generate activity heatmap
        let heatmap_lines = generate_activity_heatmap(repo);
        lines.extend(heatmap_lines);

        // Activity metrics
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "━━━ Activity Summary ━━━",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        let activity_summary_lines = generate_activity_summary(repo);
        lines.extend(activity_summary_lines);

        // Add sparkline visualizations
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "━━━ Trend Sparklines ━━━",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Generate sparklines using repo data
        let activity_sparkline = crate::sparkline::generate_activity_sparkline(
            repo.created_at,
            repo.pushed_at,
            repo.stars,
        );

        let velocity_sparkline = crate::sparkline::generate_star_velocity_sparkline(
            repo.created_at,
            repo.stars,
        );

        let issue_sparkline = crate::sparkline::generate_issue_activity_sparkline(
            repo.open_issues,
            repo.stars,
            repo.created_at,
        );

        // Display sparklines with labels
        lines.push(Line::from(vec![
            Span::raw("  ⚡ Activity Trend:  "),
            Span::styled(
                activity_sparkline,
                Style::default().fg(Color::Green),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  ⭐ Star Velocity:   "),
            Span::styled(
                velocity_sparkline,
                Style::default().fg(Color::Yellow),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  🔧 Issue Activity:  "),
            Span::styled(
                issue_sparkline,
                Style::default().fg(Color::Magenta),
            ),
        ]));

        // Add health trend if health metrics available
        if let Some(health) = &repo.health {
            let health_sparkline = crate::sparkline::generate_health_trend_sparkline(
                health.score,
            );

            lines.push(Line::from(vec![
                Span::raw("  💚 Health Trend:    "),
                Span::styled(
                    health_sparkline,
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Each bar represents a time period (12 total)",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "  ▁▂▃▄▅▆▇█ = Low to High activity",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));

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

fn render_dependencies_preview(app: &App) -> Vec<Line> {
    if app.dependencies_loading {
        return vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Loading dependencies...",
                Style::default().fg(Color::Yellow),
            )]),
        ];
    }

    if let Some(deps_option) = app.get_cached_dependencies() {
        if let Some(deps) = deps_option {
            let mut lines = vec![
                Line::from(vec![Span::styled(
                    format!("{} Dependencies", deps.ecosystem),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            // Summary
            lines.push(Line::from(vec![
                Span::raw("📦 Total:       "),
                Span::styled(
                    deps.total_count.to_string(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("⚙️  Runtime:     "),
                Span::styled(
                    deps.runtime_count.to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw("🔧 Dev:         "),
                Span::styled(
                    deps.dev_count.to_string(),
                    Style::default().fg(Color::Blue),
                ),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Dependencies List",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            // Group dependencies by type
            let runtime_deps: Vec<_> = deps.dependencies.iter()
                .filter(|d| matches!(d.dep_type, reposcout_deps::DependencyType::Runtime))
                .collect();
            let dev_deps: Vec<_> = deps.dependencies.iter()
                .filter(|d| matches!(d.dep_type, reposcout_deps::DependencyType::Dev))
                .collect();
            let build_deps: Vec<_> = deps.dependencies.iter()
                .filter(|d| matches!(d.dep_type, reposcout_deps::DependencyType::Build))
                .collect();

            // Runtime dependencies
            if !runtime_deps.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Runtime:",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )]));
                for dep in runtime_deps.iter().take(20) {
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(
                            dep.name.clone(),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", dep.version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
                if runtime_deps.len() > 20 {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("... and {} more", runtime_deps.len() - 20),
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
                lines.push(Line::from(""));
            }

            // Dev dependencies
            if !dev_deps.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Development:",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )]));
                for dep in dev_deps.iter().take(15) {
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(
                            dep.name.clone(),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", dep.version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
                if dev_deps.len() > 15 {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("... and {} more", dev_deps.len() - 15),
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
                lines.push(Line::from(""));
            }

            // Build dependencies
            if !build_deps.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Build:",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )]));
                for dep in build_deps.iter().take(10) {
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(
                            dep.name.clone(),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", dep.version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
                if build_deps.len() > 10 {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("... and {} more", build_deps.len() - 10),
                            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }

            lines
        } else {
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "No dependency file found",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "This repository doesn't have a supported dependency file:",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::raw("  • Cargo.toml (Rust)")]),
                Line::from(vec![Span::raw("  • package.json (Node.js)")]),
                Line::from(vec![Span::raw("  • requirements.txt (Python)")]),
            ]
        }
    } else if let Some(repo) = app.selected_repository() {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("Press 'd' to analyze dependencies for {}", repo.full_name),
                Style::default().fg(Color::Yellow),
            )]),
        ]
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
        vec![Span::styled(error, Style::default().fg(Color::Red))]
    } else {
        vec![match app.input_mode {
            InputMode::Searching => {
                Span::styled("SEARCH MODE | ESC: normal mode | ENTER: search", Style::default().fg(Color::Yellow))
            }
            InputMode::Filtering => {
                Span::styled("FILTER MODE | TAB/j/k: navigate | ENTER: edit | DEL: clear | ESC: close", Style::default().fg(Color::Yellow))
            }
            InputMode::EditingFilter => {
                Span::styled("EDITING | Type value | ENTER: save | ESC: cancel", Style::default().fg(Color::Green))
            }
            InputMode::FuzzySearch => {
                Span::styled("FUZZY SEARCH | Type to filter | ESC: exit", Style::default().fg(Color::Magenta))
            }
            InputMode::HistoryPopup => {
                Span::styled("HISTORY | j/k: navigate | ENTER: select | ESC: close", Style::default().fg(Color::Cyan))
            }
            InputMode::Settings => {
                Span::styled("SETTINGS | j/k: navigate | ENTER: select platform | ESC: close", Style::default().fg(Color::Cyan))
            }
            InputMode::TokenInput => {
                Span::styled("TOKEN INPUT | Type token | ENTER: save | ESC: cancel", Style::default().fg(Color::Yellow))
            }
            InputMode::Normal => {
                use crate::PreviewMode;
                match app.search_mode {
                    SearchMode::Code => {
                        Span::raw("j/k: navigate | /: search | Ctrl+R: history | Ctrl+S: settings | M: switch mode | TAB: scroll | ENTER: open | q: quit")
                    }
                    SearchMode::Repository => {
                        if app.preview_mode == PreviewMode::Readme {
                            Span::styled("README | j/k: scroll | TAB: next tab | Ctrl+R: history | Ctrl+S: settings | M: switch mode | q: quit", Style::default().fg(Color::Cyan))
                        } else {
                            Span::raw("j/k: navigate | /: search | Ctrl+R: history | Ctrl+S: settings | f: fuzzy | F: filters | M: mode | TAB: tabs | b: bookmark | q: quit")
                        }
                    }
                    SearchMode::Trending => {
                        Span::styled("o: options | ENTER: search | j/k: navigate | Ctrl+S: settings | M: mode | TAB: tabs | q: quit", Style::default().fg(Color::Magenta))
                    }
                    SearchMode::Notifications => {
                        Span::styled("j/k: navigate | m: mark read | a: mark all | f: filter | p: participating | ENTER: open | M: mode | q: quit", Style::default().fg(Color::Yellow))
                    }
                }
            }
        }]
    };

    let paragraph = Paragraph::new(Line::from(status));
    frame.render_widget(paragraph, area);
}

fn render_fuzzy_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    // Create a centered overlay area at the top of the results list
    let overlay_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: 3,
    };

    // Fuzzy search input box
    let fuzzy_text = vec![
        Line::from(vec![
            Span::styled("🔍 Fuzzy Filter: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled(&app.fuzzy_input, Style::default().fg(Color::Yellow)),
            Span::styled("█", Style::default().fg(Color::Yellow)), // Cursor
        ]),
    ];

    let match_info = if app.fuzzy_input.is_empty() {
        format!("{} results", app.all_results.len())
    } else {
        format!("{}/{} matches", app.fuzzy_match_count, app.all_results.len())
    };

    let fuzzy_widget = Paragraph::new(fuzzy_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(match_info)
                .title_alignment(ratatui::layout::Alignment::Right)
                .border_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Black))
        );

    frame.render_widget(fuzzy_widget, overlay_area);
}

fn render_code_results_list(frame: &mut Frame, app: &App, area: Rect) {
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
                Span::styled("  Please wait while we search", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(loading_text)
            .block(Block::default().borders(Borders::ALL).title(" Code Results (Loading...) "))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .code_results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let is_selected = i == app.code_selected_index;

            // Platform color
            let platform_color = match result.platform {
                reposcout_core::models::Platform::GitHub => Color::Yellow,
                reposcout_core::models::Platform::GitLab => Color::Magenta,
                reposcout_core::models::Platform::Bitbucket => Color::Rgb(33, 136, 255),
            };

            // Line 1: File path (highlighted if selected)
            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };

            let line1 = Line::from(vec![
                Span::styled("📄 ", Style::default().fg(Color::Blue)),
                Span::styled(&result.file_path, name_style),
            ]);

            // Line 2: Repository + stars
            let line2 = Line::from(vec![
                Span::styled(
                    format!("  {} ", result.platform),
                    Style::default().fg(platform_color),
                ),
                Span::styled(&result.repository, Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(
                    format!("⭐{}", format_number(result.repository_stars)),
                    Style::default().fg(Color::Rgb(255, 215, 0)),
                ),
            ]);

            // Line 3: Language + match count
            let lang_display = result.language.as_deref().unwrap_or("Unknown");
            let match_count = result.matches.len();
            let line3 = Line::from(vec![
                Span::styled(
                    format!("  {} ", lang_display),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("({} match{})", match_count, if match_count == 1 { "" } else { "es" }),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(vec![line1, line2, line3])
                .style(if is_selected {
                    Style::default().bg(Color::Rgb(40, 40, 60))
                } else {
                    Style::default()
                })
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Code Results ({}) ", app.code_results.len()))
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 60, 80))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

fn render_code_preview(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(result) = app.selected_code_result() {
        // Get all matches and create preview
        let mut preview_lines: Vec<Line> = vec![];

        // Title: file path
        preview_lines.push(Line::from(vec![
            Span::styled("File: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.file_path, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        preview_lines.push(Line::from(""));

        // Repository info
        preview_lines.push(Line::from(vec![
            Span::styled("Repo: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.repository, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(
                format!("⭐{}", format_number(result.repository_stars)),
                Style::default().fg(Color::Rgb(255, 215, 0)),
            ),
        ]));
        preview_lines.push(Line::from(""));

        // Language
        if let Some(lang) = &result.language {
            preview_lines.push(Line::from(vec![
                Span::styled("Language: ", Style::default().fg(Color::DarkGray)),
                Span::styled(lang, Style::default().fg(Color::Green)),
            ]));
            preview_lines.push(Line::from(""));
        }

        preview_lines.push(Line::from(vec![
            Span::styled("─".repeat(50), Style::default().fg(Color::DarkGray)),
        ]));
        preview_lines.push(Line::from(""));

        // Show matches with syntax highlighting
        for (idx, code_match) in result.matches.iter().enumerate() {
            if idx > 0 {
                preview_lines.push(Line::from(""));
                preview_lines.push(Line::from(vec![
                    Span::styled("─".repeat(30), Style::default().fg(Color::DarkGray)),
                ]));
                preview_lines.push(Line::from(""));
            }

            // Match header
            preview_lines.push(Line::from(vec![
                Span::styled(
                    format!("Match {} at line {}", idx + 1, code_match.line_number),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]));
            preview_lines.push(Line::from(""));

            // Syntax-highlighted code
            let highlighted = highlight_code(&code_match.content, result.language.as_deref());
            preview_lines.extend(highlighted);
        }

        // Apply scroll offset
        let start_line = app.code_scroll as usize;
        let visible_lines: Vec<Line> = preview_lines
            .into_iter()
            .skip(start_line)
            .collect();

        let paragraph = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Code Preview ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    } else {
        // No result selected
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No code result selected", Style::default().fg(Color::DarkGray)),
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

/// Syntax highlight code using syntect
fn highlight_code(code: &str, language: Option<&str>) -> Vec<Line<'static>> {
    // Load syntax definitions and themes
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Use a dark theme
    let theme = &ts.themes["base16-ocean.dark"];

    // Detect syntax
    let syntax = if let Some(lang) = language {
        ps.find_syntax_by_name(lang)
            .or_else(|| ps.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ps.find_syntax_plain_text())
    } else {
        ps.find_syntax_plain_text()
    };

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result_lines = Vec::new();

    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(SyntectStyle, &str)> = highlighter
            .highlight_line(line, &ps)
            .unwrap_or_default();

        let mut spans = Vec::new();
        for (style, text) in ranges {
            // Convert syntect style to ratatui style
            let fg_color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(text.to_string(), Style::default().fg(fg_color)));
        }

        result_lines.push(Line::from(spans));
    }

    result_lines
}

/// Render search history popup overlay
fn render_history_popup(frame: &mut Frame, app: &App, area: Rect) {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Calculate responsive popup dimensions based on available space
    // Ensure minimum viable size and proper margins
    let margin_horizontal = 2u16;
    let margin_vertical = 2u16;

    // Calculate available space after margins
    let available_width = area.width.saturating_sub(margin_horizontal * 2);
    let available_height = area.height.saturating_sub(margin_vertical * 2);

    // Determine popup size with adaptive scaling
    let popup_width = if available_width < 50 {
        // Very small terminal - use most of available space
        available_width
    } else if available_width < 80 {
        // Small terminal - use 90% of space
        (available_width * 9) / 10
    } else {
        // Normal terminal - use 60% of space, capped at 100
        ((available_width * 3) / 5).min(100)
    };

    let popup_height = if available_height < 15 {
        // Very small terminal - use most of available space
        available_height
    } else if available_height < 25 {
        // Small terminal - use 80% of space
        (available_height * 4) / 5
    } else {
        // Normal terminal - use 60% of space, capped at 30
        ((available_height * 3) / 5).min(30)
    };

    // Ensure minimum size for usability
    let popup_width = popup_width.max(30); // Minimum 30 chars
    let popup_height = popup_height.max(8); // Minimum 8 lines

    // Center the popup using ratatui Layout
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vertical_chunks[1]);

    let popup_area = horizontal_chunks[1];

    // Clear the popup area to ensure clean rendering
    frame.render_widget(Clear, popup_area);

    // Create history items
    let history_items: Vec<ListItem> = app
        .search_history
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            // Format timestamp as relative time
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let diff = now - entry.searched_at;

            let time_str = if diff < 60 {
                "just now".to_string()
            } else if diff < 3600 {
                let mins = diff / 60;
                format!("{}m ago", mins)
            } else if diff < 86400 {
                let hours = diff / 3600;
                format!("{}h ago", hours)
            } else {
                let days = diff / 86400;
                format!("{}d ago", days)
            };

            // Truncate query if too long to fit in popup
            // Account for borders (2), padding (2), result count (~15), timestamp (~10)
            let reserved_space = 30usize;
            let max_query_len = (popup_area.width as usize).saturating_sub(reserved_space).max(10);

            let query_display = if entry.query.len() > max_query_len {
                // Safely truncate, handling potential UTF-8 boundaries
                let truncate_at = max_query_len.saturating_sub(4).min(entry.query.len());
                format!(" {}... ", &entry.query[..truncate_at])
            } else {
                format!(" {} ", entry.query)
            };

            let mut spans = vec![
                Span::styled(
                    query_display,
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ];

            // Add result count if available
            if let Some(count) = entry.result_count {
                spans.push(Span::styled(
                    format!(" ({} results) ", count),
                    Style::default().fg(Color::Gray),
                ));
            }

            // Add filters if available (only if there's enough width)
            if popup_area.width > 60 {
                if let Some(filters) = &entry.filters {
                    if !filters.is_empty() {
                        let filters_display = if filters.len() > 20 {
                            format!(" [{}...] ", &filters[..17])
                        } else {
                            format!(" [{}] ", filters)
                        };
                        spans.push(Span::styled(
                            filters_display,
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }

            // Add timestamp
            spans.push(Span::styled(
                format!(" {}", time_str),
                Style::default().fg(Color::DarkGray),
            ));

            let line = Line::from(spans);

            // Highlight selected item
            if idx == app.history_selected_index {
                ListItem::new(line).style(Style::default().bg(Color::Blue).fg(Color::White))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    // Add title with terminal size info for debugging
    let title = format!(
        " Search History (Ctrl+R) [{}x{}] ",
        popup_area.width,
        popup_area.height
    );

    let list = List::new(history_items)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(list, popup_area);

    // Render help text at the bottom of the popup if there's enough space
    if popup_area.height > 5 {
        let help_text = " ↑/k: Up | ↓/j: Down | Enter: Select | Esc: Close ";

        // Ensure help text fits within popup width
        let help_text_display = if help_text.len() > popup_area.width as usize {
            " ↑/↓: Navigate | Enter: Select | Esc: Close "
        } else {
            help_text
        };

        let help_area = Rect {
            x: popup_area.x,
            y: popup_area.y.saturating_add(popup_area.height.saturating_sub(1)),
            width: popup_area.width,
            height: 1,
        };

        let help = Paragraph::new(help_text_display)
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
            .block(Block::default().borders(Borders::NONE));

        frame.render_widget(help, help_area);
    }
}

/// Generate GitHub-style contribution heatmap (52 weeks x 7 days)
fn generate_activity_heatmap(repo: &reposcout_core::models::Repository) -> Vec<Line<'_>> {
    use chrono::{Datelike, Duration, Utc};

    let now = Utc::now();
    let days_since_pushed = (now - repo.pushed_at).num_days();
    let days_since_created = (now - repo.created_at).num_days();

    // Get activity score for intensity distribution
    let activity_score = if let Some(health) = &repo.health {
        health.metrics.activity_score
    } else {
        if days_since_pushed < 7 { 25 }
        else if days_since_pushed < 30 { 20 }
        else if days_since_pushed < 90 { 15 }
        else if days_since_pushed < 180 { 10 }
        else { 5 }
    };

    let mut lines = vec![];

    // Month labels (show every ~4 weeks)
    let mut month_line = vec![Span::raw("     ")]; // Padding for day labels
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    // Calculate which month each week belongs to
    for week in (0..52).step_by(4) {
        let date = now - Duration::weeks(52 - week as i64);
        let month_idx = (date.month() - 1) as usize;
        month_line.push(Span::styled(
            format!("{:<4}", months[month_idx]),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(month_line));

    // Generate 7 rows (days of week)
    let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    for day in 0..7 {
        let mut row_spans = vec![];

        // Add day label (show only Mon, Wed, Fri)
        if day == 0 || day == 2 || day == 4 {
            row_spans.push(Span::styled(
                format!("{:<4} ", day_labels[day]),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            row_spans.push(Span::raw("     "));
        }

        // Generate 52 week squares
        for week in 0..52 {
            let days_ago = (52 - week) * 7 + (6 - day);

            // Calculate activity level for this day
            let activity_level = calculate_activity_level(
                days_ago as i64,
                days_since_pushed,
                days_since_created,
                activity_score,
            );

            let color = get_activity_color(activity_level);
            row_spans.push(Span::styled("█", Style::default().fg(color)));
        }

        lines.push(Line::from(row_spans));
    }

    // Legend
    lines.push(Line::from(""));
    let legend_spans = vec![
        Span::raw("     Less "),
        Span::styled("█", Style::default().fg(Color::Rgb(22, 27, 34))),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(Color::Rgb(14, 68, 41))),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(Color::Rgb(0, 109, 50))),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(Color::Rgb(38, 166, 65))),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(Color::Rgb(57, 211, 83))),
        Span::raw(" More"),
    ];
    lines.push(Line::from(legend_spans));

    lines
}

/// Calculate activity level for a specific day based on repository metrics
fn calculate_activity_level(
    days_ago: i64,
    days_since_pushed: i64,
    days_since_created: i64,
    activity_score: u8,
) -> u8 {
    // If repository wasn't created yet, no activity
    if days_ago > days_since_created {
        return 0;
    }

    // Calculate base activity level from score
    // activity_score is 0-30, convert to 0-4 levels
    let base_level = if activity_score >= 25 {
        4
    } else if activity_score >= 20 {
        3
    } else if activity_score >= 15 {
        2
    } else if activity_score >= 10 {
        1
    } else {
        0
    };

    // Apply decay based on how long ago
    // Recent activity (within days_since_pushed) should be brighter
    let decay_factor = if days_ago <= days_since_pushed {
        // Within the active period - use exponential decay from most recent
        let ratio = days_ago as f64 / days_since_pushed.max(1) as f64;
        1.0 - (ratio * 0.7) // Decay up to 70%
    } else {
        // Before last push - much lower activity
        0.2
    };

    // Add some randomization for realistic look
    let pseudo_random = ((days_ago * 17 + days_since_created * 13) % 5) as f64 / 10.0;

    let final_level = (base_level as f64 * decay_factor + pseudo_random).min(4.0).max(0.0);
    final_level.round() as u8
}

/// Get color for activity level (0-4)
fn get_activity_color(level: u8) -> Color {
    match level {
        0 => Color::Rgb(22, 27, 34),      // Very dark (no activity)
        1 => Color::Rgb(14, 68, 41),       // Dark green (low activity)
        2 => Color::Rgb(0, 109, 50),       // Medium green (moderate activity)
        3 => Color::Rgb(38, 166, 65),      // Bright green (good activity)
        4 => Color::Rgb(57, 211, 83),      // Very bright green (high activity)
        _ => Color::Rgb(22, 27, 34),
    }
}

/// Generate activity summary with key metrics
fn generate_activity_summary(repo: &reposcout_core::models::Repository) -> Vec<Line<'_>> {
    use chrono::Utc;

    let now = Utc::now();
    let days_since_created = (now - repo.created_at).num_days();
    let days_since_updated = (now - repo.updated_at).num_days();
    let days_since_pushed = (now - repo.pushed_at).num_days();

    let mut lines = vec![];

    // Show key activity metrics
    lines.push(Line::from(vec![
        Span::styled("Repository Age:    ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_duration_friendly(days_since_created),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Last Updated:      ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_duration_friendly(days_since_updated),
            Style::default().fg(get_freshness_color(days_since_updated)),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Last Pushed:       ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_duration_friendly(days_since_pushed),
            Style::default().fg(get_freshness_color(days_since_pushed)),
        ),
    ]));

    lines.push(Line::from(""));

    // Status indicator
    let (status_icon, status_text, status_color) = if days_since_pushed == 0 {
        ("🔥", "Active today - Very active!", Color::Green)
    } else if days_since_pushed < 7 {
        ("✅", "Active this week - Healthy", Color::Green)
    } else if days_since_pushed < 30 {
        ("✓", "Active this month - Good", Color::Rgb(154, 205, 50))
    } else if days_since_pushed < 90 {
        ("○", "Updated within 3 months - Moderate", Color::Yellow)
    } else if days_since_pushed < 180 {
        ("⚠", "Last updated 3-6 months ago - Stale", Color::Rgb(255, 165, 0))
    } else if days_since_pushed < 365 {
        ("⏸", "Last updated 6-12 months ago - Inactive", Color::Red)
    } else {
        ("💀", "No activity for over a year - Abandoned", Color::Red)
    };

    lines.push(Line::from(vec![
        Span::styled(format!("{} ", status_icon), Style::default()),
        Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
    ]));

    lines
}

fn render_trending_options(frame: &mut Frame, app: &App, area: Rect) {
    use crate::app::TrendingPeriod;

    // Create centered popup
    let popup_width = 60.min(area.width - 4);
    let popup_height = 18.min(area.height - 4);

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the popup area
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 🔥 Trending Options (Enter to search, Tab to switch field, Esc to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let filters = &app.trending_filters;

    // Build options list
    let mut lines = vec![];

    // Period
    let period_style = if app.trending_option_cursor == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Period: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{} ", filters.period.display_name()),
            period_style,
        ),
        Span::styled("(Space to toggle)", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            match filters.period {
                TrendingPeriod::Daily => "→ Last 24 hours",
                TrendingPeriod::Weekly => "→ Last 7 days",
                TrendingPeriod::Monthly => "→ Last 30 days",
            },
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Language
    let lang_style = if app.trending_option_cursor == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Language: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            filters.language.as_deref().unwrap_or("All"),
            lang_style,
        ),
        Span::styled(" (Type to edit, Backspace to clear)", Style::default().fg(Color::DarkGray)),
    ]));

    // Min Stars
    let stars_style = if app.trending_option_cursor == 2 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Min Stars: ", Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}", filters.min_stars), stars_style),
        Span::styled(" (+/- to adjust)", Style::default().fg(Color::DarkGray)),
    ]));

    // Topic
    let topic_style = if app.trending_option_cursor == 3 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Topic: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            filters.topic.as_deref().unwrap_or("None"),
            topic_style,
        ),
        Span::styled(" (Type to edit, Backspace to clear)", Style::default().fg(Color::DarkGray)),
    ]));

    // Sort by velocity
    let velocity_style = if app.trending_option_cursor == 4 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Sort by Velocity: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            if filters.sort_by_velocity { "Yes ⚡" } else { "No" },
            velocity_style,
        ),
        Span::styled(" (Space to toggle)", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  Velocity = stars/day (finds fastest growing repos)",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, inner);
}

// Helper function to format duration in a friendly way
fn format_duration_friendly(days: i64) -> String {
    if days == 0 {
        "Today".to_string()
    } else if days == 1 {
        "1 day ago".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        let weeks = days / 7;
        if weeks == 1 {
            "1 week ago".to_string()
        } else {
            format!("{} weeks ago", weeks)
        }
    } else if days < 365 {
        let months = days / 30;
        if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{} months ago", months)
        }
    } else {
        let years = days / 365;
        if years == 1 {
            "1 year ago".to_string()
        } else {
            format!("{} years ago", years)
        }
    }
}

// Helper to get color based on how fresh/stale the date is
fn get_freshness_color(days: i64) -> Color {
    if days < 7 {
        Color::Green
    } else if days < 30 {
        Color::Rgb(154, 205, 50) // Yellow-green
    } else if days < 90 {
        Color::Yellow
    } else if days < 180 {
        Color::Rgb(255, 165, 0) // Orange
    } else {
        Color::Red
    }
}

/// Render settings popup for token management
fn render_settings_popup(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout, Alignment};

    // Create centered popup (60% width, 50% height)
    let popup_area = centered_rect(60, 50, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create main block
    let block = Block::default()
        .title(" ⚙️  Settings - API Tokens ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block, popup_area);

    // Inner area for content
    let inner_area = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(4),
    };

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Instructions
            Constraint::Min(10),   // Platform list
            Constraint::Length(3), // Status message
            Constraint::Length(1), // Help text
        ])
        .split(inner_area);

    // Instructions
    let instructions = Paragraph::new(
        "Configure API tokens for code search and private repositories.\n\
         Tokens are encrypted and stored locally, valid for 30 days."
    )
    .style(Style::default().fg(Color::Gray))
    .wrap(Wrap { trim: true });
    frame.render_widget(instructions, chunks[0]);

    // Platform options
    let platforms = vec![
        ("GitHub", "github", Color::White),
        ("GitLab", "gitlab", Color::Rgb(252, 109, 38)),
        ("Bitbucket", "bitbucket", Color::Blue),
        ("Close", "", Color::Red),
    ];

    let items: Vec<ListItem> = platforms
        .iter()
        .enumerate()
        .map(|(i, (name, platform, color))| {
            let status = if !platform.is_empty() {
                app.get_token_status(platform)
            } else {
                String::new()
            };

            let style = if i == app.settings_cursor {
                Style::default()
                    .fg(*color)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(*color)
            };

            let content = if !platform.is_empty() {
                format!("  {} - {}", name, status)
            } else {
                format!("  {}", name)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, chunks[1]);

    // Status message
    if let Some(ref msg) = app.token_status_message {
        let status_style = if msg.contains("successfully") {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let status = Paragraph::new(msg.as_str())
            .style(status_style)
            .wrap(Wrap { trim: true });
        frame.render_widget(status, chunks[2]);
    }

    // Help text
    let help = Paragraph::new("↑↓/j/k: Navigate | Enter: Set token | Esc: Close")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

/// Render token input popup
fn render_token_input_popup(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout};

    // Create centered popup (70% width, 40% height)
    let popup_area = centered_rect(70, 40, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create main block
    let title = format!(" Enter {} API Token ", app.token_input_platform.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(block, popup_area);

    // Inner area
    let inner_area = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(4),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Instructions
            Constraint::Length(3), // Input field
            Constraint::Length(1), // Help
        ])
        .split(inner_area);

    // Instructions
    let instructions_text = match app.token_input_platform.as_str() {
        "github" => "Create a token at: https://github.com/settings/tokens\nRequired scopes: 'public_repo' or 'repo' for private repos",
        "gitlab" => "Create a token at: https://gitlab.com/-/profile/personal_access_tokens\nRequired scopes: 'read_api'",
        "bitbucket" => "Create app password at: https://bitbucket.org/account/settings/app-passwords/\nRequired permissions: 'Repositories: Read'",
        _ => "Enter your API token below",
    };

    let instructions = Paragraph::new(instructions_text)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    frame.render_widget(instructions, chunks[0]);

    // Token input (masked)
    let masked_token = if app.token_input_buffer.is_empty() {
        "_".to_string()
    } else {
        "*".repeat(app.token_input_buffer.len())
    };

    let input = Paragraph::new(masked_token)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Token (hidden) ")
                .border_style(Style::default().fg(Color::Yellow))
        );
    frame.render_widget(input, chunks[1]);

    // Help text
    let help = Paragraph::new("Type token | Enter: Save | Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

// Helper function to create centered rect
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

// Render notifications list
fn render_notifications_list(frame: &mut Frame, app: &App, area: Rect) {
    let filter_text = if app.notifications_show_all {
        "All"
    } else {
        "Unread"
    };
    let participating_text = if app.notifications_participating {
        " | Participating"
    } else {
        ""
    };

    let title = format!(
        " Notifications ({}) - {} {} | m: Mark Read | a: Mark All | f: Filter | p: Toggle Participating ",
        app.notifications.len(),
        filter_text,
        participating_text
    );

    let items: Vec<ListItem> = app
        .notifications
        .iter()
        .enumerate()
        .map(|(i, notif)| {
            let unread_marker = if notif.unread { "🔵" } else { "⚪" };
            let icon = match notif.subject.subject_type.as_str() {
                "Issue" => "🐛",
                "PullRequest" => "🔀",
                "Release" => "🎉",
                "Commit" => "📝",
                _ => "📬",
            };

            let line = Line::from(vec![
                Span::raw(format!("{} {} ", unread_marker, icon)),
                Span::styled(
                    notif.subject.title.clone(),
                    Style::default().fg(if notif.unread {
                        Color::White
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(
                    format!(" ({})", notif.repository.full_name),
                    Style::default().fg(Color::Blue),
                ),
            ]);

            let style = if i == app.notifications_selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if app.notifications_loading {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            }),
    );

    frame.render_widget(list, area);
}

// Render notification details/preview
fn render_notification_preview(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(notif) = app.get_selected_notification() {
        let lines = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Cyan)),
                Span::raw(&notif.subject.title),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Repository: ", Style::default().fg(Color::Cyan)),
                Span::raw(&notif.repository.full_name),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Cyan)),
                Span::raw(&notif.subject.subject_type),
            ]),
            Line::from(vec![
                Span::styled("Reason: ", Style::default().fg(Color::Cyan)),
                Span::raw(&notif.reason),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(if notif.unread { "Unread" } else { "Read" }),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::Cyan)),
                Span::raw(notif.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()),
            ]),
        ];

        let mut all_lines = lines;

        if let Some(ref desc) = notif.repository.description {
            all_lines.push(Line::from(""));
            all_lines.push(Line::from(vec![
                Span::styled("Repository Description: ", Style::default().fg(Color::Cyan)),
            ]));
            all_lines.push(Line::from(desc.as_str()));
        }

        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![
            Span::styled("URL: ", Style::default().fg(Color::Cyan)),
            Span::styled(&notif.url, Style::default().fg(Color::Blue)),
        ]));

        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![
            Span::styled("Repository URL: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &notif.repository.html_url,
                Style::default().fg(Color::Blue),
            ),
        ]));

        let paragraph = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Notification Details | Enter: Open in Browser ")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No notification selected")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Notification Details ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            );

        frame.render_widget(paragraph, area);
    }
}
