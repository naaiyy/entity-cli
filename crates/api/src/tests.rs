use super::build_router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::StreamExt;
use serde_json::Value;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use tower::util::ServiceExt;

fn write_file(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, content);
}

#[tokio::test]
async fn session_init_then_docs_read_uses_retained_engine() {
    let packs = TempDir::new().unwrap();
    let pack_root = packs.path().join("entity-auth");
    let docs_dir = pack_root.join("docs");
    let comps_dir = pack_root.join("components");
    let content_dir = docs_dir.join("content");
    let doc_path = content_dir.join("getting-started.md");
    write_file(&doc_path, "hello world");

    let docs_nodes = json!([
        {
            "id": "entityauth:docs:getting-started",
            "kind": "doc",
            "title": "Getting Started",
            "meta": { "section": "Setup", "tags": ["intro", "setup"] },
            "prerequisites": [],
            "payload": { "contentPath": doc_path.to_string_lossy() }
        }
    ]);
    let comps_nodes = json!([]);
    write_file(
        &docs_dir.join("nodes.json"),
        &serde_json::to_string(&docs_nodes).unwrap(),
    );
    write_file(
        &comps_dir.join("nodes.json"),
        &serde_json::to_string(&comps_nodes).unwrap(),
    );

    let app = build_router().await.unwrap();

    let init_body = json!({"packsPath": packs.path().to_string_lossy()});
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session/init")
                .header("content-type", "application/json")
                .body(Body::from(init_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let read_body = json!({"nodeId": "entityauth:docs:getting-started"});
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/docs/read")
                .header("content-type", "application/json")
                .body(Body::from(read_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn bridge_start_then_status_returns_state() {
    let packs = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();
    let pack_root = packs.path().join("entity-auth/bridge");
    fs::create_dir_all(&pack_root).unwrap();
    let runner = pack_root.join("runner.js");
    write_file(&runner, "console.log('noop');");
    let nodes = json!([
        {
            "id": "entityauth:bridge:test",
            "kind": "bridge",
            "title": "Test",
            "meta": {},
            "prerequisites": [],
            "payload": { "runner": runner.to_string_lossy() }
        }
    ]);
    write_file(&pack_root.join("nodes.json"), &nodes.to_string());

    let app = build_router().await.unwrap();

    let init_body = json!({"packsPath": packs.path().to_string_lossy()});
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session/init")
                .header("content-type", "application/json")
                .body(Body::from(init_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let start_body = json!({
        "nodeId": "entityauth:bridge:test",
        "workspace": workspace.path().to_string_lossy(),
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/bridge/start")
                .header("content-type", "application/json")
                .body(Body::from(start_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let status_body = json!({
        "nodeId": "entityauth:bridge:test",
        "workspace": workspace.path().to_string_lossy(),
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/bridge/status")
                .header("content-type", "application/json")
                .body(Body::from(status_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mut body = res.into_body().into_data_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.unwrap();
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["status"], "pending");
}
