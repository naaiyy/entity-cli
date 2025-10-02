use std::path::PathBuf;

use anyhow::Result;
use entity_core::error::CoreError;

pub fn emit_error(err: &CoreError) {
    let details = match err {
        CoreError::MissingSelections(keys) => Some(serde_json::json!({ "missing": keys })),
        CoreError::InvalidSelection(msg) => Some(serde_json::json!({ "message": msg })),
        CoreError::InvalidNames(list) => Some(serde_json::json!({ "invalidNames": list })),
        CoreError::PacksNotFound(p) => Some(serde_json::json!({ "packsPath": p })),
        _ => None,
    };
    let env = err.envelope(details);
    println!("{}", serde_json::to_string_pretty(&env).unwrap());
}

pub fn resolve_packs(flag: PathBuf) -> Result<PathBuf> {
    // precedence: flag -> env -> config -> default
    if flag != PathBuf::from("packs") {
        return Ok(flag);
    }
    if let Ok(env_path) = std::env::var("ENTITY_CLI_PACKS") {
        return Ok(env_path.into());
    }
    // config file entitycli.json with { "packsDir": "..." }
    let cfg_path = PathBuf::from("entitycli.json");
    if cfg_path.exists() {
        let content = std::fs::read_to_string(&cfg_path)?;
        let v: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(p) = v.get("packsDir").and_then(|x| x.as_str()) {
            return Ok(p.into());
        }
    }
    Ok(flag)
}
