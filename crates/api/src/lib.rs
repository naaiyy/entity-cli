use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::{Json, Router, routing::post, extract::State};
use engine::Engine;
use entity_core::model::GraphPackage;
use executors::{ComponentsExecutor, DocsExecutor};
use serde::Deserialize;

#[derive(Clone, Default)]
struct AppState {
    // Single-session state retained after /session/init
    inner: Arc<RwLock<Option<SessionState>>>,
}

#[derive(Clone)]
struct SessionState {
    engine: Engine,
    #[allow(dead_code)]
    graph: GraphPackage,
    #[allow(dead_code)]
    packs_path: PathBuf,
}

pub async fn build_router() -> anyhow::Result<Router> {
    let state = AppState { inner: Arc::new(RwLock::new(None)) };
    let router = Router::new()
        .route("/session/init", post(session_init))
        .route("/docs/read", post(docs_read))
        .route("/ui/install", post(ui_install))
        .with_state(state);
    Ok(router)
}

#[derive(Deserialize)]
struct SessionInitReq {
    #[serde(rename = "packsPath")]
    packs_path: String,
    #[allow(dead_code)]
    product: Option<String>,
}

async fn session_init(State(state): State<AppState>, Json(req): Json<SessionInitReq>) -> Json<serde_json::Value> {
    let packs_path = PathBuf::from(&req.packs_path);
    match Engine::bootstrap(packs_path.clone()) {
        Ok((engine, graph)) => {
            {
                let mut guard = state.inner.write().unwrap();
                *guard = Some(SessionState { engine, graph: graph.clone(), packs_path: packs_path.clone() });
            }
            Json(serde_json::to_value(&graph).unwrap())
        }
        Err(err) => {
            let env = if let Some(core) = err.downcast_ref::<entity_core::error::CoreError>() {
                // include packsPath in details for PACKS_NOT_FOUND
                let details = if matches!(core, entity_core::error::CoreError::PacksNotFound(_)) {
                    Some(serde_json::json!({"packsPath": req.packs_path}))
                } else { None };
                core.envelope(details)
            } else {
                entity_core::error::CoreError::InvalidDescriptor(err.to_string()).envelope(None)
            };
            Json(serde_json::to_value(env).unwrap())
        }
    }
}

#[derive(Deserialize)]
struct DocsReadReq {
    #[serde(rename = "nodeId")]
    node_id: String,
    #[serde(rename = "packsPath")]
    packs_path: Option<String>,
    #[allow(dead_code)]
    product: Option<String>,
}

async fn docs_read(State(state): State<AppState>, Json(req): Json<DocsReadReq>) -> Json<serde_json::Value> {
    // Prefer retained session; fallback to packsPath if provided
    let maybe_session = { state.inner.read().unwrap().clone() };
    let engine = if let Some(sess) = maybe_session {
        sess.engine
    } else if let Some(p) = req.packs_path.clone() {
        match Engine::bootstrap(PathBuf::from(&p)) {
            Ok((engine, graph)) => {
                let mut guard = state.inner.write().unwrap();
                *guard = Some(SessionState { engine: engine.clone(), graph, packs_path: PathBuf::from(p.clone()) });
                engine
            }
            Err(err) => {
                let env = if let Some(core) = err.downcast_ref::<entity_core::error::CoreError>() {
                    let details = if matches!(core, entity_core::error::CoreError::PacksNotFound(_)) {
                        Some(serde_json::json!({"packsPath": p}))
                    } else { None };
                    core.envelope(details)
                } else {
                    entity_core::error::CoreError::InvalidDescriptor(err.to_string()).envelope(None)
                };
                return Json(serde_json::to_value(env).unwrap());
            }
        }
    } else {
        let err = entity_core::error::CoreError::PacksNotFound("<unset>".into());
        let env = err.envelope(Some(serde_json::json!({"packsPath": serde_json::Value::Null})));
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = DocsExecutor::new(engine.registry());
    match exec.read(&req.node_id) {
        Ok(content) => Json(serde_json::json!({"content": content})),
        Err(err) => {
            let details = match &err {
                entity_core::error::CoreError::MissingSelections(keys) => {
                    Some(serde_json::json!({"missing": keys}))
                }
                entity_core::error::CoreError::InvalidSelection(msg) => {
                    Some(serde_json::json!({"message": msg}))
                }
                entity_core::error::CoreError::InvalidNames(list) => {
                    Some(serde_json::json!({"invalidNames": list}))
                }
                _ => None,
            };
            Json(serde_json::to_value(err.envelope(details)).unwrap())
        }
    }
}

#[derive(Deserialize)]
struct UiInstallReq {
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

async fn ui_install(State(state): State<AppState>, Json(req): Json<UiInstallReq>) -> Json<serde_json::Value> {
    // Combined missing selections if mode is absent
    if req.mode.is_none() {
        let err = entity_core::error::CoreError::MissingSelections(vec![
            "selection.mode".into(),
            "selection.names".into(),
        ]);
        return Json(serde_json::to_value(err.envelope(Some(serde_json::json!({
            "missing": ["selection.mode", "selection.names"]
        })))).unwrap());
    }

    // Prefer retained session; fallback to packsPath if provided
    let maybe_session = { state.inner.read().unwrap().clone() };
    let engine = if let Some(sess) = maybe_session {
        sess.engine
    } else if let Some(p) = req.packs_path.clone() {
        match Engine::bootstrap(PathBuf::from(&p)) {
            Ok((engine, graph)) => {
                let mut guard = state.inner.write().unwrap();
                *guard = Some(SessionState { engine: engine.clone(), graph, packs_path: PathBuf::from(p.clone()) });
                engine
            }
            Err(err) => {
                let env = if let Some(core) = err.downcast_ref::<entity_core::error::CoreError>() {
                    let details = if matches!(core, entity_core::error::CoreError::PacksNotFound(_)) {
                        Some(serde_json::json!({"packsPath": p}))
                    } else { None };
                    core.envelope(details)
                } else {
                    entity_core::error::CoreError::InvalidDescriptor(err.to_string()).envelope(None)
                };
                return Json(serde_json::to_value(env).unwrap());
            }
        }
    } else {
        let err = entity_core::error::CoreError::PacksNotFound("<unset>".into());
        let env = err.envelope(Some(serde_json::json!({"packsPath": serde_json::Value::Null})));
        return Json(serde_json::to_value(env).unwrap());
    };

    let exec = ComponentsExecutor::new(engine.registry());
    let names_opt = req.names;
    let ws = req.workspace.map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap());
    match exec.install(
        &req
            .node_id
            .unwrap_or_else(|| "entityauth:components:install".into()),
        &req.mode.unwrap(),
        names_opt,
        &ws,
    ) {
        Ok(report) => Json(serde_json::json!({
            "copied": report.copied.iter().map(|c| serde_json::json!({"from": c.from, "to": c.to, "count": c.count})).collect::<Vec<_>>(),
            "notes": report.notes,
        })),
        Err(err) => {
            let details = match &err {
                entity_core::error::CoreError::MissingSelections(keys) => {
                    Some(serde_json::json!({"missing": keys}))
                }
                entity_core::error::CoreError::InvalidSelection(msg) => {
                    Some(serde_json::json!({"message": msg}))
                }
                entity_core::error::CoreError::InvalidNames(list) => {
                    Some(serde_json::json!({"invalidNames": list}))
                }
                _ => None,
            };
            Json(serde_json::to_value(err.envelope(details)).unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt; // for `oneshot`
    use tempfile::TempDir;
    use std::fs;

    fn write_file(path: &std::path::Path, content: &str) {
        if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
        let _ = fs::write(path, content);
    }

    #[tokio::test]
    async fn session_init_then_docs_read_uses_retained_engine() {
        let packs = TempDir::new().unwrap();
        // layout: packs/entity-auth/{docs,components}
        let pack_root = packs.path().join("entity-auth");
        let docs_dir = pack_root.join("docs");
        let comps_dir = pack_root.join("components");
        let content_dir = docs_dir.join("content");
        let doc_path = content_dir.join("getting-started.md");
        write_file(&doc_path, "hello world");
        // nodes.json files
        let docs_nodes = serde_json::json!([
            {
                "id": "entityauth:docs:getting-started",
                "kind": "doc",
                "title": "Getting Started",
                "meta": { "section": "Setup", "tags": ["intro", "setup"] },
                "prerequisites": [],
                "payload": { "contentPath": doc_path.to_string_lossy() }
            }
        ]);
        let comps_nodes = serde_json::json!([]);
        write_file(&docs_dir.join("nodes.json"), &serde_json::to_string(&docs_nodes).unwrap());
        write_file(&comps_dir.join("nodes.json"), &serde_json::to_string(&comps_nodes).unwrap());

        let app = build_router().await.unwrap();

        // init
        let init_body = serde_json::json!({"packsPath": packs.path().to_string_lossy()});
        let res = app.clone().oneshot(Request::builder()
            .method("POST").uri("/session/init")
            .header("content-type","application/json")
            .body(Body::from(init_body.to_string())).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // docs/read without packsPath should succeed via retained session
        let read_body = serde_json::json!({"nodeId": "entityauth:docs:getting-started"});
        let res = app.oneshot(Request::builder()
            .method("POST").uri("/docs/read")
            .header("content-type","application/json")
            .body(Body::from(read_body.to_string())).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
