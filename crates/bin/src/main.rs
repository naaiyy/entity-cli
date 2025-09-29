use std::path::PathBuf;

use anyhow::Result;
use api::build_router;
use axum::serve;
use clap::{Args, Parser, Subcommand};
use engine::Engine;
use entity_core::error::CoreError;
use executors::{ComponentsExecutor, DocsExecutor};
use std::fs;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
#[command(
    name = "entity-cli",
    disable_help_flag = true,
    disable_help_subcommand = true,
    disable_version_flag = true
)]
#[command(about = "Entity CLI - Global Graph Engine", long_about = None)]
struct Cli {
    /// Packs root directory (global; can be placed anywhere)
    #[arg(long, global = true)]
    packs: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Emit the graph once as JSON
    Init(InitArgs),

    /// Docs related commands
    Docs(DocsCmd),

    /// UI installation commands
    Ui(UiCmd),

    /// Serve minimal HTTP API for agents
    Serve(ServeCmd),
}

// no per-command packs; use global

#[derive(Args, Debug)]
struct InitArgs {
    /// Product/pack name (e.g., entity-auth, microsoft)
    product: String,
}

#[derive(Args, Debug)]
struct DocsCmd {
    #[command(subcommand)]
    command: DocsSubcommand,
}

#[derive(Subcommand, Debug)]
enum DocsSubcommand {
    /// Read a doc node content
    Read(DocsReadArgs),
}

#[derive(Args, Debug)]
struct UiCmd {
    #[command(subcommand)]
    command: UiSubcommand,
}

#[derive(Subcommand, Debug)]
enum UiSubcommand {
    /// Install UI components
    Install(UiInstallArgs),
}

#[derive(Args, Debug)]
struct DocsReadArgs {
    /// Product/pack name (e.g., entity-auth)
    product: String,
    #[arg(long)]
    node: String,
}

#[derive(Args, Debug)]
struct UiInstallArgs {
    /// Product/pack name (e.g., entity-auth)
    product: String,
    #[arg(long, required = false)]
    mode: Option<String>,
    #[arg(long, num_args = 1..)]
    names: Option<Vec<String>>, // allow omission to distinguish for mode=all
    #[arg(long, default_value = "entityauth:components:install")]
    node: String,
}

#[derive(Args, Debug)]
struct ServeCmd {
    /// Address to bind (e.g., 127.0.0.1:8787)
    #[arg(long, default_value = "127.0.0.1:8787")]
    addr: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init(InitArgs { product }) => {
            let packs = resolve_packs(cli.packs.clone().unwrap_or_else(|| PathBuf::from("packs")))?;
            match Engine::bootstrap(packs, Some(&product)) {
                Ok((_engine, graph)) => {
                    println!("{}", serde_json::to_string_pretty(&graph)?);
                }
                Err(err) => {
                    if let Some(core) = err.downcast_ref::<CoreError>() {
                        emit_error(core);
                    } else {
                        emit_error(&CoreError::InvalidDescriptor(err.to_string()));
                    }
                }
            }
        }
        Commands::Docs(DocsCmd { command }) => match command {
            DocsSubcommand::Read(DocsReadArgs { product, node }) => {
                let packs = match resolve_packs(cli.packs.clone().unwrap_or_else(|| PathBuf::from("packs"))) {
                    Ok(p) => p,
                    Err(e) => {
                        emit_error(&CoreError::InvalidDescriptor(e.to_string()));
                        return Ok(());
                    }
                };
                match Engine::bootstrap(packs, Some(&product)) {
                    Ok((engine, _graph)) => {
                        let exec = DocsExecutor::new(engine.registry());
                        match exec.read(node.as_str()) {
                            Ok(content) => {
                                println!("{}", content);
                            }
                            Err(err) => emit_error(&err),
                        }
                    }
                    Err(err) => {
                        if let Some(core) = err.downcast_ref::<CoreError>() {
                            emit_error(core);
                        } else {
                            emit_error(&CoreError::InvalidDescriptor(err.to_string()));
                        }
                    }
                }
            }
        },
        Commands::Ui(UiCmd { command }) => match command {
            UiSubcommand::Install(UiInstallArgs { product, mode, names, node }) => {
                // Early missing selections handling to align with HTTP API behavior
                if mode.is_none() {
                    emit_error(&CoreError::MissingSelections(vec![
                        "selection.mode".into(),
                        "selection.names".into(),
                    ]));
                    return Ok(());
                }

                let packs = match resolve_packs(cli.packs.clone().unwrap_or_else(|| PathBuf::from("packs"))) {
                    Ok(p) => p,
                    Err(e) => {
                        emit_error(&CoreError::InvalidDescriptor(e.to_string()));
                        return Ok(());
                    }
                };
                match Engine::bootstrap(packs, Some(&product)) {
                    Ok((engine, _graph)) => {
                        let exec = ComponentsExecutor::new(engine.registry());
                        let cwd = std::env::current_dir().unwrap();
                        match exec.install(node.as_str(), &mode.unwrap(), names, &cwd) {
                            Ok(report) => {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&serde_json::json!({
                                        "copied": report.copied.iter().map(|c| serde_json::json!({"from": c.from, "to": c.to, "count": c.count})).collect::<Vec<_>>(),
                                        "notes": report.notes,
                                    }))?
                                );
                            }
                            Err(err) => emit_error(&err),
                        }
                    }
                    Err(err) => {
                        if let Some(core) = err.downcast_ref::<CoreError>() {
                            emit_error(core);
                        } else {
                            emit_error(&CoreError::InvalidDescriptor(err.to_string()));
                        }
                    }
                }
            }
        },
        Commands::Serve(ServeCmd { addr }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                let router = build_router().await.unwrap();
                let addr: SocketAddr = addr.parse().unwrap();
                let listener = TcpListener::bind(addr).await.unwrap();
                serve(listener, router.into_make_service()).await.unwrap();
            });
        }
    }

    Ok(())
}

fn emit_error(err: &CoreError) {
    let details = match err {
        CoreError::MissingSelections(keys) => Some(serde_json::json!({"missing": keys})),
        CoreError::InvalidSelection(msg) => Some(serde_json::json!({"message": msg})),
        CoreError::InvalidNames(list) => Some(serde_json::json!({"invalidNames": list})),
        CoreError::PacksNotFound(p) => Some(serde_json::json!({"packsPath": p})),
        _ => None,
    };
    let env = err.envelope(details);
    println!("{}", serde_json::to_string_pretty(&env).unwrap());
}

fn resolve_packs(flag: std::path::PathBuf) -> Result<std::path::PathBuf> {
    // precedence: flag -> env -> config -> default
    if flag != std::path::PathBuf::from("packs") {
        return Ok(flag);
    }
    if let Ok(env_path) = std::env::var("ENTITY_CLI_PACKS") {
        return Ok(env_path.into());
    }
    // config file entitycli.json with { "packsDir": "..." }
    let cfg_path = std::path::PathBuf::from("entitycli.json");
    if cfg_path.exists() {
        let content = fs::read_to_string(&cfg_path)?;
        let v: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(p) = v.get("packsDir").and_then(|x| x.as_str()) {
            return Ok(p.into());
        }
    }
    Ok(flag)
}
