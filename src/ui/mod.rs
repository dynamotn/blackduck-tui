use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::app::{App, FilterField, Focus, Screen, VersionTab};

pub mod colors {
    use ratatui::style::Color;
    pub const PRIMARY: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::Blue;
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const DANGER: Color = Color::Red;
    pub const MUTED: Color = Color::DarkGray;
    pub const SELECTED_BG: Color = Color::DarkGray;
    pub const CRITICAL: Color = Color::Red;
    pub const HIGH: Color = Color::LightRed;
    pub const MEDIUM: Color = Color::Yellow;
    pub const LOW: Color = Color::Green;
}

/// Main render entry point
pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    match &app.screen {
        Screen::Login => render_login(f, app, size),
        Screen::Projects => render_projects(f, app, size),
        Screen::Versions => render_versions(f, app, size),
        Screen::Components | Screen::Vulnerabilities | Screen::PolicyViolations => {
            render_version_detail(f, app, size);
        }
    }
}

// ---------------------------------------------------------------------------
// Login screen
// ---------------------------------------------------------------------------

fn render_login(f: &mut Frame, app: &App, area: Rect) {
    // Background
    let bg = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(bg, area);

    // Center box
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(14),
            Constraint::Min(0),
        ])
        .split(area);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vchunks[1]);

    let box_area = hchunks[1];

    f.render_widget(Clear, box_area);

    let title = Line::from(vec![
        Span::styled(
            " Black Duck ",
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("TUI ", Style::default().fg(Color::White)),
    ]);

    let outer = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::PRIMARY));
    f.render_widget(outer, box_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(box_area);

    // URL field
    let url_style = if app.login_active_field == 0 {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };
    let url_block = Block::default()
        .title(" Server URL ")
        .borders(Borders::ALL)
        .border_style(url_style);
    let url_para = Paragraph::new(app.login_url_input.as_str())
        .block(url_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(url_para, inner[0]);

    // Token field
    let tok_style = if app.login_active_field == 1 {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };
    let tok_block = Block::default()
        .title(" API Token ")
        .borders(Borders::ALL)
        .border_style(tok_style);
    // Mask token
    let masked: String = "*".repeat(app.login_token_input.len().min(40));
    let tok_para = Paragraph::new(masked.as_str())
        .block(tok_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(tok_para, inner[2]);

    // Hint
    let hint = Paragraph::new("Tab: switch field  Enter: connect  q: quit")
        .style(Style::default().fg(colors::MUTED));
    f.render_widget(hint, inner[4]);

    // Error
    if let Some(err) = &app.login_error {
        let err_para = Paragraph::new(err.as_str())
            .style(
                Style::default()
                    .fg(colors::DANGER)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(err_para, inner[5]);
    }

    if app.loading {
        let loading = Paragraph::new("Connecting...").style(
            Style::default()
                .fg(colors::WARNING)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(loading, inner[4]);
    }
}

// ---------------------------------------------------------------------------
// Projects screen (left: projects, right: help/info)
// ---------------------------------------------------------------------------

fn render_projects(f: &mut Frame, app: &App, area: Rect) {
    let chunks = main_layout(area);
    render_project_list(f, app, chunks[0]);
    render_project_detail(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);
}

fn render_project_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let page_size = app.config.tui.page_size;
    let shown_from = app.projects_offset + 1;
    let shown_to =
        app.projects_offset + u32::try_from(app.projects.items.len()).unwrap_or(u32::MAX);
    let pagination = if app.projects_total > u64::from(page_size) {
        format!("{shown_from}\u{2013}{shown_to}/{}", app.projects_total)
    } else {
        format!("{}", app.projects.items.len())
    };
    let title = format!(" Projects ({pagination}) ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let filtered = app.filtered_projects();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, p)| {
            let desc = p
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect::<String>();
            let line = if desc.is_empty() {
                Line::from(Span::raw(p.name.clone()))
            } else {
                Line::from(vec![
                    Span::raw(p.name.clone()),
                    Span::styled(format!(" - {desc}"), Style::default().fg(colors::MUTED)),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    // Map the real selected index to filtered list index
    let filtered_sel = filtered
        .iter()
        .position(|(i, _)| *i == app.projects.selected);
    state.select(filtered_sel);

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTED_BG)
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut state);

    render_search_bar(f, app, area);
}

fn render_project_detail(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Project Info ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(p) = app.projects.selected_item() {
        let mut lines = vec![Line::from(vec![
            Span::styled(
                "Name: ",
                Style::default()
                    .fg(colors::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(p.name.clone()),
        ])];
        if let Some(desc) = &p.description {
            lines.push(Line::from(vec![
                Span::styled(
                    "Description: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(desc.clone()),
            ]));
        }
        if let Some(href) = p.href() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("URL: ", Style::default().fg(colors::MUTED)),
                Span::styled(href, Style::default().fg(colors::MUTED)),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Press Enter to view versions",
            Style::default().fg(colors::PRIMARY),
        ));
        lines
    } else if app.loading {
        vec![Line::styled(
            "Loading projects...",
            Style::default().fg(colors::WARNING),
        )]
    } else {
        vec![
            Line::styled("No project selected", Style::default().fg(colors::MUTED)),
            Line::raw(""),
            Line::styled(
                "Keybindings:",
                Style::default()
                    .fg(colors::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(vec![
                Span::styled("  j/↓  ", Style::default().fg(colors::PRIMARY)),
                Span::raw("move down"),
            ]),
            Line::from(vec![
                Span::styled("  k/↑  ", Style::default().fg(colors::PRIMARY)),
                Span::raw("move up"),
            ]),
            Line::from(vec![
                Span::styled("  Enter", Style::default().fg(colors::PRIMARY)),
                Span::raw("  open versions"),
            ]),
            Line::from(vec![
                Span::styled("  /    ", Style::default().fg(colors::PRIMARY)),
                Span::raw("  search"),
            ]),
            Line::from(vec![
                Span::styled("  r    ", Style::default().fg(colors::PRIMARY)),
                Span::raw("  refresh"),
            ]),
            Line::from(vec![
                Span::styled("  q    ", Style::default().fg(colors::PRIMARY)),
                Span::raw("  quit"),
            ]),
        ]
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

// ---------------------------------------------------------------------------
// Versions screen
// ---------------------------------------------------------------------------

fn render_versions(f: &mut Frame, app: &App, area: Rect) {
    let chunks = main_layout(area);
    render_version_list(f, app, chunks[0]);
    render_version_info(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);
}

fn render_version_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let proj_name = app
        .selected_project
        .as_ref()
        .map_or("Project", |p| p.name.as_str());

    let page_size = app.config.tui.page_size;
    let shown_from = app.versions_offset + 1;
    let shown_to =
        app.versions_offset + u32::try_from(app.versions.items.len()).unwrap_or(u32::MAX);
    let pagination = if app.versions_total > u64::from(page_size) {
        format!("{shown_from}\u{2013}{shown_to}/{}", app.versions_total)
    } else {
        format!("{}", app.versions.items.len())
    };
    let title = format!(" {proj_name} - Versions ({pagination}) ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let filtered = app.filtered_versions();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, v)| {
            let phase = v.phase.as_deref().unwrap_or("-");
            let line = Line::from(vec![
                Span::raw(v.version_name.clone()),
                Span::styled(format!(" [{phase}]"), Style::default().fg(colors::MUTED)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    let filtered_sel = filtered
        .iter()
        .position(|(i, _)| *i == app.versions.selected);
    state.select(filtered_sel);

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTED_BG)
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut state);
    render_search_bar(f, app, area);
}

fn render_version_info(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Version Info ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(v) = app.versions.selected_item() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "Version: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(v.version_name.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Phase:   ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(v.phase.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Distribution: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(v.distribution.as_deref().unwrap_or("-")),
            ]),
        ];
        if let Some(rel) = &v.released_on {
            lines.push(Line::from(vec![
                Span::styled(
                    "Released: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(rel.clone()),
            ]));
        }
        if let Some(created) = &v.created_at {
            lines.push(Line::from(vec![
                Span::styled(
                    "Created:  ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(created.clone()),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Press Enter to view components/vulnerabilities",
            Style::default().fg(colors::PRIMARY),
        ));
        lines.push(Line::styled(
            "Press Backspace to go back",
            Style::default().fg(colors::MUTED),
        ));
        lines
    } else if app.loading {
        vec![Line::styled(
            "Loading versions...",
            Style::default().fg(colors::WARNING),
        )]
    } else {
        vec![Line::styled(
            "No version selected",
            Style::default().fg(colors::MUTED),
        )]
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

// ---------------------------------------------------------------------------
// Version detail (Components / Vulnerabilities / Policy Violations)
// ---------------------------------------------------------------------------

fn render_version_detail(f: &mut Frame, app: &App, area: Rect) {
    let chunks = main_layout(area);

    // Left panel: list
    let tab = app.version_tab;
    match tab {
        VersionTab::Components => render_components_list(f, app, chunks[0]),
        VersionTab::Vulnerabilities => render_vulnerabilities_list(f, app, chunks[0]),
        VersionTab::PolicyViolations => render_policy_list(f, app, chunks[0]),
    }

    // Right panel: detail
    match tab {
        VersionTab::Components => render_component_detail(f, app, chunks[1]),
        VersionTab::Vulnerabilities => render_vulnerability_detail(f, app, chunks[1]),
        VersionTab::PolicyViolations => render_policy_detail(f, app, chunks[1]),
    }

    render_status_bar(f, app, chunks[2]);

    // Filter popup overlay (rendered last so it appears on top)
    if app.filter_popup.open {
        render_filter_popup(f, app, area);
    }
}

fn render_tab_header<'a>(app: &App) -> Tabs<'a> {
    let tab_titles = vec![
        Line::from(VersionTab::Components.title()),
        Line::from(VersionTab::Vulnerabilities.title()),
        Line::from(VersionTab::PolicyViolations.title()),
    ];
    let selected = match app.version_tab {
        VersionTab::Components => 0,
        VersionTab::Vulnerabilities => 1,
        VersionTab::PolicyViolations => 2,
    };
    Tabs::new(tab_titles)
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|")
}

fn render_components_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let ver_name = app
        .selected_version
        .as_ref()
        .map_or("Version", |v| v.version_name.as_str());

    let filter_badge = if app.filter.active_count() > 0 {
        format!(" [{} filters]", app.filter.active_count())
    } else {
        String::new()
    };
    let page_size = app.config.tui.page_size;
    let shown_from = app.components_offset + 1;
    let shown_to =
        app.components_offset + u32::try_from(app.components.items.len()).unwrap_or(u32::MAX);
    let pagination = if app.components_total > u64::from(page_size) {
        format!("{shown_from}\u{2013}{shown_to}/{}", app.components_total)
    } else {
        format!("{}", app.components.items.len())
    };
    let title = format!(
        " {ver_name} - Components ({}) {filter_badge}",
        pagination.trim(),
    );

    // Split for tabs + list
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tabs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title.clone());
    let tabs = render_tab_header(app).block(tabs_block);
    f.render_widget(tabs, inner[0]);

    let filtered = app.filtered_components();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, c)| {
            let ver = c.component_version_name.as_deref().unwrap_or("?");
            let review = c.review_status.as_deref().unwrap_or("UNREVIEWED");
            let review_style = match review {
                "REVIEWED" => Style::default().fg(colors::SUCCESS),
                "DYNAMIC" => Style::default().fg(colors::WARNING),
                _ => Style::default().fg(colors::MUTED),
            };
            let line = Line::from(vec![
                Span::raw(c.component_name.clone()),
                Span::styled(format!("@{ver}"), Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(format!("[{review}]"), review_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    let filtered_sel = filtered
        .iter()
        .position(|(i, _)| *i == app.components.selected);
    state.select(filtered_sel);

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(border_style);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTED_BG)
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, inner[1], &mut state);
    render_search_bar(f, app, inner[1]);
}

fn render_vulnerabilities_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let ver_name = app
        .selected_version
        .as_ref()
        .map_or("Version", |v| v.version_name.as_str());
    let page_size = app.config.tui.page_size;
    let shown_from = app.vulnerabilities_offset + 1;
    let shown_to = app.vulnerabilities_offset
        + u32::try_from(app.vulnerabilities.items.len()).unwrap_or(u32::MAX);
    let pagination = if app.vulnerabilities_total > u64::from(page_size) {
        format!(
            "{shown_from}\u{2013}{shown_to}/{}",
            app.vulnerabilities_total
        )
    } else {
        format!("{}", app.vulnerabilities.items.len())
    };
    let title = format!(" {ver_name} - Vulnerabilities ({pagination}) ");

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tabs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let tabs = render_tab_header(app).block(tabs_block);
    f.render_widget(tabs, inner[0]);

    let filtered = app.filtered_vulnerabilities();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, v)| {
            if let Some(detail) = &v.vulnerability_with_remediation {
                let severity = detail.severity.as_deref().unwrap_or("?");
                let sev_style = severity_style(severity);
                let score = detail
                    .cvss3_score
                    .map(|s| format!("{s:.1}"))
                    .or_else(|| detail.cvss2_score.map(|s| format!("{s:.1}")))
                    .unwrap_or_else(|| "?".to_string());
                let comp = v.component_name.as_deref().unwrap_or("?");
                let line = Line::from(vec![
                    Span::styled(format!("[{severity:8}]"), sev_style),
                    Span::raw(" "),
                    Span::styled(
                        detail.vulnerability_name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" ({score})"), Style::default().fg(colors::MUTED)),
                    Span::styled(format!(" - {comp}"), Style::default().fg(Color::Gray)),
                ]);
                ListItem::new(line)
            } else {
                ListItem::new(Line::raw("Unknown vulnerability"))
            }
        })
        .collect();

    let mut state = ListState::default();
    let filtered_sel = filtered
        .iter()
        .position(|(i, _)| *i == app.vulnerabilities.selected);
    state.select(filtered_sel);

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(border_style);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTED_BG)
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, inner[1], &mut state);
    render_search_bar(f, app, inner[1]);
}

fn render_policy_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let ver_name = app
        .selected_version
        .as_ref()
        .map_or("Version", |v| v.version_name.as_str());

    // Policy violations only uses review/approval filter badge (not policy_status or rule_names)
    let pv_filter_count = usize::from(!app.filter.review_statuses.is_empty())
        + usize::from(!app.filter.approval_statuses.is_empty());
    let filter_badge = if pv_filter_count > 0 {
        format!(" [{pv_filter_count} filters]")
    } else {
        String::new()
    };
    let page_size = app.config.tui.page_size;
    let shown_from = app.policy_violations_offset + 1;
    let shown_to = app.policy_violations_offset
        + u32::try_from(app.policy_violations.items.len()).unwrap_or(u32::MAX);
    let pagination = if app.policy_violations_total > u64::from(page_size) {
        format!(
            "{shown_from}\u{2013}{shown_to}/{}",
            app.policy_violations_total
        )
    } else {
        format!("{}", app.policy_violations.items.len())
    };
    let title = format!(" {ver_name} - Policy Violations ({pagination}){filter_badge} ");

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tabs_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let tabs = render_tab_header(app).block(tabs_block);
    f.render_widget(tabs, inner[0]);

    let filtered = app.filtered_policy_violations();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, c)| {
            let ver = c.component_version_name.as_deref().unwrap_or("?");
            let status = c.policy_status.as_deref().unwrap_or("IN_VIOLATION");
            let status_style = Style::default().fg(colors::DANGER);
            let line = Line::from(vec![
                Span::styled(format!("[{status:12}]"), status_style),
                Span::raw(" "),
                Span::raw(c.component_name.clone()),
                Span::styled(format!("@{ver}"), Style::default().fg(Color::Gray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    let filtered_sel = filtered
        .iter()
        .position(|(i, _)| *i == app.policy_violations.selected);
    state.select(filtered_sel);

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(border_style);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(colors::SELECTED_BG)
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, inner[1], &mut state);
    render_search_bar(f, app, inner[1]);
}

// ---------------------------------------------------------------------------
// Detail panels (right side)
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_lines,
    reason = "detail panel renders many optional fields"
)]
fn render_component_detail(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Component Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(c) = app.components.selected_item() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "Name:    ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.component_name.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Version: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.component_version_name.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Review:  ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.review_status.as_deref().unwrap_or("-")),
            ]),
        ];

        // Licenses
        if let Some(lic_list) = &c.licenses {
            if !lic_list.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "Licenses:",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ));
                for lic in lic_list {
                    if let Some(name) = &lic.license_name {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(name.clone(), Style::default().fg(Color::White)),
                        ]));
                    } else if let Some(sub) = &lic.licenses {
                        for sl in sub {
                            if let Some(n) = &sl.name {
                                lines.push(Line::from(vec![
                                    Span::raw("  • "),
                                    Span::styled(n.clone(), Style::default().fg(Color::White)),
                                ]));
                            }
                        }
                    }
                }
            }
        }

        // Risk
        let sec_risk = risk_summary(c.security_risk_profile.as_ref());
        let lic_risk = risk_summary(c.license_risk_profile.as_ref());
        let op_risk = risk_summary(c.operational_risk_profile.as_ref());

        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Risk Profile:",
            Style::default()
                .fg(colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(vec![
            Span::styled("  Security:    ", Style::default().fg(Color::Gray)),
            Span::raw(sec_risk),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  License:     ", Style::default().fg(Color::Gray)),
            Span::raw(lic_risk),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Operational: ", Style::default().fg(Color::Gray)),
            Span::raw(op_risk),
        ]));

        lines
    } else {
        vec![Line::styled(
            "Select a component",
            Style::default().fg(colors::MUTED),
        )]
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

#[expect(
    clippy::too_many_lines,
    reason = "detail panel renders many optional vulnerability fields"
)]
fn render_vulnerability_detail(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Vulnerability Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(v) = app.vulnerabilities.selected_item() {
        if let Some(detail) = &v.vulnerability_with_remediation {
            let severity = detail.severity.as_deref().unwrap_or("?");
            let sev_style = severity_style(severity);
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(
                        "CVE:       ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        detail.vulnerability_name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Severity:  ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(severity, sev_style.add_modifier(Modifier::BOLD)),
                ]),
            ];

            if let Some(score) = detail.cvss3_score {
                lines.push(Line::from(vec![
                    Span::styled(
                        "CVSS3:     ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("{score:.1}")),
                ]));
            }
            if let Some(score) = detail.cvss2_score {
                lines.push(Line::from(vec![
                    Span::styled(
                        "CVSS2:     ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("{score:.1}")),
                ]));
            }

            if let Some(comp) = &v.component_name {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Component: ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(comp.clone()),
                ]));
            }
            if let Some(ver) = &v.component_version_name {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Version:   ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(ver.clone()),
                ]));
            }

            if let Some(status) = &detail.remediation_status {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Remediation: ",
                        Style::default()
                            .fg(colors::SECONDARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(status.clone()),
                ]));
            }

            if let Some(published) = &detail.published_date {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled("Published: ", Style::default().fg(Color::Gray)),
                    Span::raw(published.clone()),
                ]));
            }

            if let Some(desc) = &detail.description {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "Description:",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::raw(desc.clone()));
            }

            if let Some(comment) = &detail.remediation_comment {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "Comment:",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::raw(comment.clone()));
            }

            lines
        } else {
            vec![Line::styled(
                "No detail available",
                Style::default().fg(colors::MUTED),
            )]
        }
    } else {
        vec![Line::styled(
            "Select a vulnerability",
            Style::default().fg(colors::MUTED),
        )]
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

#[expect(
    clippy::too_many_lines,
    reason = "detail panel renders many optional policy violation fields"
)]
fn render_policy_detail(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        Style::default().fg(colors::PRIMARY)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(" Policy Violation Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let content = if let Some(c) = app.policy_violations.selected_item() {
        let status = c.policy_status.as_deref().unwrap_or("IN_VIOLATION");
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "Component: ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.component_name.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Version:   ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.component_version_name.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Status:    ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    status,
                    Style::default()
                        .fg(colors::DANGER)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Review:    ",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.review_status.as_deref().unwrap_or("-")),
            ]),
        ];

        // Licenses
        if let Some(lic_list) = &c.licenses {
            if !lic_list.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "Licenses:",
                    Style::default()
                        .fg(colors::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ));
                for lic in lic_list {
                    if let Some(name) = &lic.license_name {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(name.clone(), Style::default().fg(Color::White)),
                        ]));
                    } else if let Some(sub) = &lic.licenses {
                        for sl in sub {
                            if let Some(n) = &sl.name {
                                lines.push(Line::from(vec![
                                    Span::raw("  • "),
                                    Span::styled(n.clone(), Style::default().fg(Color::White)),
                                ]));
                            }
                        }
                    }
                }
            }
        }

        // Risk
        let sec_risk = risk_summary(c.security_risk_profile.as_ref());
        let lic_risk = risk_summary(c.license_risk_profile.as_ref());
        let op_risk = risk_summary(c.operational_risk_profile.as_ref());

        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Risk Profile:",
            Style::default()
                .fg(colors::SECONDARY)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(vec![
            Span::styled("  Security:    ", Style::default().fg(Color::Gray)),
            Span::raw(sec_risk),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  License:     ", Style::default().fg(Color::Gray)),
            Span::raw(lic_risk),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Operational: ", Style::default().fg(Color::Gray)),
            Span::raw(op_risk),
        ]));

        lines
    } else {
        vec![Line::styled(
            "Select a policy violation",
            Style::default().fg(colors::MUTED),
        )]
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let breadcrumb = build_breadcrumb(app);

    let right_hint = match app.screen {
        Screen::Login => "",
        Screen::Projects => {
            "j/k:move  Enter:open  n/p:page  /:search  r:refresh  Tab:switch  q:quit"
        }
        Screen::Versions => "j/k:move  Enter:open  n/p:page  Backspace:back  /:search  q:quit",
        Screen::Components | Screen::Vulnerabilities | Screen::PolicyViolations => {
            "j/k:move  Tab:tab  f:filter  n/p:page  Backspace:back  /:search  q:quit"
        }
    };

    let (msg_text, msg_style) = if let Some(err) = &app.error_message {
        (err.as_str(), Style::default().fg(colors::DANGER))
    } else if let Some(status) = &app.status_message {
        (status.as_str(), Style::default().fg(colors::SUCCESS))
    } else if app.loading {
        ("Loading...", Style::default().fg(colors::WARNING))
    } else {
        ("", Style::default())
    };

    let bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Breadcrumb
    let bc_para = Paragraph::new(breadcrumb)
        .style(Style::default().fg(colors::SECONDARY))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(bc_para, bar_chunks[0]);

    // Status message
    let msg_para = Paragraph::new(msg_text)
        .style(msg_style)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(msg_para, bar_chunks[1]);

    // Hint
    let hint_para = Paragraph::new(right_hint)
        .style(Style::default().fg(colors::MUTED))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(hint_para, bar_chunks[2]);
}

fn build_breadcrumb(app: &App) -> String {
    let mut parts = vec!["BlackDuck TUI".to_string()];
    if let Some(p) = &app.selected_project {
        parts.push(p.name.clone());
    }
    if let Some(v) = &app.selected_version {
        parts.push(v.version_name.clone());
    }
    match app.screen {
        Screen::Components | Screen::Vulnerabilities | Screen::PolicyViolations => {
            parts.push(app.version_tab.title().to_string());
        }
        _ => {}
    }
    parts.join(" > ")
}

// ---------------------------------------------------------------------------
// Filter popup overlay
// ---------------------------------------------------------------------------

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "filter popup renders 4 sections with checkboxes and dynamic content"
)]
fn render_filter_popup(f: &mut Frame, app: &App, area: Rect) {
    // Collect all option rows across all 4 sections to compute total height
    let policy_opts = FilterField::PolicyStatus.options();
    let review_opts = FilterField::ReviewStatus.options();
    let approval_opts = FilterField::ApprovalStatus.options();
    let rule_opts: Vec<&str> = app
        .available_policy_rules
        .iter()
        .map(|(name, _id)| name.as_str())
        .collect();

    // For PolicyRuleName, create a list of selected display names based on selected IDs
    let selected_rule_names: Vec<String> = app
        .filter
        .rule_ids
        .iter()
        .filter_map(|id| {
            app.available_policy_rules
                .iter()
                .find(|(_, rule_id)| rule_id == id)
                .map(|(name, _)| name.clone())
        })
        .collect();

    // Height: title border (2) + 4 section headers + their options + spacing between sections
    let to_u16 = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
    let content_rows: u16 = 1 // title padding
        + 1 + to_u16(policy_opts.len())     // Policy Status header + opts
        + 1                                  // blank separator
        + 1 + to_u16(review_opts.len())     // Review Status
        + 1
        + 1 + to_u16(approval_opts.len())   // Approval Status
        + 1
        + 1 + (if rule_opts.is_empty() { 1 } else { to_u16(rule_opts.len()) }) // Policy Rule Name
        + 1; // footer spacing

    let popup_height = content_rows + 2; // +2 for border
    let popup_width: u16 = 54;

    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Filters  j/k:field  Tab:option  Space:toggle  c:clear  Esc:close ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::PRIMARY));
    f.render_widget(block, popup_area);

    // Inner area for content
    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    let current_field = app.filter_popup.current_field();

    let mut lines: Vec<Line> = Vec::new();

    // Helper closure: render one section
    let render_section = |lines: &mut Vec<Line>,
                          field: FilterField,
                          opts: &[&str],
                          selected_set: &[String],
                          focused_option: usize,
                          is_focused_field: bool| {
        let label_style = if is_focused_field {
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(colors::SECONDARY)
                .add_modifier(Modifier::BOLD)
        };
        lines.push(Line::styled(format!("  {}:", field.label()), label_style));
        if opts.is_empty() {
            lines.push(Line::styled(
                "    (loading...)",
                Style::default().fg(colors::MUTED),
            ));
        } else {
            for (i, opt) in opts.iter().enumerate() {
                let checked = selected_set.iter().any(|s| s == opt);
                let checkbox = if checked { "[✓]" } else { "[ ]" };
                let is_highlighted = is_focused_field && i == focused_option;
                let opt_style = if is_highlighted {
                    Style::default()
                        .bg(colors::SELECTED_BG)
                        .fg(colors::PRIMARY)
                        .add_modifier(Modifier::BOLD)
                } else if checked {
                    Style::default().fg(colors::SUCCESS)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::styled(format!("    {checkbox} {opt}"), opt_style));
            }
        }
    };

    render_section(
        &mut lines,
        FilterField::PolicyStatus,
        policy_opts,
        &app.filter.policy_statuses,
        app.filter_popup.focused_option,
        current_field == FilterField::PolicyStatus,
    );
    lines.push(Line::raw(""));

    render_section(
        &mut lines,
        FilterField::ReviewStatus,
        review_opts,
        &app.filter.review_statuses,
        app.filter_popup.focused_option,
        current_field == FilterField::ReviewStatus,
    );
    lines.push(Line::raw(""));

    render_section(
        &mut lines,
        FilterField::ApprovalStatus,
        approval_opts,
        &app.filter.approval_statuses,
        app.filter_popup.focused_option,
        current_field == FilterField::ApprovalStatus,
    );
    lines.push(Line::raw(""));

    // PolicyRuleName — dynamic options
    render_section(
        &mut lines,
        FilterField::PolicyRuleName,
        &rule_opts,
        &selected_rule_names,
        app.filter_popup.focused_option,
        current_field == FilterField::PolicyRuleName,
    );

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

// ---------------------------------------------------------------------------
// Search bar overlay
// ---------------------------------------------------------------------------

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    if !app.search_active && app.search_input.is_empty() {
        return;
    }

    let search_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(3),
        width: area.width.saturating_sub(2).min(40),
        height: 3,
    };

    f.render_widget(Clear, search_area);
    let block = Block::default()
        .title(" Search ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::WARNING));
    let para = Paragraph::new(app.search_input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(para, search_area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn main_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(area)
        .iter()
        .flat_map(|&r| {
            if r.height == 2 {
                vec![r]
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .split(r)
                    .to_vec()
            }
        })
        .collect()
}

fn severity_style(severity: &str) -> Style {
    match severity.to_uppercase().as_str() {
        "CRITICAL" => Style::default().fg(colors::CRITICAL),
        "HIGH" => Style::default().fg(colors::HIGH),
        "MEDIUM" => Style::default().fg(colors::MEDIUM),
        "LOW" => Style::default().fg(colors::LOW),
        _ => Style::default().fg(colors::MUTED),
    }
}

fn risk_summary(risk: Option<&crate::api::RiskCount>) -> String {
    if let Some(r) = risk {
        if let Some(counts) = &r.counts {
            let parts: Vec<String> = counts
                .iter()
                .filter(|e| e.count > 0)
                .map(|e| format!("{}:{}", e.count_type.chars().next().unwrap_or('?'), e.count))
                .collect();
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }
    }
    "none".to_string()
}
