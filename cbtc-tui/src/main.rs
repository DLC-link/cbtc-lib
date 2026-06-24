//! cbtc-tui — interactive terminal UI over the `cbtc` library.

mod app;
mod config;
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
    /// Import a `.env` file as a profile into the config, then exit (no TUI).
    #[arg(long, value_name = "ENV_FILE")]
    import_env: Option<PathBuf>,
    /// Name for the imported profile (defaults to the file's `ENVIRONMENT` value).
    #[arg(long)]
    profile_name: Option<String>,
    /// When importing, also set the new profile as the default.
    #[arg(long)]
    set_default: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _guard = init_logging(&args.log_level)?;

    let config_path = args.config.clone().unwrap_or_else(config::config_path);

    if let Some(env_file) = args.import_env.as_deref() {
        return run_import(
            env_file,
            &config_path,
            args.profile_name.clone(),
            args.set_default,
        );
    }

    let config = if config_path.exists() {
        Config::load(&config_path).context("loading config")?
    } else {
        tracing::warn!("no config at {}; starting empty", config_path.display());
        Config::default()
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, App::new(config), &config_path).await;
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
        .unwrap_or_else(|_| EnvFilter::new(format!("cbtc_tui={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Ok(guard)
}

/// Import a `.env` file into the config as a profile (merging into any existing
/// config) and persist it, without launching the TUI. The secret password flows
/// file → config and is never printed.
fn run_import(
    env_file: &Path,
    config_path: &Path,
    profile_name: Option<String>,
    set_default: bool,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(env_file)
        .with_context(|| format!("reading {}", env_file.display()))?;

    // Default the profile name to the file's ENVIRONMENT (e.g. "mainnet").
    let name = profile_name.unwrap_or_else(|| {
        env_import::parse_env(&content)
            .get("ENVIRONMENT")
            .cloned()
            .unwrap_or_else(|| "imported".to_string())
    });

    let (mut profile, override_env) = env_import::import(&content, &name);
    if profile.ledger_host.is_empty() || profile.keycloak_host.is_empty() {
        anyhow::bail!(
            "{} is missing LEDGER_HOST/KEYCLOAK_HOST — cannot build a usable profile",
            env_file.display()
        );
    }

    let mut config = if config_path.exists() {
        Config::load(config_path).context("loading existing config")?
    } else {
        Config::default()
    };

    // Preserve the previously-selected party when re-importing the same profile.
    if profile.last_selected_party.is_none()
        && let Some(existing) = config.profiles.iter().find(|p| p.name == name)
    {
        profile.last_selected_party = existing.last_selected_party.clone();
    }

    // Only write an environment override when the file actually carried one.
    if let Some((env_name, env)) = override_env {
        config.environments.insert(env_name, env);
    }

    // Replace a same-named profile in place, otherwise append.
    if let Some(slot) = config.profiles.iter_mut().find(|p| p.name == name) {
        *slot = profile;
    } else {
        config.profiles.push(profile);
    }

    if set_default {
        config.default_profile = Some(name.clone());
    }

    config
        .save(config_path)
        .with_context(|| format!("saving config to {}", config_path.display()))?;

    println!(
        "Imported profile '{name}' into {} ({} profile total).",
        config_path.display(),
        config.profiles.len()
    );
    if set_default {
        println!("Set '{name}' as the default profile.");
    }
    Ok(())
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    mut app: App,
    config_path: &Path,
) -> anyhow::Result<()> {
    let theme = Theme::detect();
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    event::spawn_input_reader(tx.clone());

    let mut spinner: usize = 0;
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(120));
    terminal.draw(|f| ui::draw(f, &app, &theme, spinner))?;

    loop {
        let ev = tokio::select! {
            maybe_ev = rx.recv() => match maybe_ev {
                Some(ev) => ev,
                None => break,
            },
            _ = ticker.tick() => {
                // Animate the spinner only while an operation is in flight.
                if app.loading {
                    spinner = spinner.wrapping_add(1);
                    terminal.draw(|f| ui::draw(f, &app, &theme, spinner))?;
                }
                continue;
            }
        };
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
                        let err_effects =
                            app.update(Event::OpResult(Err("no active party/session".to_string())));
                        if !err_effects.is_empty() {
                            tracing::warn!("unexpected effects from synthetic error: {err_effects:?}");
                        }
                    }
                }
                Effect::RunCommand(command) => {
                    if let Some(ctx) = build_context(&app) {
                        event::spawn_command(tx.clone(), command, ctx);
                    } else {
                        let _ = app
                            .update(Event::CommandResult(Err("no active party/session".to_string())));
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
        persist_selection(&mut app, config_path);
        terminal.draw(|f| ui::draw(f, &app, &theme, spinner))?;
    }
    tracing::debug!("event channel closed; exiting run loop");
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
        registry_url: env.registry_url,
        decentralized_party_id: env.decentralized_party_id,
        user_name: profile.keycloak_username.clone(),
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
