use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::BlackDuckClient;
use crate::app::{App, AppEvent, Focus, Screen, StatefulList, VersionTab};

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
                    let tx2 = tx.clone();
                    let c2 = client.clone();
                    let page_size = app.config.tui.page_size;
                    tokio::spawn(async move {
                        match c2.get_versions(&href, 0, page_size).await {
                            Ok(resp) => {
                                let _ = tx2.send(AppEvent::VersionsLoaded(resp.items)).await;
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
            load_projects(app, client, tx);
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
                        match c2.get_components(&href2, 0, page_size).await {
                            Ok(resp) => {
                                let _ = tx2.send(AppEvent::ComponentsLoaded(resp.items)).await;
                            }
                            Err(e) => {
                                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                            }
                        }
                    });
                }
            }
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

    match code {
        KeyCode::Char('q') => {
            app.should_quit = true;
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
            tokio::spawn(async move {
                match c2.get_components(&href, 0, page_size).await {
                    Ok(resp) => {
                        let _ = tx2.send(AppEvent::ComponentsLoaded(resp.items)).await;
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
                        let _ = tx2.send(AppEvent::VulnerabilitiesLoaded(resp.items)).await;
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
            tokio::spawn(async move {
                match c2.get_policy_violations(&href, 0, page_size).await {
                    Ok(resp) => {
                        let _ = tx2.send(AppEvent::PolicyViolationsLoaded(resp.items)).await;
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
    tokio::spawn(async move {
        match c2.get_projects(0, page_size).await {
            Ok(resp) => {
                let _ = tx2.send(AppEvent::ProjectsLoaded(resp.items)).await;
            }
            Err(e) => {
                let _ = tx2.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });
}
