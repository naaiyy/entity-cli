use std::path::PathBuf;

use anyhow::Result;

use super::resolve_packs;

#[derive(Clone, Debug)]
pub struct AppContext {
    packs_flag: Option<PathBuf>,
}

impl AppContext {
    pub fn new(packs_flag: Option<PathBuf>) -> Self {
        Self { packs_flag }
    }

    pub fn resolve_packs(&self) -> Result<PathBuf> {
        resolve_packs(
            self.packs_flag
                .clone()
                .unwrap_or_else(|| PathBuf::from("packs")),
        )
    }
}
