use std::path::PathBuf;

use anyhow::Result;
use engine::Engine;
use executors::BridgeExecutor;
use uuid::Uuid;

use crate::cli::{BridgeAttachArgs, BridgeCmd, BridgeHeartbeatArgs, BridgeSubcommand};
use crate::support::{AppContext, emit_error};

pub fn run(ctx: &AppContext, BridgeCmd { command }: BridgeCmd) -> Result<()> {
    match command {
        BridgeSubcommand::Scaffold(args) => scaffold(ctx, args.base),
        BridgeSubcommand::Start(args) => start(ctx, args.base),
        BridgeSubcommand::Status(args) => status(ctx, args.base),
        BridgeSubcommand::Stop(args) => stop(ctx, args.base),
        BridgeSubcommand::Attach(args) => attach(ctx, args),
        BridgeSubcommand::Heartbeat(args) => heartbeat(ctx, args),
    }
}

fn scaffold(ctx: &AppContext, base: crate::cli::BridgeArgsBase) -> Result<()> {
    let packs = match ctx.resolve_packs() {
        Ok(p) => p,
        Err(e) => {
            emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                e.to_string(),
            ));
            return Ok(());
        }
    };

    match Engine::bootstrap(packs, Some(&base.product)) {
        Ok((engine, _graph)) => {
            let exec = BridgeExecutor::new(engine.registry());
            let ws = workspace_dir(base.workspace)?;

            match exec.scaffold(base.node.as_str(), &ws) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
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

fn start(ctx: &AppContext, base: crate::cli::BridgeArgsBase) -> Result<()> {
    let packs = match ctx.resolve_packs() {
        Ok(p) => p,
        Err(e) => {
            emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                e.to_string(),
            ));
            return Ok(());
        }
    };

    match Engine::bootstrap(packs.clone(), Some(&base.product)) {
        Ok((engine, _graph)) => {
            let exec = BridgeExecutor::new(engine.registry());
            let ws = workspace_dir(base.workspace.clone())?;
            let workspace_display = ws.display().to_string();
            let node_id = base.node.clone();
            let packs_display = packs.display().to_string();

            match exec.spawn_descriptor(base.node.as_str()) {
                Ok(info) => {
                    let state_id = Uuid::new_v4().to_string();
                    match exec.persist_state(
                        base.node.as_str(),
                        info.clone(),
                        &ws,
                        packs,
                        &state_id,
                    ) {
                        Ok(()) => {
                            let env = info
                                .env
                                .into_iter()
                                .map(|(key, value)| {
                                    serde_json::json!({
                                        "key": key,
                                        "value": value,
                                    })
                                })
                                .collect::<Vec<_>>();
                            let payload = serde_json::json!({
                                "stateId": state_id,
                                "nodeId": node_id,
                                "entry": info.entry,
                                "args": info.args,
                                "env": env,
                                "cwd": info.cwd,
                                "configPath": info.config_path,
                                "logsPath": info.logs_path,
                                "workspace": workspace_display,
                                "packsRoot": packs_display,
                                "status": "pending",
                            });
                            println!("{}", serde_json::to_string_pretty(&payload)?);
                        }
                        Err(err) => emit_error(&err),
                    }
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

fn status(_ctx: &AppContext, base: crate::cli::BridgeArgsBase) -> Result<()> {
    let ws = workspace_dir(base.workspace)?;

    match BridgeExecutor::read_state(&ws, base.node.as_str()) {
        Ok(Some(state)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "stateId": state.id,
                    "nodeId": state.node_id,
                    "pid": state.pid,
                    "status": state.status,
                    "entry": state.process.entry,
                    "args": state.process.args,
                    "env": state
                        .process
                        .env
                        .into_iter()
                        .map(|(key, value)| serde_json::json!({
                            "key": key,
                            "value": value
                        }))
                        .collect::<Vec<_>>(),
                    "workspace": state.workspace,
                    "packsRoot": state.packs_root,
                    "lastUpdated": state.updated_at,
                    "logs": state.logs_path,
                    "heartbeat": state.heartbeat_at,
                    "statusMessage": state.status_message,
                    "exitCode": state.exit_code,
                }))?
            );
        }
        Ok(None) => {
            emit_error(&entity_core::error::CoreError::TargetNotFound(
                "bridge not started for requested node".into(),
            ));
        }
        Err(err) => emit_error(&err),
    }

    Ok(())
}

fn stop(_ctx: &AppContext, base: crate::cli::BridgeArgsBase) -> Result<()> {
    let ws = workspace_dir(base.workspace)?;

    match BridgeExecutor::stop(&ws, base.node.as_str()) {
        Ok(Some(result)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "stopped": true,
                    "pid": result.pid,
                    "status": result.status,
                    "stateId": result.state_id,
                }))?
            );
        }
        Ok(None) => {
            emit_error(&entity_core::error::CoreError::TargetNotFound(
                "no running bridge found for node".into(),
            ));
        }
        Err(err) => emit_error(&err),
    }

    Ok(())
}

fn attach(_ctx: &AppContext, args: BridgeAttachArgs) -> Result<()> {
    let ws = workspace_dir(args.base.workspace)?;
    match BridgeExecutor::attach_pid(
        &ws,
        args.base.node.as_str(),
        args.pid,
        args.status.as_deref(),
        args.status_message.as_deref(),
    ) {
        Ok(Some(state)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "stateId": state.id,
                    "nodeId": state.node_id,
                    "pid": state.pid,
                    "status": state.status,
                    "heartbeat": state.heartbeat_at,
                    "statusMessage": state.status_message,
                }))?
            );
        }
        Ok(None) => {
            emit_error(&entity_core::error::CoreError::TargetNotFound(
                "bridge state not found".into(),
            ));
        }
        Err(err) => emit_error(&err),
    }
    Ok(())
}

fn heartbeat(_ctx: &AppContext, args: BridgeHeartbeatArgs) -> Result<()> {
    let ws = workspace_dir(args.base.workspace)?;
    match BridgeExecutor::heartbeat(
        &ws,
        args.base.node.as_str(),
        args.status.as_deref(),
        args.status_message.as_deref(),
    ) {
        Ok(Some(state)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "stateId": state.id,
                    "nodeId": state.node_id,
                    "status": state.status,
                    "heartbeat": state.heartbeat_at,
                    "statusMessage": state.status_message,
                }))?
            );
        }
        Ok(None) => {
            emit_error(&entity_core::error::CoreError::TargetNotFound(
                "bridge state not found".into(),
            ));
        }
        Err(err) => emit_error(&err),
    }
    Ok(())
}

fn workspace_dir(workspace: Option<String>) -> Result<PathBuf> {
    match workspace {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(std::env::current_dir()?),
    }
}
