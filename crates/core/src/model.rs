use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    pub meta: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    pub payload: NodePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Doc,
    Component,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NodePayload {
    Doc {
        #[serde(rename = "contentPath")]
        content_path: String,
    },
    Component {
        #[serde(rename = "sourceRoot")]
        source_root: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub key: String,
    pub schema: serde_json::Value,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPackage {
    pub nodes: Vec<Node>,
    pub command_shapes: CommandShapes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandShapes {
    pub docs: DocsCommandShape,
    pub ui: UiCommandShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsCommandShape {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCommandShape {
    pub template: String,
}
