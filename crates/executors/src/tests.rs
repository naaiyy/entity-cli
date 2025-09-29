use std::fs;

use entity_core::model::{Node, NodeKind, NodePayload};
use entity_core::registry::Registry;

use crate::{ComponentsExecutor, DocsExecutor};
use std::time::Instant;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn write_file(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn docs_read_happy_path() {
    let dir = temp_dir();
    let doc_path = dir.path().join("doc.md");
    write_file(&doc_path, "hello");
    let node = Node {
        id: "x:doc:one".into(),
        kind: NodeKind::Doc,
        title: "t".into(),
        meta: Default::default(),
        prerequisites: vec![],
        payload: NodePayload::Doc {
            content_path: doc_path.display().to_string(),
        },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = DocsExecutor::new(&reg);
    let out = exec.read("x:doc:one").unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn ui_install_mode_all_names_present_errors() {
    let dir = temp_dir();
    let pack_root = dir.path().join("pack/ui/SignIn");
    write_file(&pack_root.join("index.tsx"), "export const A = 1;\n");
    let node = Node {
        id: "x:comp:install".into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [("names".into(), serde_json::json!(["SignIn"]))]
            .into_iter()
            .collect(),
        prerequisites: vec![],
        payload: NodePayload::Component {
            source_root: dir.path().join("pack/ui").display().to_string(),
        },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let err = exec
        .install(
            "x:comp:install",
            "all",
            Some(vec!["SignIn".into()]),
            ws.path(),
        )
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("names must be omitted for mode all"));
}

#[test]
fn ui_install_single_copies_files() {
    let dir = temp_dir();
    let pack_root = dir.path().join("pack/ui/SignIn");
    write_file(&pack_root.join("index.tsx"), "export const A = 1;\n");
    let node = Node {
        id: "x:comp:install".into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [("names".into(), serde_json::json!(["SignIn"]))]
            .into_iter()
            .collect(),
        prerequisites: vec![],
        payload: NodePayload::Component {
            source_root: dir.path().join("pack/ui").display().to_string(),
        },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let rep = exec
        .install(
            "x:comp:install",
            "single",
            Some(vec!["SignIn".into()]),
            ws.path(),
        )
        .unwrap();
    assert_eq!(rep.copied.len(), 1);
    assert!(
        ws.path()
            .join("entity-auth/components/SignIn/index.tsx")
            .exists()
    );
}

#[test]
fn ui_install_multiple_validates_and_copies_many() {
    let dir = temp_dir();
    // create two components
    let root = dir.path().join("pack/ui");
    write_file(&root.join("SignIn/index.tsx"), "export const A = 1;\n");
    write_file(&root.join("UserMenu/index.tsx"), "export const B = 2;\n");
    let node = Node {
        id: "x:comp:install".into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [
            ("names".into(), serde_json::json!(["SignIn","UserMenu"]))
        ].into_iter().collect(),
        prerequisites: vec![],
        payload: NodePayload::Component { source_root: root.display().to_string() },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let rep = exec.install("x:comp:install", "multiple", Some(vec!["SignIn".into(), "UserMenu".into()]), ws.path()).unwrap();
    assert_eq!(rep.copied.len(), 2);
    assert!(ws.path().join("entity-auth/components/SignIn/index.tsx").exists());
    assert!(ws.path().join("entity-auth/components/UserMenu/index.tsx").exists());
}

#[test]
fn ui_install_invalid_names_reports_list() {
    let dir = temp_dir();
    let root = dir.path().join("pack/ui");
    write_file(&root.join("SignIn/index.tsx"), "export const A = 1;\n");
    let node = Node {
        id: "x:comp:install".into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [("names".into(), serde_json::json!(["SignIn"]))].into_iter().collect(),
        prerequisites: vec![],
        payload: NodePayload::Component { source_root: root.display().to_string() },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let err = exec.install("x:comp:install", "multiple", Some(vec!["Nope".into(), "SignIn".into()]), ws.path()).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid selection names"));
}

#[test]
fn ui_install_overwrite_copies_and_counts_files() {
    let dir = temp_dir();
    let root = dir.path().join("pack/ui");
    // component with nested files
    write_file(&root.join("AuthProvider/index.tsx"), "export const A = 1;\n");
    write_file(&root.join("AuthProvider/nested/util.ts"), "export const U = 1;\n");
    let node = Node {
        id: "x:comp:install".into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [("names".into(), serde_json::json!(["AuthProvider"]))].into_iter().collect(),
        prerequisites: vec![],
        payload: NodePayload::Component { source_root: root.display().to_string() },
    };
    let reg = Registry::new(vec![node]).unwrap();
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    // first copy
    let rep1 = exec.install("x:comp:install", "single", Some(vec!["AuthProvider".into()]), ws.path()).unwrap();
    assert_eq!(rep1.copied[0].count, 2);
    // modify dest file to ensure overwrite doesn't error
    write_file(&ws.path().join("entity-auth/components/AuthProvider/index.tsx"), "changed\n");
    // second copy should overwrite and count again
    let rep2 = exec.install("x:comp:install", "single", Some(vec!["AuthProvider".into()]), ws.path()).unwrap();
    assert_eq!(rep2.copied[0].count, 2);
}

#[test]
fn performance_smoke_targets() {
    let dir = temp_dir();
    // Docs read ≤ 10ms
    let doc_path = dir.path().join("doc.md");
    write_file(&doc_path, "hello perf");
    let doc_node = Node { id: "x:doc:perf".into(), kind: NodeKind::Doc, title: "t".into(), meta: Default::default(), prerequisites: vec![], payload: NodePayload::Doc { content_path: doc_path.display().to_string() } };
    // Component with ~10 files to copy
    let root = dir.path().join("pack/ui/SignIn");
    for i in 0..10 { write_file(&root.join(format!("f{}/a.txt", i)), "x"); }
    let comp_node = Node { id: "x:comp:perf".into(), kind: NodeKind::Component, title: "t".into(), meta: [("names".into(), serde_json::json!(["SignIn"]))].into_iter().collect(), prerequisites: vec![], payload: NodePayload::Component { source_root: dir.path().join("pack/ui").display().to_string() } };
    let reg = Registry::new(vec![doc_node, comp_node]).unwrap();
    let docs = DocsExecutor::new(&reg);
    let comps = ComponentsExecutor::new(&reg);
    let ws = temp_dir();

    let t0 = Instant::now();
    let _ = docs.read("x:doc:perf").unwrap();
    assert!(t0.elapsed().as_millis() <= 10, "docs read too slow");

    let t1 = Instant::now();
    let rep = comps.install("x:comp:perf", "single", Some(vec!["SignIn".into()]), ws.path()).unwrap();
    assert!(rep.copied[0].count >= 10);
    assert!(t1.elapsed().as_millis() <= 300, "components copy too slow");
}
