use std::path::PathBuf;

use axum::{Json, extract::State};
use entity_core::error::CoreError;
use executors::BridgeExecutor;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::{AppState, SessionState};

#[derive(Deserialize)]
pub struct BridgeScaffoldReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
}

#[derive(Deserialize)]
pub struct BridgeStartReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
}

#[derive(Deserialize)]
pub struct BridgeStatusReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
}

#[derive(Deserialize)]
pub struct BridgeStopReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
}

#[derive(Deserialize)]
pub struct BridgeAttachReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
    pid: i32,
    status: Option<String>,
    #[serde(rename = "statusMessage")]
    status_message: Option<String>,
}

#[derive(Deserialize)]
pub struct BridgeHeartbeatReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
    status: Option<String>,
    #[serde(rename = "statusMessage")]
    status_message: Option<String>,
}

pub async fn bridge_scaffold(
    State(state): State<AppState>,
    Json(req): Json<BridgeScaffoldReq>,
) -> Json<serde_json::Value> {
    let Some(session) = state.session() else {
        let env = CoreError::InvalidDescriptor("session not initialized".into()).envelope(None);
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = BridgeExecutor::new(session.engine.registry());
    let ws = workspace_or_default(req.workspace);
    match exec.scaffold(&req.node_id, &ws) {
        Ok(report) => Json(serde_json::json!({
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
        })),
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

pub async fn bridge_start(
    State(state): State<AppState>,
    Json(req): Json<BridgeStartReq>,
) -> Json<serde_json::Value> {
    let Some(SessionState {
        engine, packs_path, ..
    }) = state.session()
    else {
        let env = CoreError::InvalidDescriptor("session not initialized".into()).envelope(None);
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = BridgeExecutor::new(engine.registry());
    let ws = workspace_or_default(req.workspace);
    match exec.spawn_descriptor(&req.node_id) {
        Ok(info) => {
            let state_id = Uuid::new_v4().to_string();
            match exec.persist_state(&req.node_id, info, &ws, packs_path, &state_id) {
                Ok(_) => Json(serde_json::json!({"stateId": state_id})),
                Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
            }
        }
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

pub async fn bridge_status(Json(req): Json<BridgeStatusReq>) -> Json<serde_json::Value> {
    let ws = workspace_or_default(req.workspace);
    match BridgeExecutor::read_state(&ws, &req.node_id) {
        Ok(Some(state)) => Json(serde_json::json!({
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
            "logs": state.logs_path,
            "lastUpdated": state.updated_at,
            "heartbeat": state.heartbeat_at,
            "statusMessage": state.status_message,
            "exitCode": state.exit_code,
        })),
        Ok(None) => {
            let err = CoreError::TargetNotFound("bridge not started for requested node".into());
            Json(serde_json::to_value(err.envelope(None)).unwrap())
        }
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

pub async fn bridge_stop(Json(req): Json<BridgeStopReq>) -> Json<serde_json::Value> {
    let ws = workspace_or_default(req.workspace);
    match BridgeExecutor::stop(&ws, &req.node_id) {
        Ok(Some(result)) => Json(serde_json::json!({
            "stopped": true,
            "pid": result.pid,
            "status": result.status,
            "stateId": result.state_id,
        })),
        Ok(None) => {
            let err = CoreError::TargetNotFound("no running bridge found for node".into());
            Json(serde_json::to_value(err.envelope(None)).unwrap())
        }
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

pub async fn bridge_attach(Json(req): Json<BridgeAttachReq>) -> Json<serde_json::Value> {
    let ws = workspace_or_default(req.workspace);
    match BridgeExecutor::attach_pid(
        &ws,
        &req.node_id,
        req.pid,
        req.status.as_deref(),
        req.status_message.as_deref(),
    ) {
        Ok(Some(state)) => Json(serde_json::json!({
            "stateId": state.id,
            "nodeId": state.node_id,
            "pid": state.pid,
            "status": state.status,
            "heartbeat": state.heartbeat_at,
            "statusMessage": state.status_message,
        })),
        Ok(None) => {
            let err = CoreError::TargetNotFound("bridge state not found".into());
            Json(serde_json::to_value(err.envelope(None)).unwrap())
        }
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

pub async fn bridge_heartbeat(Json(req): Json<BridgeHeartbeatReq>) -> Json<serde_json::Value> {
    let ws = workspace_or_default(req.workspace);
    match BridgeExecutor::heartbeat(
        &ws,
        &req.node_id,
        req.status.as_deref(),
        req.status_message.as_deref(),
    ) {
        Ok(Some(state)) => Json(serde_json::json!({
            "stateId": state.id,
            "nodeId": state.node_id,
            "status": state.status,
            "heartbeat": state.heartbeat_at,
            "statusMessage": state.status_message,
        })),
        Ok(None) => {
            let err = CoreError::TargetNotFound("bridge state not found".into());
            Json(serde_json::to_value(err.envelope(None)).unwrap())
        }
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}

fn workspace_or_default(workspace: Option<String>) -> PathBuf {
    workspace
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap())
}
