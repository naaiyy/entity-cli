use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::error::{CoreError, CoreResult};
use crate::model::{Node, NodeKind, NodePayload};

#[derive(Debug, Default, Clone)]
pub struct Registry {
    nodes: IndexMap<String, Node>,
    docs: Vec<String>,
    components: Vec<String>,
    by_tag: HashMap<String, Vec<String>>,     // tag -> node ids
    by_section: HashMap<String, Vec<String>>, // section -> node ids
    bridges: Vec<String>,
}

impl Registry {
    pub fn new(nodes: Vec<Node>) -> CoreResult<Self> {
        let mut registry = Registry::default();
        for node in nodes {
            registry.insert(node)?;
        }
        Ok(registry)
    }

    pub fn insert(&mut self, node: Node) -> CoreResult<()> {
        if self.nodes.contains_key(&node.id) {
            return Err(CoreError::InvalidDescriptor(format!(
                "duplicate node id {}",
                node.id
            )));
        }

        // Validate prerequisites keys uniqueness
        let mut keys = HashSet::new();
        for prereq in &node.prerequisites {
            if !keys.insert(prereq.key.clone()) {
                return Err(CoreError::InvalidDescriptor(format!(
                    "duplicate prerequisite key {} on node {}",
                    prereq.key, node.id
                )));
            }
        }

        // Validate payload paths exist
        match &node.payload {
            NodePayload::Doc { content_path } => {
                if !std::path::Path::new(content_path).exists() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "doc content path not found for node {}: {}",
                        node.id, content_path
                    )));
                }
            }
            NodePayload::Component { source_root } => {
                if !std::path::Path::new(source_root).exists() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "component source root not found for node {}: {}",
                        node.id, source_root
                    )));
                }
            }
            NodePayload::Setup { template_root, .. } => {
                if !std::path::Path::new(template_root).exists() {
                    return Err(CoreError::InvalidDescriptor(format!(
                        "setup template root not found for node {}: {}",
                        node.id, template_root
                    )));
                }
            }
            NodePayload::Bridge {
                template_root,
                runner,
                config_template,
                spawn,
                logs_path,
                heartbeat_interval_ms,
            } => {
                if let Some(root) = template_root {
                    if !std::path::Path::new(root).exists() {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge template root not found for node {}: {}",
                            node.id, root
                        )));
                    }
                }
                if let Some(path) = runner {
                    if !std::path::Path::new(path).exists() {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge runner not found for node {}: {}",
                            node.id, path
                        )));
                    }
                }
                if let Some(path) = config_template {
                    if !std::path::Path::new(path).exists() {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge config template not found for node {}: {}",
                            node.id, path
                        )));
                    }
                }
                if let Some(descriptor) = spawn {
                    if !std::path::Path::new(&descriptor.entry).exists() {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge spawn entry not found for node {}: {}",
                            node.id, descriptor.entry
                        )));
                    }
                }
                if let Some(path) = logs_path {
                    let parent = std::path::Path::new(path)
                        .parent()
                        .map(|p| p.exists())
                        .unwrap_or(true);
                    if !parent {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge logs path parent missing for node {}: {}",
                            node.id, path
                        )));
                    }
                }
                if let Some(interval) = heartbeat_interval_ms {
                    if *interval == 0 {
                        return Err(CoreError::InvalidDescriptor(format!(
                            "bridge heartbeat interval must be > 0 for node {}",
                            node.id
                        )));
                    }
                }
            }
        }

        // Index by tag and section if present
        if let Some(tags) = node.meta.get("tags").and_then(|v| v.as_array()) {
            for t in tags.iter().filter_map(|v| v.as_str()) {
                self.by_tag
                    .entry(t.to_string())
                    .or_default()
                    .push(node.id.clone());
            }
        }
        if let Some(section) = node.meta.get("section").and_then(|v| v.as_str()) {
            self.by_section
                .entry(section.to_string())
                .or_default()
                .push(node.id.clone());
        }

        match node.kind {
            NodeKind::Doc => self.docs.push(node.id.clone()),
            NodeKind::Component => self.components.push(node.id.clone()),
            NodeKind::Setup => { /* currently not indexed; may add later */ }
            NodeKind::Bridge => self.bridges.push(node.id.clone()),
        }

        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn get(&self, id: &str) -> CoreResult<&Node> {
        self.nodes
            .get(id)
            .ok_or_else(|| CoreError::UnknownNode(id.to_string()))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn nodes_by_kind(&self) -> (&[String], &[String], &[String]) {
        (&self.docs, &self.components, &self.bridges)
    }

    pub fn into_map(self) -> HashMap<String, Node> {
        self.nodes.into_iter().collect()
    }

    pub fn nodes_by_tag(&self, tag: &str) -> Option<&[String]> {
        self.by_tag.get(tag).map(|v| v.as_slice())
    }

    pub fn nodes_by_section(&self, section: &str) -> Option<&[String]> {
        self.by_section.get(section).map(|v| v.as_slice())
    }
}
