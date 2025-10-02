use std::fs;
use std::path::PathBuf;

use entity_core::error::CoreError;
use entity_core::loader::load_nodes_from_file;
use entity_core::model::{
    BridgeCommandShape, CommandShapes, DocsCommandShape, GraphPackage, InitCommandShape, Node,
    Platforms, Semantics, SetupCommandShape, UiCommandShape,
};
use entity_core::registry::Registry;
use tracing::info;

#[derive(Clone)]
pub struct Engine {
    registry: Registry,
}

impl Engine {
    pub fn bootstrap(
        packs_root: PathBuf,
        product: Option<&str>,
    ) -> anyhow::Result<(Self, GraphPackage)> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut loaded = 0usize;
        // Validate packs root
        if !packs_root.exists() || !packs_root.is_dir() {
            return Err(CoreError::PacksNotFound(packs_root.display().to_string()).into());
        }
        // Scan packs/*/{docs,components,setup}/nodes.json or specific product if provided
        let pack_dirs: Vec<PathBuf> = if let Some(prod) = product {
            // Only scan the specific product directory
            vec![packs_root.join(prod)]
        } else {
            // Scan all subdirectories
            fs::read_dir(&packs_root)
                .map_err(|_| CoreError::PacksNotFound(packs_root.display().to_string()))?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect()
        };

        for pack in pack_dirs {
            let docs_nodes = pack.join("docs").join("nodes.json");
            let comp_nodes = pack.join("components").join("nodes.json");
            let setup_nodes = pack.join("setup").join("nodes.json");
            let bridge_nodes = pack.join("bridge").join("nodes.json");
            if docs_nodes.exists() {
                nodes.extend(load_nodes_from_file(&docs_nodes)?);
                loaded += 1;
            }
            if comp_nodes.exists() {
                nodes.extend(load_nodes_from_file(&comp_nodes)?);
                loaded += 1;
            }
            if setup_nodes.exists() {
                nodes.extend(load_nodes_from_file(&setup_nodes)?);
                loaded += 1;
            }
            if bridge_nodes.exists() {
                nodes.extend(load_nodes_from_file(&bridge_nodes)?);
                loaded += 1;
            }
        }
        info!(packs = %packs_root.display(), product = ?product, loaded_sets = loaded, nodes_count = nodes.len(), "loaded packs nodes");

        let registry = Registry::new(nodes.clone())?;
        // Determine executable from environment (shim sets ENTITY_CLI_EXECUTABLE), default to entity-cli
        let exe =
            std::env::var("ENTITY_CLI_EXECUTABLE").unwrap_or_else(|_| "entity-cli".to_string());

        let command_shapes = CommandShapes {
            init: InitCommandShape {
                template: format!("{} init <product>", exe),
            },
            docs: DocsCommandShape {
                template: format!("{} docs read <product> --node <id>", exe),
            },
            ui: UiCommandShape {
                template: format!(
                    "{} ui install <product> --mode <single|multiple|all> [--names <Name...>]",
                    exe
                ),
            },
            setup: SetupCommandShape {
                template: format!(
                    "{} setup run <product> --node <id> [--workspace <path>]",
                    exe
                ),
            },
            bridge: Some(BridgeCommandShape {
                scaffold_template: format!(
                    "{} bridge scaffold <product> --node <id> [--workspace <path>]",
                    exe
                ),
                start_template: format!(
                    "{} bridge start <product> --node <id> [--workspace <path>]",
                    exe
                ),
                status_template: format!(
                    "{} bridge status <product> --node <id> [--workspace <path>]",
                    exe
                ),
                stop_template: format!(
                    "{} bridge stop <product> --node <id> [--workspace <path>]",
                    exe
                ),
            }),
        };
        let graph = GraphPackage {
            nodes,
            command_shapes,
            executable: exe,
            semantics: Semantics {
                writes_to: "cwd".to_string(),
                overwrite_on_write: true,
                platforms: Platforms {
                    os: vec!["darwin".into()],
                    arch: vec!["arm64".into()],
                },
            },
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
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
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
        write_file(
            &docs_dir.join("nodes.json"),
            &serde_json::to_string(&docs_nodes).unwrap(),
        );
        write_file(&comps_dir.join("nodes.json"), "[]");

        let (_engine, graph) = Engine::bootstrap(packs.path().to_path_buf(), None).unwrap();
        assert!(!graph.nodes.is_empty());
        assert!(
            graph
                .command_shapes
                .docs
                .template
                .contains("entity-cli docs read")
        );
        assert!(
            graph
                .command_shapes
                .ui
                .template
                .contains("entity-cli ui install")
        );
    }
}
