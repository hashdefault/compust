//! Compust: an experimental standalone X11 compositor.
mod animation;
mod atoms;
mod capabilities;
mod compositor;
mod config;
mod events;
mod picture;
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
    /// TOML configuration (defaults are used when omitted).
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
    let config = config::Config::load(cli.config.as_deref())?;
    if cli.check_config {
        writeln!(std::io::stdout().lock(), "configuration valid")?;
        return Ok(());
    }
    let session = session::Session::connect(cli.display.as_deref())?;
    if cli.diagnose {
        session.capabilities.report()?;
        return Ok(());
    }
    compositor::Compositor::new(session, config)?.run()
}
