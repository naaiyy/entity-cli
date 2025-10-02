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
    Setup,
    Bridge,
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
    Setup {
        #[serde(rename = "templateRoot")]
        template_root: String,
        #[serde(default)]
        commands: Option<Vec<String>>,
    },
    Bridge {
        #[serde(rename = "templateRoot")]
        template_root: Option<String>,
        #[serde(rename = "runner", default)]
        runner: Option<String>,
        #[serde(rename = "configTemplate", default)]
        config_template: Option<String>,
        #[serde(rename = "spawn", default)]
        spawn: Option<BridgeSpawnDescriptor>,
        #[serde(default, rename = "logsPath")]
        logs_path: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSpawnDescriptor {
    /// Node runner entry file relative to packs root
    pub entry: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<BridgeEnvVar>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEnvVar {
    pub key: String,
    pub default: Option<String>,
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
    /// Name of the executable to use in examples/materialized templates
    pub executable: String,
    /// High-level semantics to avoid guessing in agents
    pub semantics: Semantics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandShapes {
    pub init: InitCommandShape,
    pub docs: DocsCommandShape,
    pub ui: UiCommandShape,
    pub setup: SetupCommandShape,
    pub bridge: Option<BridgeCommandShape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsCommandShape {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCommandShape {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitCommandShape {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupCommandShape {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCommandShape {
    pub scaffold_template: String,
    pub start_template: String,
    pub status_template: String,
    pub stop_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Semantics {
    /// Where write operations target by default
    pub writes_to: String, // e.g., "cwd"
    /// Whether component installs overwrite existing files
    pub overwrite_on_write: bool,
    /// Supported platforms (informational)
    pub platforms: Platforms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platforms {
    pub os: Vec<String>,
    pub arch: Vec<String>,
}
