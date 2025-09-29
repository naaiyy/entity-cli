use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use entity_core::error::{CoreError, CoreResult};
use entity_core::model::{NodeKind, NodePayload, Prerequisite};
use entity_core::registry::Registry;
use tracing::info;
use walkdir::WalkDir;
#[cfg(test)]
mod tests;

pub struct DocsExecutor<'a> {
    registry: &'a Registry,
}

impl<'a> DocsExecutor<'a> {
    pub fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }

    pub fn read(&self, node_id: &str) -> CoreResult<String> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Doc {
            return Err(CoreError::WrongKind {
                expected: "doc".into(),
                actual: format!("{:?}", node.kind),
            });
        }
        let path = match &node.payload {
            NodePayload::Doc { content_path } => content_path,
            _ => unreachable!(),
        };
        let mut file = fs::File::open(path).map_err(|_| CoreError::MissingSource(path.clone()))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(buf)
    }
}

pub struct ComponentsExecutor<'a> {
    registry: &'a Registry,
}

#[derive(Debug)]
pub struct CopyItemReport {
    pub from: String,
    pub to: String,
    pub count: usize,
}

#[derive(Debug)]
pub struct CopyReport {
    pub copied: Vec<CopyItemReport>,
    pub notes: Vec<String>,
}

impl<'a> ComponentsExecutor<'a> {
    pub fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }

    pub fn install(
        &self,
        node_id: &str,
        mode: &str,
        names: Option<Vec<String>>,
        write_root: &Path,
    ) -> CoreResult<CopyReport> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Component {
            return Err(CoreError::WrongKind {
                expected: "component".into(),
                actual: format!("{:?}", node.kind),
            });
        }
        if !write_root.exists() {
            return Err(CoreError::TargetNotFound(
                write_root.display().to_string(),
            ));
        }

        // basic writability check: attempt to create and remove a temp directory under workspace
        let probe_dir = write_root.join(".entitycli_write_probe");
        if let Err(_e) = fs::create_dir_all(&probe_dir) {
            return Err(CoreError::TargetNotWritable(
                write_root.display().to_string(),
            ));
        }
        // best-effort cleanup: try writing a small file too
        let probe_file = probe_dir.join(".probe");
        if let Err(_e) = fs::File::create(&probe_file) {
            let _ = fs::remove_dir_all(&probe_dir);
            return Err(CoreError::TargetNotWritable(
                write_root.display().to_string(),
            ));
        }
        let _ = fs::remove_file(&probe_file);
        let _ = fs::remove_dir_all(&probe_dir);

        // Generic prerequisite enforcement for any future flat keys beyond mode/names
        let mut missing_generic: Vec<String> = Vec::new();
        for p in &node.prerequisites {
            if p.optional {
                continue;
            }
            if p.key != "selection.mode" && p.key != "selection.names" {
                missing_generic.push(p.key.clone());
            }
        }
        if !missing_generic.is_empty() {
            return Err(CoreError::MissingSelections(missing_generic));
        }

        // validate mode and names against prerequisites schema and node meta
        let (allowed_modes_from_schema, allowed_names_from_schema, names_optional) =
            parse_prereqs(&node.prerequisites);
        if allowed_modes_from_schema
            .as_ref()
            .is_some_and(|modes| !modes.contains(&mode.to_string()))
        {
            let allowed = allowed_modes_from_schema
                .as_ref()
                .map(|m| m.join("|"))
                .unwrap_or_default();
            return Err(CoreError::InvalidSelection(format!(
                "mode must be one of {}",
                allowed
            )));
        }

        if names.is_none() && !names_optional {
            // if prereqs mark names required, enforce presence except for mode=all which is handled later
            if mode == "single" || mode == "multiple" {
                return Err(CoreError::MissingSelections(vec!["selection.names".into()]));
            }
        }

        let all_names: Vec<String> = node
            .meta
            .get("names")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let allowed_names: Vec<String> = if let Some(schema_names) = allowed_names_from_schema {
            schema_names
        } else {
            all_names.clone()
        };

        let selected: Vec<String> = match mode {
            "single" => {
                let list = names.unwrap_or_default();
                if list.len() != 1 {
                    return Err(CoreError::MissingSelections(vec!["selection.names".into()]));
                }
                let n = list[0].clone();
                if !allowed_names.contains(&n) {
                    return Err(CoreError::InvalidNames(vec![n]));
                }
                vec![n]
            }
            "multiple" => {
                let list = names.unwrap_or_default();
                if list.is_empty() {
                    return Err(CoreError::MissingSelections(vec!["selection.names".into()]));
                }
                let invalid: Vec<String> = list
                    .iter()
                    .filter(|n| !allowed_names.contains(n))
                    .cloned()
                    .collect();
                if !invalid.is_empty() {
                    return Err(CoreError::InvalidNames(invalid));
                }
                list
            }
            "all" => {
                if names.is_some() {
                    return Err(CoreError::InvalidSelection(
                        "names must be omitted for mode all".into(),
                    ));
                }
                all_names
            }
            _ => {
                return Err(CoreError::InvalidSelection(
                    "mode must be one of single|multiple|all".into(),
                ));
            }
        };

        let mut report = CopyReport {
            copied: Vec::new(),
            notes: vec!["Overwrite-on-write by default".into()],
        };
        let to_root = write_root.join("entity-auth").join("components");
        fs::create_dir_all(&to_root)?;

        for name in selected {
            let (_source_root, src_dir) = match &node.payload {
                NodePayload::Component { source_root } => {
                    (source_root.clone(), PathBuf::from(source_root).join(&name))
                }
                _ => unreachable!(),
            };
            if !src_dir.exists() {
                return Err(CoreError::MissingSource(src_dir.display().to_string()));
            }
            let dest_dir = to_root.join(&name);
            fs::create_dir_all(&dest_dir)?;
            let mut files_copied = 0usize;
            for entry in WalkDir::new(&src_dir).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    let rel = path.strip_prefix(&src_dir).unwrap();
                    let to_path = dest_dir.join(rel);
                    if let Some(parent) = to_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(path, &to_path)?;
                    files_copied += 1;
                }
            }
            report.copied.push(CopyItemReport {
                from: src_dir.display().to_string(),
                to: dest_dir.display().to_string(),
                count: files_copied,
            });
            info!(from = %src_dir.display(), to = %dest_dir.display(), count = files_copied, "component copied");
        }
        Ok(report)
    }
}

fn parse_prereqs(prereqs: &Vec<Prerequisite>) -> (Option<Vec<String>>, Option<Vec<String>>, bool) {
    let mut modes: Option<Vec<String>> = None;
    let mut names: Option<Vec<String>> = None;
    let mut names_optional = true;
    for p in prereqs {
        if p.key == "selection.mode" {
            if let Some(arr) = p.schema.get("enum").and_then(|v| v.as_array()) {
                let vals = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                if !vals.is_empty() {
                    modes = Some(vals);
                }
            }
        } else if p.key == "selection.names" {
            names_optional = p.optional;
            if let Some(arr) = p
                .schema
                .get("items")
                .and_then(|it| it.get("enum"))
                .and_then(|v| v.as_array())
            {
                let vals = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                if !vals.is_empty() {
                    names = Some(vals);
                }
            }
        }
    }
    (modes, names, names_optional)
}
