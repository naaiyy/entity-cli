use std::fs;

use entity_core::model::{Node, NodeKind, NodePayload};
use entity_core::registry::Registry;

pub(crate) fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

pub(crate) fn write_file(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub(crate) fn bridge_registry(node: Node) -> Registry {
    Registry::new(vec![node]).unwrap()
}

pub(crate) fn component_registry_fixture(names: &[&str]) -> (Registry, tempfile::TempDir) {
    let dir = temp_dir();
    let root = dir.path().join("pack/components");
    for name in names {
        write_file(
            &root.join(name).join("index.tsx"),
            "export const Component = () => null;\n",
        );
    }
    let node = component_node("x:comp:install", &root, names);
    (Registry::new(vec![node]).unwrap(), dir)
}

pub(crate) fn doc_node(id: &str, path: &std::path::Path) -> Node {
    Node {
        id: id.into(),
        kind: NodeKind::Doc,
        title: "t".into(),
        meta: Default::default(),
        prerequisites: vec![],
        payload: NodePayload::Doc {
            content_path: path.display().to_string(),
        },
    }
}

pub(crate) fn component_node(id: &str, source_root: &std::path::Path, names: &[&str]) -> Node {
    Node {
        id: id.into(),
        kind: NodeKind::Component,
        title: "t".into(),
        meta: [("names".into(), serde_json::json!(names))]
            .into_iter()
            .collect(),
        prerequisites: vec![],
        payload: NodePayload::Component {
            source_root: source_root.display().to_string(),
        },
    }
}

pub(crate) fn bridge_node(
    id: &str,
    template_root: Option<&std::path::Path>,
    runner: Option<&std::path::Path>,
    config_template: Option<&std::path::Path>,
    logs_path: Option<&std::path::Path>,
) -> Node {
    Node {
        id: id.into(),
        kind: NodeKind::Bridge,
        title: "bridge".into(),
        meta: Default::default(),
        prerequisites: vec![],
        payload: NodePayload::Bridge {
            template_root: template_root.map(|p| p.display().to_string()),
            runner: runner.map(|p| p.display().to_string()),
            config_template: config_template.map(|p| p.display().to_string()),
            spawn: None,
            logs_path: logs_path.map(|p| p.display().to_string()),
            heartbeat_interval_ms: Some(5_000),
        },
    }
}
