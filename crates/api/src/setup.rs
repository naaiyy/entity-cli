use std::path::PathBuf;

use axum::{Json, extract::State};
use entity_core::error::CoreError;
use executors::SetupExecutor;
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetupRunReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    workspace: Option<String>,
}

pub async fn setup_run(
    State(state): State<AppState>,
    Json(req): Json<SetupRunReq>,
) -> Json<serde_json::Value> {
    let Some(session) = state.session() else {
        let env = CoreError::InvalidDescriptor("session not initialized".into()).envelope(None);
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = SetupExecutor::new(session.engine.registry());
    let ws = req
        .workspace
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    match exec.run(&req.node_id, &ws) {
        Ok(report) => Json(serde_json::json!({
            "scaffolded": report.scaffolded,
            "copied": report
                .copied
                .iter()
                .map(|c| serde_json::json!({"from": c.from, "to": c.to, "count": c.count}))
                .collect::<Vec<_>>(),
            "notes": report.notes,
        })),
        Err(err) => Json(serde_json::to_value(err.envelope(None)).unwrap()),
    }
}
