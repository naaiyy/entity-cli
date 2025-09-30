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

fn to_kebab(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_is_sep = false;
    for (i, ch) in input.chars().enumerate() {
        if ch == '_' || ch == '-' || ch == ' ' {
            if !prev_is_sep && !out.is_empty() {
                out.push('-');
            }
            prev_is_sep = true;
            continue;
        }
        let is_upper = ch.is_ascii_uppercase();
        if i > 0 && is_upper && !prev_is_sep {
            out.push('-');
        }
        out.push(ch.to_ascii_lowercase());
        prev_is_sep = false;
    }
    out
}

fn to_snake(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_is_sep = false;
    for (i, ch) in input.chars().enumerate() {
        if ch == '-' || ch == '_' || ch == ' ' {
            if !prev_is_sep && !out.is_empty() {
                out.push('_');
            }
            prev_is_sep = true;
            continue;
        }
        let is_upper = ch.is_ascii_uppercase();
        if i > 0 && is_upper && !prev_is_sep {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
        prev_is_sep = false;
    }
    out
}

fn name_variants(name: &str) -> Vec<String> {
    vec![
        name.to_string(),
        to_kebab(name),
        to_snake(name),
        name.to_ascii_lowercase(),
    ]
}

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
            let source_root = match &node.payload {
                NodePayload::Component { source_root } => PathBuf::from(source_root),
                _ => unreachable!(),
            };

            let dest_dir = to_root.join(&name);

            // Case 1: Directory-based component (existing behavior) with name variants
            let mut handled = false;
            for base in name_variants(&name) {
                let dir_candidate = source_root.join(&base);
                if dir_candidate.is_dir() {
                fs::create_dir_all(&dest_dir)?;
                let mut files_copied = 0usize;
                    for entry in WalkDir::new(&dir_candidate).into_iter().filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_file() {
                            let rel = path.strip_prefix(&dir_candidate).unwrap();
                            let to_path = dest_dir.join(rel);
                            if let Some(parent) = to_path.parent() {
                                fs::create_dir_all(parent)?;
                            }
                            fs::copy(path, &to_path)?;
                            files_copied += 1;
                        }
                    }
                    report.copied.push(CopyItemReport {
                        from: dir_candidate.display().to_string(),
                        to: dest_dir.display().to_string(),
                        count: files_copied,
                    });
                    info!(from = %dir_candidate.display(), to = %dest_dir.display(), count = files_copied, "component copied");
                    handled = true;
                    break;
                }
            }
            if handled { continue; }

            // Case 2: Single-file component: <Name>.tsx or <Name>.ts under source_root with name variants
            let mut file_candidate: Option<PathBuf> = None;
            'outer: for base in name_variants(&name) {
                for ext in ["tsx", "ts", "jsx", "js"] {
                    let f = source_root.join(format!("{base}.{ext}"));
                    if f.is_file() {
                        file_candidate = Some(f);
                        break 'outer;
                    }
                }
            }

            if let Some(file_path) = file_candidate {
                let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("tsx");
                let base = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&name);
                let to_path = to_root.join(format!("{base}.{ext}"));
                if let Some(parent) = to_path.parent() { fs::create_dir_all(parent)?; }
                fs::copy(&file_path, &to_path)?;
                report.copied.push(CopyItemReport {
                    from: file_path.display().to_string(),
                    to: to_path.display().to_string(),
                    count: 1,
                });
                info!(from = %file_path.display(), to = %to_path.display(), count = 1, "single-file component copied");
                continue;
            }

            // Neither directory nor single-file found
            return Err(CoreError::MissingSource(source_root.join(&name).display().to_string()));
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

pub struct SetupExecutor<'a> {
    registry: &'a Registry,
}

#[derive(Debug)]
pub struct SetupReport {
    pub scaffolded: Vec<String>,
    pub copied: Vec<CopyItemReport>,
    pub notes: Vec<String>,
}

impl<'a> SetupExecutor<'a> {
    pub fn new(registry: &'a Registry) -> Self { Self { registry } }

    pub fn run(&self, node_id: &str, workspace: &Path) -> CoreResult<SetupReport> {
        let node = self.registry.get(node_id)?;
        if node.kind != NodeKind::Setup {
            return Err(CoreError::WrongKind { expected: "setup".into(), actual: format!("{:?}", node.kind) });
        }
        if !workspace.exists() {
            return Err(CoreError::TargetNotFound(workspace.display().to_string()));
        }
        // basic writability probe
        let probe_dir = workspace.join(".entitycli_write_probe");
        if let Err(_e) = fs::create_dir_all(&probe_dir) {
            return Err(CoreError::TargetNotWritable(workspace.display().to_string()));
        }
        let _ = fs::remove_dir_all(&probe_dir);

        // Execute optional scaffold commands first
        let mut scaffolded: Vec<String> = Vec::new();
        if let NodePayload::Setup { commands: Some(cmds), .. } = &node.payload {
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
                                "setup command failed (exit {}): {} {:?}", s.code().unwrap_or(-1), bin, args
                            )));
                        }
                        Err(e) => {
                            return Err(CoreError::InvalidDescriptor(format!(
                                "failed to spawn setup command: {} ({:?} {:?})", e, bin, args
                            )));
                        }
                    }
                }
            }
        }

        // Copy template tree into /entity-auth
        let mut report = SetupReport { scaffolded, copied: Vec::new(), notes: vec!["Overwrite-on-write by default".into()] };
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
                if let Some(parent) = to_path.parent() { fs::create_dir_all(parent)?; }
                fs::copy(path, &to_path)?;
                files_copied += 1;
            }
        }
        report.copied.push(CopyItemReport { from: from_root.display().to_string(), to: to_root.display().to_string(), count: files_copied });
        Ok(report)
    }
}
