use std::path::Path;

pub fn to_kebab(input: &str) -> String {
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

pub fn to_snake(input: &str) -> String {
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

pub fn name_variants(name: &str) -> Vec<String> {
    vec![
        name.to_string(),
        to_kebab(name),
        to_snake(name),
        name.to_ascii_lowercase(),
    ]
}

pub fn ensure_writable_dir(dir: &Path) -> std::io::Result<()> {
    // basic writability check: attempt to create and remove a temp directory under workspace
    let probe_dir = dir.join(".entitycli_write_probe");
    std::fs::create_dir_all(&probe_dir)?;
    // best-effort cleanup: try writing a small file too
    let probe_file = probe_dir.join(".probe");
    let _ = std::fs::File::create(&probe_file)?;
    let _ = std::fs::remove_file(&probe_file);
    std::fs::remove_dir_all(&probe_dir)
}
