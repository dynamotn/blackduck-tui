mod api;
mod app;
mod config;
mod events;
mod ui;

use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

use app::{App, AppEvent, Screen, StatefulList};
use config::Config;
use events::{handle_events, load_projects};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup file logger (TUI takes over stdout)
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("blackduck-tui");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "blackduck-tui.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .with_writer(non_blocking)
        .init();

    // Load config
    let config = Config::load().unwrap_or_default();

    // Build API client
    let mut client =
        api::BlackDuckClient::new(&config.server.url, config.server.accept_invalid_certs)
            .unwrap_or_else(|_| api::BlackDuckClient::new("https://localhost", true).unwrap());

    // Event channel
    let (tx, mut rx) = mpsc::channel::<AppEvent>(64);

    // Initialize app
    let mut app = App::new(config);

    // If we have credentials, authenticate immediately
    if app.screen == Screen::Projects {
        let token = app.config.server.api_token.clone().unwrap_or_default();
        app.loading = true;
        app.set_status("Authenticating...");

        let tx2 = tx.clone();
        let mut c2 = client.clone();
        tokio::spawn(async move {
            match c2.authenticate(&token).await {
                Ok(()) => {
                    let _ = tx2.send(AppEvent::AuthSuccess).await;
                }
                Err(e) => {
                    let _ = tx2.send(AppEvent::Error(e.to_string())).await;
                }
            }
        });
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Main loop
    let result = run(&mut terminal, &mut app, &mut client, &mut rx, &tx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    client: &mut api::BlackDuckClient,
    rx: &mut mpsc::Receiver<AppEvent>,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|f| ui::render(f, app))?;

        // Handle async events from background tasks
        while let Ok(event) = rx.try_recv() {
            handle_app_event(app, client, event, tx).await?;
        }

        // Handle keyboard events
        handle_events(app, client, tx).await?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "event handler dispatches across many event variants; each arm is simple"
)]
async fn handle_app_event(
    app: &mut App,
    client: &mut api::BlackDuckClient,
    event: AppEvent,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    match event {
        AppEvent::AuthSuccess => {
            // Sync the bearer token back to our client by re-authenticating the stored client.
            // The spawned task has a clone of the client, so we re-authenticate the main instance.
            let token = app.config.server.api_token.clone().unwrap_or_default();
            let url = app.config.server.url.clone();
            let accept_invalid = app.config.server.accept_invalid_certs;
            match api::BlackDuckClient::new(&url, accept_invalid) {
                Ok(mut new_client) => {
                    if let Err(e) = new_client.authenticate(&token).await {
                        app.set_error(format!("Re-auth failed: {e}"));
                        return Ok(());
                    }
                    *client = new_client;
                }
                Err(e) => {
                    app.set_error(format!("Client rebuild failed: {e}"));
                    return Ok(());
                }
            }

            app.loading = false;
            app.screen = Screen::Projects;
            app.login_error = None;
            app.set_status("Authenticated successfully");
            load_projects(app, client, tx);
        }

        AppEvent::ProjectsLoaded {
            items: projects,
            total,
            offset,
        } => {
            app.loading = false;
            let count = projects.len();
            app.projects = StatefulList::new(projects);
            app.projects_total = total;
            app.projects_offset = offset;
            app.set_status(format!("Loaded {count} projects"));
        }

        AppEvent::VersionsLoaded {
            items: versions,
            total,
            offset,
        } => {
            app.loading = false;
            let count = versions.len();
            app.versions = StatefulList::new(versions);
            app.versions_total = total;
            app.versions_offset = offset;
            app.set_status(format!("Loaded {count} versions"));
        }

        AppEvent::ComponentsLoaded {
            items: components,
            total,
            offset,
        } => {
            app.loading = false;
            let count = components.len();
            app.components = StatefulList::new(components);
            app.components_total = total;
            app.components_offset = offset;
            app.set_status(format!(
                "Loaded {count} components (page offset {offset}, total {total})"
            ));
        }

        AppEvent::VulnerabilitiesLoaded {
            items: vulns,
            total,
            offset,
        } => {
            app.loading = false;
            let count = vulns.len();
            app.vulnerabilities = StatefulList::new(vulns);
            app.vulnerabilities_total = total;
            app.vulnerabilities_offset = offset;
            app.set_status(format!(
                "Loaded {count} vulnerabilities (page offset {offset}, total {total})"
            ));
        }

        AppEvent::PolicyViolationsLoaded {
            items: violations,
            total,
            offset,
        } => {
            app.loading = false;
            let count = violations.len();
            app.policy_violations = StatefulList::new(violations);
            app.policy_violations_total = total;
            app.policy_violations_offset = offset;
            app.set_status(format!(
                "Loaded {count} policy violations (page offset {offset}, total {total})"
            ));
        }

        AppEvent::PolicyRulesLoaded(rules) => {
            // Deduplicate and sort the incoming rules by name, then store on App.
            let mut sorted = rules;
            sorted.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            sorted.dedup_by(|a, b| a.1 == b.1); // dedup by ID
            app.available_policy_rules = sorted;
        }

        AppEvent::Error(msg) => {
            app.set_error(msg.clone());
            // If auth failed while on the Projects screen (startup auto-auth),
            // drop back to Login so the user can correct the URL / token.
            if app.screen == Screen::Projects && !app.loading {
                app.screen = Screen::Login;
                app.login_error = Some(msg);
            } else if app.screen == Screen::Login {
                app.login_error = Some(msg);
            }
            app.loading = false;
        }
    }

    Ok(())
}
