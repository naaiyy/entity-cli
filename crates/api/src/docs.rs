use std::path::PathBuf;

use axum::{Json, extract::State};
use engine::Engine;
use entity_core::error::CoreError;
use serde::Deserialize;

use crate::state::{AppState, SessionState};
use executors::DocsExecutor;

#[derive(Deserialize)]
pub struct DocsReadReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(rename = "packsPath")]
    packs_path: Option<String>,
    #[allow(dead_code)]
    product: Option<String>,
}

pub async fn docs_read(
    State(state): State<AppState>,
    Json(req): Json<DocsReadReq>,
) -> Json<serde_json::Value> {
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

    let exec = DocsExecutor::new(engine.registry());
    match exec.read(&req.node_id) {
        Ok(content) => Json(serde_json::json!({"content": content})),
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
