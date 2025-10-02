use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use entity_core::error::{CoreError, CoreResult};
use entity_core::model::{BridgeEnvVar, NodeKind, NodePayload};
use entity_core::registry::Registry;
use walkdir::WalkDir;

use crate::components::CopyItemReport;

#[derive(Debug)]
pub struct BridgeScaffoldReport {
    pub copied: Vec<CopyItemReport>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BridgeProcessInfo {
    pub entry: String,
    pub args: Vec<String>,
    pub env: Vec<(String, Option<String>)>,
    pub cwd: Option<String>,
    pub config_path: Option<String>,
    pub logs_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeProcessState {
    pub id: String,
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub workspace: String,
    #[serde(rename = "packsRoot")]
    pub packs_root: String,
    #[serde(rename = "process")]
    pub process: BridgeProcessStateProcess,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
    pub status: String,
    #[serde(
        rename = "statusMessage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub status_message: Option<String>,
    #[serde(rename = "logsPath", default, skip_serializing_if = "Option::is_none")]
    pub logs_path: Option<String>,
    #[serde(
        rename = "heartbeatAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub heartbeat_at: Option<u64>,
    #[serde(rename = "exitCode", default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeProcessStateProcess {
    pub entry: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    #[serde(rename = "configPath")]
    pub config_path: Option<String>,
}

#[derive(Debug)]
pub struct BridgeStopResult {
    pub pid: Option<i32>,
    pub status: String,
    pub state_id: String,
}

pub struct BridgeExecutor<'a> {
    registry: &'a Registry,
}

impl<'a> BridgeExecutor<'a> {
    pub fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }

    pub fn scaffold(&self, node_id: &str, workspace: &Path) -> CoreResult<BridgeScaffoldReport> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Bridge {
            return Err(CoreError::WrongKind {
                expected: "bridge".into(),
                actual: format!("{:?}", node.kind),
            });
        }
        if !workspace.exists() {
            return Err(CoreError::TargetNotFound(workspace.display().to_string()));
        }

        let mut report = BridgeScaffoldReport {
            copied: Vec::new(),
            notes: vec![],
        };

        let NodePayload::Bridge { template_root, .. } = &node.payload else {
            unreachable!();
        };

        if let Some(root) = template_root {
            let template = Path::new(root);
            if !template.exists() {
                return Err(CoreError::MissingSource(template.display().to_string()));
            }
            let to_root = workspace.join("entity-auth");
            fs::create_dir_all(&to_root)?;
            let mut files_copied = 0usize;
            for entry in WalkDir::new(template).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    let rel = path.strip_prefix(template).unwrap();
                    let to_path = to_root.join(rel);
                    if let Some(parent) = to_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(path, &to_path)?;
                    files_copied += 1;
                }
            }
            report.copied.push(CopyItemReport {
                from: template.display().to_string(),
                to: to_root.display().to_string(),
                count: files_copied,
            });
            report
                .notes
                .push("Overwrite-on-write by default".to_string());
        }

        Ok(report)
    }

    pub fn spawn_descriptor(&self, node_id: &str) -> CoreResult<BridgeProcessInfo> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Bridge {
            return Err(CoreError::WrongKind {
                expected: "bridge".into(),
                actual: format!("{:?}", node.kind),
            });
        }

        let NodePayload::Bridge {
            runner,
            spawn,
            config_template,
            logs_path,
            ..
        } = &node.payload
        else {
            unreachable!();
        };

        if let Some(runner_path) = runner.clone() {
            let cwd = Path::new(&runner_path)
                .parent()
                .map(|p| p.display().to_string());
            return Ok(BridgeProcessInfo {
                entry: runner_path,
                args: Vec::new(),
                env: Vec::new(),
                cwd,
                config_path: config_template.clone(),
                logs_path: logs_path.clone(),
            });
        }

        if let Some(descriptor) = spawn.clone() {
            return Ok(BridgeProcessInfo {
                entry: descriptor.entry,
                args: descriptor.args,
                env: descriptor
                    .env
                    .into_iter()
                    .map(|BridgeEnvVar { key, default }| (key, default))
                    .collect(),
                cwd: descriptor.cwd,
                config_path: config_template.clone(),
                logs_path: logs_path.clone(),
            });
        }

        Err(CoreError::InvalidDescriptor(format!(
            "bridge node {} missing runner or spawn descriptor",
            node.id
        )))
    }

    pub fn state_dir(workspace: &Path) -> PathBuf {
        workspace.join(".entitycli").join("bridge").join("state")
    }

    pub fn state_file(workspace: &Path, node_id: &str) -> PathBuf {
        Self::state_dir(workspace)
            .join(Self::safe_node_filename(node_id))
            .with_extension("json")
    }

    fn safe_node_filename(node_id: &str) -> String {
        node_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn persist_state(
        &self,
        node_id: &str,
        process: BridgeProcessInfo,
        workspace: &Path,
        packs: PathBuf,
        state_id: &str,
    ) -> CoreResult<()> {
        let dir = Self::state_dir(workspace);
        fs::create_dir_all(&dir)?;
        let state = BridgeProcessState {
            id: state_id.to_string(),
            node_id: node_id.to_string(),
            workspace: workspace.display().to_string(),
            packs_root: packs.display().to_string(),
            process: BridgeProcessStateProcess {
                entry: process.entry,
                args: process.args,
                env: process
                    .env
                    .into_iter()
                    .map(|(key, default)| (key, default.unwrap_or_default()))
                    .collect(),
                cwd: process.cwd,
                config_path: process.config_path,
            },
            pid: None,
            status: "pending".into(),
            status_message: None,
            logs_path: process.logs_path,
            heartbeat_at: None,
            exit_code: None,
            updated_at: Self::now_ms(),
        };
        let file = Self::state_file(workspace, node_id);
        fs::write(file, serde_json::to_string_pretty(&state)?)?;
        Ok(())
    }

    pub fn read_state(workspace: &Path, node_id: &str) -> CoreResult<Option<BridgeProcessState>> {
        let file = Self::state_file(workspace, node_id);
        if !file.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&file)?;
        let mut state: BridgeProcessState = serde_json::from_str(&content)?;
        state.updated_at = Self::now_ms();
        Ok(Some(state))
    }

    pub fn update_state(
        workspace: &Path,
        node_id: &str,
        mutation: impl FnOnce(&mut BridgeProcessState),
    ) -> CoreResult<Option<BridgeProcessState>> {
        let file = Self::state_file(workspace, node_id);
        if !file.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&file)?;
        let mut state: BridgeProcessState = serde_json::from_str(&content)?;
        mutation(&mut state);
        state.updated_at = Self::now_ms();
        fs::write(&file, serde_json::to_string_pretty(&state)?)?;
        Ok(Some(state))
    }

    pub fn remove_state(workspace: &Path, node_id: &str) -> CoreResult<()> {
        let file = Self::state_file(workspace, node_id);
        if file.exists() {
            fs::remove_file(file)?;
        }
        Ok(())
    }

    pub fn attach_pid(
        workspace: &Path,
        node_id: &str,
        pid: i32,
        status: Option<&str>,
        status_message: Option<&str>,
    ) -> CoreResult<Option<BridgeProcessState>> {
        Self::update_state(workspace, node_id, |state| {
            state.pid = Some(pid);
            if let Some(status) = status {
                state.status = status.to_string();
            } else {
                state.status = "running".into();
            }
            state.status_message = status_message.map(|s| s.to_string());
            state.heartbeat_at = Some(Self::now_ms());
            state.exit_code = None;
        })
    }

    pub fn heartbeat(
        workspace: &Path,
        node_id: &str,
        status: Option<&str>,
        status_message: Option<&str>,
    ) -> CoreResult<Option<BridgeProcessState>> {
        Self::update_state(workspace, node_id, |state| {
            if let Some(status) = status {
                state.status = status.to_string();
            }
            if status_message.is_some() {
                state.status_message = status_message.map(|s| s.to_string());
            }
            state.heartbeat_at = Some(Self::now_ms());
        })
    }

    pub fn complete(
        workspace: &Path,
        node_id: &str,
        exit_code: Option<i32>,
        status: Option<&str>,
        status_message: Option<&str>,
    ) -> CoreResult<Option<BridgeProcessState>> {
        Self::update_state(workspace, node_id, |state| {
            state.pid = None;
            state.exit_code = exit_code;
            if let Some(status) = status {
                state.status = status.to_string();
            } else {
                state.status = "exited".into();
            }
            state.status_message = status_message.map(|s| s.to_string());
            state.heartbeat_at = Some(Self::now_ms());
        })
    }

    pub fn stop(workspace: &Path, node_id: &str) -> CoreResult<Option<BridgeStopResult>> {
        let Some(state) = Self::read_state(workspace, node_id)? else {
            return Ok(None);
        };
        let signal_pid = state.pid;
        if let Some(pid_value) = signal_pid {
            #[cfg(unix)]
            {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid_value),
                    nix::sys::signal::Signal::SIGINT,
                );
            }
            #[cfg(not(unix))]
            {
                let _ = pid_value;
            }
        }
        // Remove state file once stop has been requested.
        Self::remove_state(workspace, node_id)?;
        Ok(Some(BridgeStopResult {
            pid: signal_pid,
            status: "stopped".into(),
            state_id: state.id,
        }))
    }
}
