use std::fs;
use std::path::PathBuf;

use entity_core::error::CoreError;
use entity_core::loader::load_nodes_from_file;
use entity_core::model::{CommandShapes, DocsCommandShape, GraphPackage, Node, UiCommandShape};
use entity_core::registry::Registry;
use tracing::info;

#[derive(Clone)]
pub struct Engine {
    registry: Registry,
}

impl Engine {
    pub fn bootstrap(packs_root: PathBuf) -> anyhow::Result<(Self, GraphPackage)> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut loaded = 0usize;
        // Validate packs root
        if !packs_root.exists() || !packs_root.is_dir() {
            return Err(CoreError::PacksNotFound(packs_root.display().to_string()).into());
        }
        // Scan packs/*/{docs,components}/nodes.json
        for entry in fs::read_dir(&packs_root)
            .map_err(|_| CoreError::PacksNotFound(packs_root.display().to_string()))?
        {
            let pack = entry?.path();
            let docs_nodes = pack.join("docs").join("nodes.json");
            let comp_nodes = pack.join("components").join("nodes.json");
            if docs_nodes.exists() {
                nodes.extend(load_nodes_from_file(&docs_nodes)?);
                loaded += 1;
            }
            if comp_nodes.exists() {
                nodes.extend(load_nodes_from_file(&comp_nodes)?);
                loaded += 1;
            }
        }
        info!(packs = %packs_root.display(), loaded_sets = loaded, nodes_count = nodes.len(), "loaded packs nodes");

        let registry = Registry::new(nodes.clone())?;
        let command_shapes = CommandShapes {
            docs: DocsCommandShape { template: "entity-cli docs read <product> --node <id>".to_string() },
            ui: UiCommandShape { template: "entity-cli ui install <product> --mode <single|multiple|all> [--names <Name...>]".to_string() },
        };
        let graph = GraphPackage {
            nodes,
            command_shapes,
        };
        Ok((Self { registry }, graph))
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(path: &std::path::Path, content: &str) {
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(path, content);
    }

    #[test]
    fn bootstrap_emits_graph_with_command_shapes() {
        let packs = TempDir::new().unwrap();
        let pack_root = packs.path().join("entity-auth");
        let docs_dir = pack_root.join("docs");
        let comps_dir = pack_root.join("components");
        let content_dir = docs_dir.join("content");
        let doc_path = content_dir.join("a.md");
        write_file(&doc_path, "hello");
        let docs_nodes = serde_json::json!([
            {
                "id": "x:docs:a",
                "kind": "doc",
                "title": "A",
                "meta": {},
                "prerequisites": [],
                "payload": { "contentPath": doc_path.to_string_lossy() }
            }
        ]);
        write_file(&docs_dir.join("nodes.json"), &serde_json::to_string(&docs_nodes).unwrap());
        write_file(&comps_dir.join("nodes.json"), "[]");

        let (_engine, graph) = Engine::bootstrap(packs.path().to_path_buf()).unwrap();
        assert!(!graph.nodes.is_empty());
        assert!(graph.command_shapes.docs.template.contains("entity-cli docs read"));
        assert!(graph.command_shapes.ui.template.contains("entity-cli ui install"));
    }
}
