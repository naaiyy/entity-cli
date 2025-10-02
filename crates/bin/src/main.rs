use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;
mod support;

use cli::{Cli, Commands};
use commands::{bridge, docs, init, serve, setup, ui};
use support::AppContext;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let ctx = AppContext::new(cli.packs.clone());

    match cli.command {
        Commands::Init(args) => init::run(&ctx, args)?,
        Commands::Docs(cmd) => docs::run(&ctx, cmd)?,
        Commands::Ui(cmd) => ui::run(&ctx, cmd)?,
        Commands::Setup(cmd) => setup::run(&ctx, cmd)?,
        Commands::Bridge(cmd) => bridge::run(&ctx, cmd)?,
        Commands::Serve(args) => serve::run(args)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests;
