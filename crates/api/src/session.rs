use std::path::PathBuf;

use axum::{Json, extract::State};
use engine::Engine;
use entity_core::error::CoreError;
use serde::Deserialize;

use crate::state::{AppState, SessionState};

#[derive(Deserialize)]
pub struct SessionInitReq {
    #[serde(rename = "packsPath")]
    packs_path: String,
    #[allow(dead_code)]
    product: Option<String>,
}

pub async fn session_init(
    State(state): State<AppState>,
    Json(req): Json<SessionInitReq>,
) -> Json<serde_json::Value> {
    let packs_path = PathBuf::from(&req.packs_path);
    match Engine::bootstrap(packs_path.clone(), req.product.as_deref()) {
        Ok((engine, graph)) => {
            state.set_session(SessionState::new(engine.clone(), graph.clone(), packs_path));
            Json(serde_json::to_value(&graph).unwrap())
        }
        Err(err) => {
            let env = if let Some(core) = err.downcast_ref::<CoreError>() {
                // include packsPath in details for PACKS_NOT_FOUND
                let details = if matches!(core, CoreError::PacksNotFound(_)) {
                    Some(serde_json::json!({"packsPath": req.packs_path}))
                } else {
                    None
                };
                core.envelope(details)
            } else {
                CoreError::InvalidDescriptor(err.to_string()).envelope(None)
            };
            Json(serde_json::to_value(env).unwrap())
        }
    }
}
