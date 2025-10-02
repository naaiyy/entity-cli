use std::path::PathBuf;

use anyhow::Result;
use engine::Engine;
use executors::SetupExecutor;

use crate::cli::{SetupCmd, SetupRunArgs, SetupSubcommand};
use crate::support::{AppContext, emit_error};

pub fn run(ctx: &AppContext, SetupCmd { command }: SetupCmd) -> Result<()> {
    match command {
        SetupSubcommand::Run(args) => run_setup(ctx, args),
    }
}

fn run_setup(ctx: &AppContext, args: SetupRunArgs) -> Result<()> {
    let SetupRunArgs {
        product,
        node,
        workspace,
    } = args;

    let packs = match ctx.resolve_packs() {
        Ok(p) => p,
        Err(e) => {
            emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                e.to_string(),
            ));
            return Ok(());
        }
    };

    match Engine::bootstrap(packs, Some(&product)) {
        Ok((engine, _graph)) => {
            let exec = SetupExecutor::new(engine.registry());
            let ws = workspace
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap());

            match exec.run(node.as_str(), &ws) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "scaffolded": report.scaffolded,
                            "copied": report
                                .copied
                                .iter()
                                .map(|c| serde_json::json!({
                                    "from": c.from,
                                    "to": c.to,
                                    "count": c.count
                                }))
                                .collect::<Vec<_>>(),
                            "notes": report.notes,
                        }))?
                    );
                }
                Err(err) => emit_error(&err),
            }
        }
        Err(err) => {
            if let Some(core) = err.downcast_ref() {
                emit_error(core);
            } else {
                emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                    err.to_string(),
                ));
            }
        }
    }

    Ok(())
}
