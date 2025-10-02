use std::path::PathBuf;

use axum::{Json, extract::State};
use engine::Engine;
use entity_core::error::CoreError;
use executors::ComponentsExecutor;
use serde::Deserialize;

use crate::state::{AppState, SessionState};

#[derive(Deserialize)]
pub struct UiInstallReq {
    mode: Option<String>,
    names: Option<Vec<String>>,
    workspace: Option<String>,
    #[serde(rename = "nodeId")]
    node_id: Option<String>,
    #[serde(rename = "packsPath")]
    packs_path: Option<String>,
    #[allow(dead_code)]
    product: Option<String>,
}

pub async fn ui_install(
    State(state): State<AppState>,
    Json(req): Json<UiInstallReq>,
) -> Json<serde_json::Value> {
    if req.mode.is_none() {
        let err =
            CoreError::MissingSelections(vec!["selection.mode".into(), "selection.names".into()]);
        return Json(
            serde_json::to_value(err.envelope(Some(serde_json::json!({
                "missing": ["selection.mode", "selection.names"]
            }))))
            .unwrap(),
        );
    }

    let maybe_session = state.session();
    let engine = if let Some(sess) = maybe_session {
        sess.engine
    } else if let Some(p) = req.packs_path.clone() {
        match Engine::bootstrap(PathBuf::from(&p), req.product.as_deref()) {
            Ok((engine, graph)) => {
                state.set_session(SessionState::new(engine.clone(), graph, PathBuf::from(&p)));
                engine
            }
            Err(err) => {
                let env = if let Some(core) = err.downcast_ref::<CoreError>() {
                    let details = if matches!(core, CoreError::PacksNotFound(_)) {
                        Some(serde_json::json!({"packsPath": p}))
                    } else {
                        None
                    };
                    core.envelope(details)
                } else {
                    CoreError::InvalidDescriptor(err.to_string()).envelope(None)
                };
                return Json(serde_json::to_value(env).unwrap());
            }
        }
    } else {
        let err = CoreError::PacksNotFound("<unset>".into());
        let env = err.envelope(Some(
            serde_json::json!({"packsPath": serde_json::Value::Null}),
        ));
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = ComponentsExecutor::new(engine.registry());
    let names_opt = req.names;
    let ws = req
        .workspace
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    match exec.install(
        &req.node_id
            .unwrap_or_else(|| "entityauth:components:install".into()),
        &req.mode.unwrap(),
        names_opt,
        &ws,
    ) {
        Ok(report) => Json(serde_json::json!({
            "copied": report
                .copied
                .iter()
                .map(|c| serde_json::json!({"from": c.from, "to": c.to, "count": c.count}))
                .collect::<Vec<_>>(),
            "notes": report.notes,
        })),
        Err(err) => {
            let details = match &err {
                CoreError::MissingSelections(keys) => Some(serde_json::json!({"missing": keys})),
                CoreError::InvalidSelection(msg) => Some(serde_json::json!({"message": msg})),
                CoreError::InvalidNames(list) => Some(serde_json::json!({"invalidNames": list})),
                _ => None,
            };
            Json(serde_json::to_value(err.envelope(details)).unwrap())
        }
    }
}
