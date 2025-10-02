use anyhow::Result;
use engine::Engine;

use crate::cli::InitArgs;
use crate::support::{AppContext, emit_error};

pub fn run(ctx: &AppContext, InitArgs { product }: InitArgs) -> Result<()> {
    let packs = ctx.resolve_packs()?;

    match Engine::bootstrap(packs, Some(&product)) {
        Ok((_engine, graph)) => {
            println!("{}", serde_json::to_string_pretty(&graph)?);
        }
        Err(err) => {
            if let Some(core) = err.downcast_ref() {
                emit_error(core);
            } else {
                emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                    err.to_string(),
                ));
            }
        }
    }

    Ok(())
}
