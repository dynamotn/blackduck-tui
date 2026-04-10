use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::BlackDuckClient;
use crate::app::{App, AppEvent, ComponentFilter, Focus, Screen, StatefulList, VersionTab};

pub async fn handle_events(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<bool> {
    // Non-blocking event poll
    if !event::poll(std::time::Duration::from_millis(100))? {
        return Ok(false);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }

        // Global quit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            app.should_quit = true;
            return Ok(true);
        }

        match &app.screen {
            Screen::Login => handle_login(app, client, tx, key.code),
            Screen::Projects => handle_projects(app, client, tx, key.code),
            Screen::Versions => handle_versions(app, client, tx, key.code),
            Screen::Components | Screen::Vulnerabilities | Screen::PolicyViolations => {
                handle_version_detail(app, client, tx, key.code);
            }
        }
    }

    Ok(false)
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

fn handle_login(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    code: KeyCode,
) {
    if app.search_active {
        return;
    }

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.login_active_field = (app.login_active_field + 1) % 2;
        }
        KeyCode::BackTab => {
            app.login_active_field = usize::from(app.login_active_field == 0);
        }
        KeyCode::Char(c) => {
            if app.login_active_field == 0 {
                app.login_url_input.push(c);
            } else {
                app.login_token_input.push(c);
            }
        }
        KeyCode::Backspace => {
            if app.login_active_field == 0 {
                app.login_url_input.pop();
            } else {
                app.login_token_input.pop();
            }
        }
        KeyCode::Enter => {
            let url = app.login_url_input.trim().to_string();
            let token = app.login_token_input.trim().to_string();

            if url.is_empty() {
                app.login_error = Some("Server URL is required".to_string());
                return;
            }
            if token.is_empty() {
                app.login_error = Some("API Token is required".to_string());
                return;
            }

            app.login_error = None;
            app.loading = true;

            // Rebuild client with new URL
            match BlackDuckClient::new(&url, app.config.server.accept_invalid_certs) {
                Ok(new_client) => {
                    *client = new_client;
                }
                Err(e) => {
                    app.login_error = Some(format!("Failed to create client: {e}"));
                    app.loading = false;
                    return;
                }
            }

            let tx2 = tx.clone();
            let tok = token.clone();
            let mut c2 = client.clone();
            tokio::spawn(async move {
                match c2.authenticate(&tok).await {
                    Ok(()) => {
                        let _ = tx2.send(AppEvent::AuthSuccess).await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });

            // Save config
            app.config.server.url = url;
            app.config.server.api_token = Some(token);
            let _ = app.config.save();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

fn handle_projects(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    code: KeyCode,
) {
    if app.search_active {
        handle_search(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Left {
                let filtered = app.filtered_projects();
                let filtered_sel = filtered
                    .iter()
                    .position(|(i, _)| *i == app.projects.selected);
                if let Some(pos) = filtered_sel {
                    if pos + 1 < filtered.len() {
                        app.projects.selected = filtered[pos + 1].0;
                    }
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Left {
                let filtered = app.filtered_projects();
                let filtered_sel = filtered
                    .iter()
                    .position(|(i, _)| *i == app.projects.selected);
                if let Some(pos) = filtered_sel {
                    if pos > 0 {
                        app.projects.selected = filtered[pos - 1].0;
                    }
                }
            }
        }
        KeyCode::Enter => {
            if let Some(project) = app.projects.selected_item().cloned() {
                if let Some(href) = project.href() {
                    let href = href.to_string();
                    app.selected_project = Some(project);
                    app.loading = true;
                    app.screen = Screen::Versions;
                    app.versions = StatefulList::default();
                    app.versions_total = 0;
                    app.versions_offset = 0;
                    let tx2 = tx.clone();
                    let c2 = client.clone();
                    let page_size = app.config.tui.page_size;
                    tokio::spawn(async move {
                        match c2.get_versions(&href, 0, page_size).await {
                            Ok(resp) => {
                                let _ = tx2
                                    .send(AppEvent::VersionsLoaded {
                                        items: resp.items,
                                        total: resp.total_count,
                                        offset: 0,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                            }
                        }
                    });
                }
            }
        }
        KeyCode::Char('r') => {
            app.projects_offset = 0;
            load_projects(app, client, tx);
        }
        KeyCode::Char('n') => {
            load_next_projects_page(app, client, tx);
        }
        KeyCode::Char('p') => {
            load_prev_projects_page(app, client, tx);
        }
        KeyCode::Tab => {
            app.focus = if app.focus == Focus::Left {
                Focus::Right
            } else {
                Focus::Left
            };
        }
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_input.clear();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

fn handle_versions(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    code: KeyCode,
) {
    if app.search_active {
        handle_search(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Left {
                let filtered = app.filtered_versions();
                let filtered_sel = filtered
                    .iter()
                    .position(|(i, _)| *i == app.versions.selected);
                if let Some(pos) = filtered_sel {
                    if pos + 1 < filtered.len() {
                        app.versions.selected = filtered[pos + 1].0;
                    }
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Left {
                let filtered = app.filtered_versions();
                let filtered_sel = filtered
                    .iter()
                    .position(|(i, _)| *i == app.versions.selected);
                if let Some(pos) = filtered_sel {
                    if pos > 0 {
                        app.versions.selected = filtered[pos - 1].0;
                    }
                }
            }
        }
        KeyCode::Enter => {
            if let Some(version) = app.versions.selected_item().cloned() {
                if let Some(href) = version.href() {
                    let href = href.to_string();
                    app.selected_version = Some(version);
                    app.loading = true;
                    app.screen = Screen::Components;
                    app.version_tab = VersionTab::Components;
                    app.components = StatefulList::default();
                    app.vulnerabilities = StatefulList::default();
                    app.policy_violations = StatefulList::default();

                    let tx2 = tx.clone();
                    let c2 = client.clone();
                    let href2 = href.clone();
                    let page_size = app.config.tui.page_size;
                    tokio::spawn(async move {
                        match c2.get_components(&href2, 0, page_size, &[]).await {
                            Ok(resp) => {
                                let _ = tx2
                                    .send(AppEvent::ComponentsLoaded {
                                        items: resp.items,
                                        total: resp.total_count,
                                        offset: 0,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                            }
                        }
                    });
                }
            }
        }
        KeyCode::Char('n') => {
            load_next_versions_page(app, client, tx);
        }
        KeyCode::Char('p') => {
            load_prev_versions_page(app, client, tx);
        }
        KeyCode::Backspace | KeyCode::Esc => {
            app.go_back();
        }
        KeyCode::Tab => {
            app.focus = if app.focus == Focus::Left {
                Focus::Right
            } else {
                Focus::Left
            };
        }
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_input.clear();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Version detail (components / vulns / policy)
// ---------------------------------------------------------------------------

#[expect(
    clippy::too_many_lines,
    reason = "key-handler with 3 tabs × 2 directions is inherently long"
)]
fn handle_version_detail(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    code: KeyCode,
) {
    if app.search_active {
        handle_search(app, code);
        return;
    }

    // If the filter popup is open, delegate all keys to it
    if app.filter_popup.open {
        handle_filter_popup(app, client, tx, code);
        return;
    }

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('f') => {
            open_filter_popup(app, client, tx);
        }
        KeyCode::Char('n') => {
            load_next_page(app, client, tx);
        }
        KeyCode::Char('p') => {
            load_prev_page(app, client, tx);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Left {
                match app.version_tab {
                    VersionTab::Components => {
                        let filtered = app.filtered_components();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.components.selected);
                        if let Some(p) = pos {
                            if p + 1 < filtered.len() {
                                app.components.selected = filtered[p + 1].0;
                            }
                        }
                    }
                    VersionTab::Vulnerabilities => {
                        let filtered = app.filtered_vulnerabilities();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.vulnerabilities.selected);
                        if let Some(p) = pos {
                            if p + 1 < filtered.len() {
                                app.vulnerabilities.selected = filtered[p + 1].0;
                            }
                        }
                    }
                    VersionTab::PolicyViolations => {
                        let filtered = app.filtered_policy_violations();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.policy_violations.selected);
                        if let Some(p) = pos {
                            if p + 1 < filtered.len() {
                                app.policy_violations.selected = filtered[p + 1].0;
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Left {
                match app.version_tab {
                    VersionTab::Components => {
                        let filtered = app.filtered_components();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.components.selected);
                        if let Some(p) = pos {
                            if p > 0 {
                                app.components.selected = filtered[p - 1].0;
                            }
                        }
                    }
                    VersionTab::Vulnerabilities => {
                        let filtered = app.filtered_vulnerabilities();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.vulnerabilities.selected);
                        if let Some(p) = pos {
                            if p > 0 {
                                app.vulnerabilities.selected = filtered[p - 1].0;
                            }
                        }
                    }
                    VersionTab::PolicyViolations => {
                        let filtered = app.filtered_policy_violations();
                        let pos = filtered
                            .iter()
                            .position(|(i, _)| *i == app.policy_violations.selected);
                        if let Some(p) = pos {
                            if p > 0 {
                                app.policy_violations.selected = filtered[p - 1].0;
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Tab => {
            // Switch focus or switch tabs
            if app.focus == Focus::Left {
                let new_tab = app.version_tab.next();
                switch_version_tab(app, client, tx, new_tab);
            } else {
                app.focus = Focus::Left;
            }
        }
        KeyCode::BackTab => {
            if app.focus == Focus::Left {
                let new_tab = app.version_tab.prev();
                switch_version_tab(app, client, tx, new_tab);
            }
        }
        KeyCode::Left => {
            app.focus = Focus::Left;
        }
        KeyCode::Right => {
            app.focus = Focus::Right;
        }
        KeyCode::Backspace | KeyCode::Esc => {
            app.go_back();
        }
        KeyCode::Char('/') => {
            app.search_active = true;
            app.search_input.clear();
        }
        _ => {}
    }
}

fn switch_version_tab(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    new_tab: VersionTab,
) {
    app.version_tab = new_tab;
    app.search_input.clear();

    // Update screen enum to match
    app.screen = match new_tab {
        VersionTab::Components => Screen::Components,
        VersionTab::Vulnerabilities => Screen::Vulnerabilities,
        VersionTab::PolicyViolations => Screen::PolicyViolations,
    };

    let href = app
        .selected_version
        .as_ref()
        .and_then(|v| v.href())
        .map(ToString::to_string);

    let Some(href) = href else {
        return;
    };

    let page_size = app.config.tui.page_size;

    match new_tab {
        VersionTab::Components if app.components.is_empty() => {
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let filter_params = app.filter.to_api_params();
            tokio::spawn(async move {
                match c2.get_components(&href, 0, page_size, &filter_params).await {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::ComponentsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: 0,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::Vulnerabilities if app.vulnerabilities.is_empty() => {
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            tokio::spawn(async move {
                match c2.get_vulnerabilities(&href, 0, page_size).await {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::VulnerabilitiesLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: 0,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::PolicyViolations if app.policy_violations.is_empty() => {
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let extra_params = pv_extra_filter_params(&app.filter);
            tokio::spawn(async move {
                match c2
                    .get_policy_violations(&href, 0, page_size, &extra_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::PolicyViolationsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: 0,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        _ => {} // already loaded
    }
}

// ---------------------------------------------------------------------------
// Pagination helpers
// ---------------------------------------------------------------------------

/// Build the extra `filter=` params for the Policy Violations API endpoint.
///
/// The `policyStatus:IN_VIOLATION` filter is already hardcoded in `get_policy_violations`;
/// this helper adds the `reviewStatus` and `approvalStatus` params from the active filter.
/// `policy_statuses` and `rule_names` are intentionally excluded here — the former is
/// hardcoded and the latter is not supported server-side.
fn pv_extra_filter_params(filter: &crate::app::ComponentFilter) -> Vec<(&'static str, String)> {
    let mut params: Vec<(&'static str, String)> = Vec::new();
    for s in &filter.review_statuses {
        params.push(("filter", format!("reviewStatus:{s}")));
    }
    for s in &filter.approval_statuses {
        params.push(("filter", format!("approvalStatus:{s}")));
    }
    params
}

fn load_next_page(app: &mut App, client: &mut BlackDuckClient, tx: &mpsc::Sender<AppEvent>) {
    let page_size = app.config.tui.page_size;
    let href = app
        .selected_version
        .as_ref()
        .and_then(|v| v.href())
        .map(ToString::to_string);
    let Some(href) = href else { return };

    match app.version_tab {
        VersionTab::Components => {
            let next_offset = app.components_offset + page_size;
            if u64::from(next_offset) >= app.components_total {
                return; // already on last page
            }
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let filter_params = app.filter.to_api_params();
            tokio::spawn(async move {
                match c2
                    .get_components(&href, next_offset, page_size, &filter_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::ComponentsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: next_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::Vulnerabilities => {
            let next_offset = app.vulnerabilities_offset + page_size;
            if u64::from(next_offset) >= app.vulnerabilities_total {
                return;
            }
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            tokio::spawn(async move {
                match c2.get_vulnerabilities(&href, next_offset, page_size).await {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::VulnerabilitiesLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: next_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::PolicyViolations => {
            let next_offset = app.policy_violations_offset + page_size;
            if u64::from(next_offset) >= app.policy_violations_total {
                return;
            }
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            // For policy violations, only pass review/approval filter params server-side.
            // policy_status is always hardcoded as IN_VIOLATION; rule_names are client-side only.
            let extra_params = pv_extra_filter_params(&app.filter);
            tokio::spawn(async move {
                match c2
                    .get_policy_violations(&href, next_offset, page_size, &extra_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::PolicyViolationsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: next_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
    }
}

fn load_prev_page(app: &mut App, client: &mut BlackDuckClient, tx: &mpsc::Sender<AppEvent>) {
    let page_size = app.config.tui.page_size;
    let href = app
        .selected_version
        .as_ref()
        .and_then(|v| v.href())
        .map(ToString::to_string);
    let Some(href) = href else { return };

    match app.version_tab {
        VersionTab::Components => {
            if app.components_offset == 0 {
                return; // already on first page
            }
            let prev_offset = app.components_offset.saturating_sub(page_size);
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let filter_params = app.filter.to_api_params();
            tokio::spawn(async move {
                match c2
                    .get_components(&href, prev_offset, page_size, &filter_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::ComponentsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: prev_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::Vulnerabilities => {
            if app.vulnerabilities_offset == 0 {
                return;
            }
            let prev_offset = app.vulnerabilities_offset.saturating_sub(page_size);
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            tokio::spawn(async move {
                match c2.get_vulnerabilities(&href, prev_offset, page_size).await {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::VulnerabilitiesLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: prev_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::PolicyViolations => {
            if app.policy_violations_offset == 0 {
                return;
            }
            let prev_offset = app.policy_violations_offset.saturating_sub(page_size);
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let extra_params = pv_extra_filter_params(&app.filter);
            tokio::spawn(async move {
                match c2
                    .get_policy_violations(&href, prev_offset, page_size, &extra_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::PolicyViolationsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: prev_offset,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Filter popup
// ---------------------------------------------------------------------------

/// Open the filter popup. If policy rule names haven't been fetched yet,
/// kick off the API fetch using all hrefs accumulated across every page
/// loaded so far (stored in `app.component_policy_rule_hrefs`).
fn open_filter_popup(app: &mut App, client: &mut BlackDuckClient, tx: &mpsc::Sender<AppEvent>) {
    app.filter_popup.open = true;

    // Fetch policy rule filter options on-demand if not yet loaded
    if app.available_policy_rules.is_empty() {
        let version_href = app
            .selected_version
            .as_ref()
            .and_then(|v| v.href())
            .map(ToString::to_string);

        if let Some(href) = version_href {
            let tx2 = tx.clone();
            let c2 = client.clone();
            tokio::spawn(async move {
                match c2.get_component_filters(&href, "policyRuleViolation").await {
                    Ok(resp) => {
                        // Extract (label, key) pairs where key is already "PR~uuid" format
                        let rules: Vec<(String, String)> = resp
                            .values
                            .into_iter()
                            .map(|opt| (opt.label, opt.key))
                            .collect();
                        let _ = tx2.send(AppEvent::PolicyRulesLoaded(rules)).await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
    }
}

/// Re-fetch page 0 of the current version-detail tab with the currently active filter.
///
/// Called when the filter popup closes (Esc) or when the filter is cleared (`c`), so that
/// the pagination reflects the new server-side filtered total count.
///
/// CRITICAL: Also clears cached data for the other tabs (without fetching) so that when
/// the user switches tabs via `switch_version_tab`, the `is_empty()` check will trigger
/// a fresh re-fetch with the new filter applied. This ensures pagination totals are
/// always correct for the active filter.
fn refetch_with_filter(app: &mut App, client: &mut BlackDuckClient, tx: &mpsc::Sender<AppEvent>) {
    let href = app
        .selected_version
        .as_ref()
        .and_then(|v| v.href())
        .map(ToString::to_string);
    let Some(href) = href else { return };

    let page_size = app.config.tui.page_size;

    // Clear cached data for ALL tabs so switch_version_tab will re-fetch with new filter
    match app.version_tab {
        VersionTab::Components => {
            // Clear sibling tabs (PolicyViolations) — will re-fetch when user switches to them
            app.policy_violations = StatefulList::default();
            app.policy_violations_offset = 0;
            app.policy_violations_total = 0;
            // Re-fetch current tab (Components) with filter
            app.components_offset = 0;
            app.components = StatefulList::default();
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let filter_params = app.filter.to_api_params();
            tokio::spawn(async move {
                match c2.get_components(&href, 0, page_size, &filter_params).await {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::ComponentsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: 0,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
        VersionTab::Vulnerabilities => {
            // Vulnerabilities tab has no filter popup support yet; nothing to do.
        }
        VersionTab::PolicyViolations => {
            // Clear sibling tabs (Components) — will re-fetch when user switches to them
            app.components = StatefulList::default();
            app.components_offset = 0;
            app.components_total = 0;
            // Re-fetch current tab (PolicyViolations) with filter
            app.policy_violations_offset = 0;
            app.policy_violations = StatefulList::default();
            app.loading = true;
            let tx2 = tx.clone();
            let c2 = client.clone();
            let extra_params = pv_extra_filter_params(&app.filter);
            tokio::spawn(async move {
                match c2
                    .get_policy_violations(&href, 0, page_size, &extra_params)
                    .await
                {
                    Ok(resp) => {
                        let _ = tx2
                            .send(AppEvent::PolicyViolationsLoaded {
                                items: resp.items,
                                total: resp.total_count,
                                offset: 0,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                    }
                }
            });
        }
    }
}

/// Handle keypresses while the filter popup is open.
fn handle_filter_popup(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
    code: KeyCode,
) {
    use crate::app::FilterField;

    let current = app.filter_popup.current_field();

    // Resolve option count: static options for most fields; dynamic for PolicyRuleName
    let options_count = match current {
        FilterField::PolicyRuleName => app.available_policy_rules.len(),
        _ => current.options().len(),
    };

    match code {
        KeyCode::Esc => {
            app.filter_popup.open = false;
            app.clamp_selection_to_filter();
            // Re-fetch page 0 with the current filter applied server-side
            refetch_with_filter(app, client, tx);
        }
        KeyCode::Char('c') => {
            app.filter = ComponentFilter::default();
            app.clamp_selection_to_filter();
            // Re-fetch page 0 with cleared filter
            refetch_with_filter(app, client, tx);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.filter_popup.move_field_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.filter_popup.move_field_up();
        }
        KeyCode::Tab | KeyCode::Right => {
            // Move option cursor down within the focused field
            app.filter_popup.move_option_down(options_count);
        }
        KeyCode::BackTab | KeyCode::Left => {
            app.filter_popup.move_option_up(options_count);
        }
        KeyCode::Char(' ') => {
            // Toggle the currently highlighted option
            if options_count == 0 {
                return;
            }
            let idx = app.filter_popup.focused_option;
            let value = match current {
                FilterField::PolicyRuleName => app
                    .available_policy_rules
                    .get(idx)
                    .map(|(_name, id)| id.clone()),
                _ => current.options().get(idx).map(|s| (*s).to_string()),
            };
            if let Some(val) = value {
                let set = match current {
                    FilterField::PolicyStatus => &mut app.filter.policy_statuses,
                    FilterField::ReviewStatus => &mut app.filter.review_statuses,
                    FilterField::ApprovalStatus => &mut app.filter.approval_statuses,
                    FilterField::PolicyRuleName => &mut app.filter.rule_ids,
                };
                ComponentFilter::toggle(set, &val);
                app.clamp_selection_to_filter();
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Enter => {
            app.search_active = false;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        KeyCode::Backspace => {
            if app.search_input.pop().is_none() {
                app.search_active = false;
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Projects pagination helpers
// ---------------------------------------------------------------------------

fn load_next_projects_page(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
) {
    let page_size = app.config.tui.page_size;
    let next_offset = app.projects_offset + page_size;
    if u64::from(next_offset) >= app.projects_total {
        return; // already on last page
    }
    app.projects_offset = next_offset;
    app.projects = StatefulList::default();
    app.loading = true;
    let tx2 = tx.clone();
    let c2 = client.clone();
    tokio::spawn(async move {
        match c2.get_projects(next_offset, page_size).await {
            Ok(resp) => {
                let _ = tx2
                    .send(AppEvent::ProjectsLoaded {
                        items: resp.items,
                        total: resp.total_count,
                        offset: next_offset,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}

fn load_prev_projects_page(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
) {
    if app.projects_offset == 0 {
        return; // already on first page
    }
    let page_size = app.config.tui.page_size;
    let prev_offset = app.projects_offset.saturating_sub(page_size);
    app.projects_offset = prev_offset;
    app.projects = StatefulList::default();
    app.loading = true;
    let tx2 = tx.clone();
    let c2 = client.clone();
    tokio::spawn(async move {
        match c2.get_projects(prev_offset, page_size).await {
            Ok(resp) => {
                let _ = tx2
                    .send(AppEvent::ProjectsLoaded {
                        items: resp.items,
                        total: resp.total_count,
                        offset: prev_offset,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Versions pagination helpers
// ---------------------------------------------------------------------------

fn load_next_versions_page(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
) {
    let page_size = app.config.tui.page_size;
    let next_offset = app.versions_offset + page_size;
    if u64::from(next_offset) >= app.versions_total {
        return;
    }
    let href = app
        .selected_project
        .as_ref()
        .and_then(|p| p.href())
        .map(ToString::to_string);
    let Some(href) = href else { return };
    app.versions_offset = next_offset;
    app.versions = StatefulList::default();
    app.loading = true;
    let tx2 = tx.clone();
    let c2 = client.clone();
    tokio::spawn(async move {
        match c2.get_versions(&href, next_offset, page_size).await {
            Ok(resp) => {
                let _ = tx2
                    .send(AppEvent::VersionsLoaded {
                        items: resp.items,
                        total: resp.total_count,
                        offset: next_offset,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}

fn load_prev_versions_page(
    app: &mut App,
    client: &mut BlackDuckClient,
    tx: &mpsc::Sender<AppEvent>,
) {
    if app.versions_offset == 0 {
        return;
    }
    let page_size = app.config.tui.page_size;
    let prev_offset = app.versions_offset.saturating_sub(page_size);
    let href = app
        .selected_project
        .as_ref()
        .and_then(|p| p.href())
        .map(ToString::to_string);
    let Some(href) = href else { return };
    app.versions_offset = prev_offset;
    app.versions = StatefulList::default();
    app.loading = true;
    let tx2 = tx.clone();
    let c2 = client.clone();
    tokio::spawn(async move {
        match c2.get_versions(&href, prev_offset, page_size).await {
            Ok(resp) => {
                let _ = tx2
                    .send(AppEvent::VersionsLoaded {
                        items: resp.items,
                        total: resp.total_count,
                        offset: prev_offset,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Load projects helper
// ---------------------------------------------------------------------------

pub fn load_projects(app: &mut App, client: &mut BlackDuckClient, tx: &mpsc::Sender<AppEvent>) {
    if !client.is_authenticated() {
        return;
    }
    app.loading = true;
    app.set_status("Loading projects...");
    app.projects = StatefulList::default();

    let tx2 = tx.clone();
    let c2 = client.clone();
    let page_size = app.config.tui.page_size;
    let offset = app.projects_offset;
    tokio::spawn(async move {
        match c2.get_projects(offset, page_size).await {
            Ok(resp) => {
                let _ = tx2
                    .send(AppEvent::ProjectsLoaded {
                        items: resp.items,
                        total: resp.total_count,
                        offset,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}
