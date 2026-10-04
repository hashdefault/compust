//! Compust: an experimental standalone X11 compositor.
mod animation;
mod atoms;
mod capabilities;
mod compositor;
mod config;
mod events;
mod picture;
mod region;
mod renderer;
mod scene;
mod session;
mod surface;

use anyhow::Result;
use clap::Parser;
use std::{io::Write, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "A minimal experimental Rust compositor for Xorg and XLibre"
)]
struct Cli {
    /// X display (defaults to DISPLAY).
    #[arg(long)]
    display: Option<String>,
    /// TOML configuration (defaults to compust/compust.toml in the XDG configuration
    /// directories, then built-in defaults). SIGUSR1 reloads it.
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Inspect server extensions without becoming the compositor.
    #[arg(long)]
    diagnose: bool,
    /// Validate configuration without connecting to X11.
    #[arg(long)]
    check_config: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("compust=info")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
    let source = config::Source::new(cli.config);
    let (config, path) = source.load()?;
    if cli.check_config {
        let mut out = std::io::stdout().lock();
        match path {
            Some(path) => writeln!(out, "configuration valid: {}", path.display())?,
            None => writeln!(out, "no configuration file found; built-in defaults apply")?,
        }
        return Ok(());
    }
    let session = session::Session::connect(cli.display.as_deref())?;
    if cli.diagnose {
        session.capabilities.report()?;
        return Ok(());
    }
    if let Some(path) = &path {
        tracing::info!(path = %path.display(), "configuration loaded");
    } else {
        tracing::info!("no configuration file found; using built-in defaults");
    }
    compositor::Compositor::new(session, source, config)?.run()
}
