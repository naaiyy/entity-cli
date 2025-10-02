use std::fs;
use std::path::{Path, PathBuf};

use entity_core::error::{CoreError, CoreResult};
use entity_core::model::{NodeKind, NodePayload};
use entity_core::registry::Registry;
use walkdir::WalkDir;

use crate::components::CopyItemReport;
use crate::util::ensure_writable_dir;

#[derive(Debug)]
pub struct SetupReport {
    pub scaffolded: Vec<String>,
    pub copied: Vec<CopyItemReport>,
    pub notes: Vec<String>,
}

pub struct SetupExecutor<'a> {
    registry: &'a Registry,
}

impl<'a> SetupExecutor<'a> {
    pub fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }

    pub fn run(&self, node_id: &str, workspace: &Path) -> CoreResult<SetupReport> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Setup {
            return Err(CoreError::WrongKind {
                expected: "setup".into(),
                actual: format!("{:?}", node.kind),
            });
        }
        if !workspace.exists() {
            return Err(CoreError::TargetNotFound(workspace.display().to_string()));
        }

        ensure_writable_dir(workspace)
            .map_err(|_| CoreError::TargetNotWritable(workspace.display().to_string()))?;

        // Execute optional scaffold commands first
        let mut scaffolded: Vec<String> = Vec::new();
        if let NodePayload::Setup {
            commands: Some(cmds),
            ..
        } = &node.payload
        {
            for cmd in cmds {
                // naive split by whitespace; in the future support args array in payload
                let mut parts = cmd.split_whitespace();
                if let Some(bin) = parts.next() {
                    let args: Vec<&str> = parts.collect();
                    let status = std::process::Command::new(bin)
                        .args(args.clone())
                        .current_dir(workspace)
                        .status();
                    match status {
                        Ok(s) if s.success() => {
                            scaffolded.push(format!("{} {:?}", bin, args));
                        }
                        Ok(s) => {
                            return Err(CoreError::InvalidDescriptor(format!(
                                "setup command failed (exit {}): {} {:?}",
                                s.code().unwrap_or(-1),
                                bin,
                                args
                            )));
                        }
                        Err(e) => {
                            return Err(CoreError::InvalidDescriptor(format!(
                                "failed to spawn setup command: {} ({:?} {:?})",
                                e, bin, args
                            )));
                        }
                    }
                }
            }
        }

        // Copy template tree into /entity-auth
        let mut report = SetupReport {
            scaffolded,
            copied: Vec::new(),
            notes: vec!["Overwrite-on-write by default".into()],
        };
        let (template_root, _) = match &node.payload {
            NodePayload::Setup { template_root, .. } => (PathBuf::from(template_root), true),
            _ => unreachable!(),
        };
        // Copy into workspace/entity-auth. TemplateRoot should contain the contents
        // that belong directly under entity-auth to avoid double nesting.
        let to_root = workspace.join("entity-auth");
        fs::create_dir_all(&to_root)?;
        let from_root = template_root;
        let mut files_copied = 0usize;
        for entry in WalkDir::new(&from_root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() {
                let rel = path.strip_prefix(&from_root).unwrap();
                let to_path = to_root.join(rel);
                if let Some(parent) = to_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &to_path)?;
                files_copied += 1;
            }
        }
        report.copied.push(CopyItemReport {
            from: from_root.display().to_string(),
            to: to_root.display().to_string(),
            count: files_copied,
        });
        Ok(report)
    }
}
