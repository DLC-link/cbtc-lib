//! cbtc-tui — interactive terminal UI over the `cbtc` library.

mod app;
mod config;
// import-from-.env feature; tested in env_import; UI/CLI binding is a deferred follow-up.
#[allow(dead_code)]
mod env_import;
mod error;
mod event;
mod ops;
mod session;
mod theme;
mod ui;

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

use crate::app::{App, Effect, Event};
use crate::config::Config;
use crate::ops::OpContext;
use crate::theme::Theme;

#[derive(Parser, Debug)]
#[command(name = "cbtc-tui", about = "Interactive TUI for CBTC on Canton")]
struct Args {
    /// Path to the config file (overrides the default location).
    #[arg(long, env = "CBTC_TUI_CONFIG")]
    config: Option<PathBuf>,
    /// Log level (trace|debug|info|warn|error).
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _guard = init_logging(&args.log_level)?;

    let config_path = args.config.unwrap_or_else(config::config_path);
    let config = if config_path.exists() {
        Config::load(&config_path).context("loading config")?
    } else {
        tracing::warn!("no config at {}; starting empty", config_path.display());
        Config::default()
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, App::new(config), config_path).await;
    ratatui::restore();
    result
}

/// Initialize file-based tracing; returns the appender guard (keep it alive).
fn init_logging(level: &str) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let dir = config::log_dir();
    std::fs::create_dir_all(&dir).context("creating log dir")?;
    let file = tracing_appender::rolling::daily(&dir, "cbtc-tui.log");
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("cbtc_tui={level},info")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Ok(guard)
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    mut app: App,
    config_path: PathBuf,
) -> anyhow::Result<()> {
    let theme = Theme::detect();
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    event::spawn_input_reader(tx.clone());

    let mut spinner: usize = 0;
    terminal.draw(|f| ui::draw(f, &app, &theme, spinner))?;

    while let Some(ev) = rx.recv().await {
        spinner = spinner.wrapping_add(1);
        let effects = app.update(ev);
        for effect in effects {
            match effect {
                Effect::Quit => return Ok(()),
                Effect::Login(idx) => {
                    if let Some(profile) = app.config.profiles.get(idx).cloned() {
                        event::spawn_login(tx.clone(), profile);
                    }
                }
                Effect::RunOp(op) => {
                    if let Some(ctx) = build_context(&app) {
                        event::spawn_op(tx.clone(), op, ctx);
                    } else {
                        app.update(Event::OpResult(Err("no active party/session".to_string())));
                    }
                }
                Effect::FetchParties => {
                    if let Some(idx) = app.active_profile
                        && let Some(profile) = app.config.profiles.get(idx).cloned()
                    {
                        event::spawn_login(tx.clone(), profile);
                    }
                }
            }
        }
        // Persist last-selected party as it changes.
        persist_selection(&mut app, &config_path);
        terminal.draw(|f| ui::draw(f, &app, &theme, spinner))?;
    }
    Ok(())
}

/// Assemble an `OpContext` from the active profile, environment, party, token.
fn build_context(app: &App) -> Option<OpContext> {
    let idx = app.active_profile?;
    let profile = app.config.profiles.get(idx)?;
    let env = app.config.resolved_environment(&profile.environment)?;
    let party = app.active_party.clone()?;
    let token = app.access_token.clone()?;
    Some(OpContext {
        ledger_host: profile.ledger_host.clone(),
        party,
        access_token: token,
        bitsafe_api_url: env.bitsafe_api_url,
        dar_dirs: Vec::new(),
    })
}

fn persist_selection(app: &mut App, config_path: &Path) {
    let (Some(idx), Some(party)) = (app.active_profile, app.active_party.clone()) else {
        return;
    };
    if let Some(profile) = app.config.profiles.get_mut(idx)
        && profile.last_selected_party.as_deref() != Some(party.as_str())
    {
        profile.last_selected_party = Some(party);
        if let Err(e) = app.config.save(config_path) {
            tracing::warn!("failed to persist config: {e}");
        }
    }
}
