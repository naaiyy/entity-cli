use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{CoreError, CoreResult};
use crate::model::{BridgeSpawnDescriptor, Node, NodePayload};

pub fn load_nodes_from_file(path: &Path) -> CoreResult<Vec<Node>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let raw_nodes: Vec<Value> = serde_json::from_reader(reader)?;
    let base_dir = path.parent().unwrap_or(Path::new("."));
    let mut nodes: Vec<Node> = Vec::new();
    for value in raw_nodes.into_iter() {
        let mut node = Node::try_from(value)?;
        // Rewrite payload paths to be relative to the nodes.json directory if they are relative
        match &mut node.payload {
            NodePayload::Doc { content_path } => {
                let p = PathBuf::from(&*content_path);
                if p.is_relative() {
                    let abs = base_dir.join(&p);
                    *content_path = abs.to_string_lossy().to_string();
                }
            }
            NodePayload::Component { source_root } => {
                let p = PathBuf::from(&*source_root);
                if p.is_relative() {
                    let abs = base_dir.join(&p);
                    *source_root = abs.to_string_lossy().to_string();
                }
            }
            NodePayload::Setup { template_root, .. } => {
                let p = PathBuf::from(&*template_root);
                if p.is_relative() {
                    let abs = base_dir.join(&p);
                    *template_root = abs.to_string_lossy().to_string();
                }
            }
            NodePayload::Bridge {
                template_root,
                runner,
                config_template,
                spawn,
                logs_path,
                heartbeat_interval_ms: _,
            } => {
                if let Some(root) = template_root {
                    let p = Path::new(root);
                    if p.is_relative() {
                        let abs = base_dir.join(p);
                        *template_root = Some(abs.to_string_lossy().to_string());
                    }
                }
                if let Some(path) = runner {
                    let p = Path::new(path);
                    if p.is_relative() {
                        let abs = base_dir.join(p);
                        *runner = Some(abs.to_string_lossy().to_string());
                    }
                }
                if let Some(path) = config_template {
                    let p = Path::new(path);
                    if p.is_relative() {
                        let abs = base_dir.join(p);
                        *config_template = Some(abs.to_string_lossy().to_string());
                    }
                }
                if let Some(descriptor) = spawn {
                    normalize_spawn_paths(descriptor, base_dir);
                }
                if let Some(path) = logs_path {
                    let p = Path::new(path);
                    if p.is_relative() {
                        let abs = base_dir.join(p);
                        *logs_path = Some(abs.to_string_lossy().to_string());
                    }
                }
            }
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn normalize_spawn_paths(descriptor: &mut BridgeSpawnDescriptor, base_dir: &Path) {
    let entry_path = Path::new(&descriptor.entry);
    if entry_path.is_relative() {
        let abs = base_dir.join(entry_path);
        descriptor.entry = abs.to_string_lossy().to_string();
    }
}

impl TryFrom<Value> for Node {
    type Error = CoreError;

    fn try_from(value: Value) -> CoreResult<Self> {
        // Allow both snake_case and camelCase payloads by remapping keys for our struct
        let node: Node = serde_json::from_value(value.clone()).map_err(|err| {
            CoreError::InvalidDescriptor(format!("invalid node json: {err}: {value}"))
        })?;

        match &node.payload {
            NodePayload::Doc { content_path } => {
                if content_path.is_empty() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "doc node {} missing content_path",
                        node.id
                    )));
                }
            }
            NodePayload::Component { source_root } => {
                if source_root.is_empty() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "component node {} missing source_root",
                        node.id
                    )));
                }
            }
            NodePayload::Setup { template_root, .. } => {
                if template_root.is_empty() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "setup node {} missing template_root",
                        node.id
                    )));
                }
            }
            NodePayload::Bridge {
                template_root,
                runner,
                spawn,
                ..
            } => {
                if template_root.is_none() && runner.is_none() && spawn.is_none() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "bridge node {} must specify templateRoot, runner, or spawn entry",
                        node.id
                    )));
                }
            }
        }

        Ok(node)
    }
}
