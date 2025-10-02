use anyhow::Result;
use engine::Engine;
use executors::DocsExecutor;

use crate::cli::{DocsCmd, DocsReadArgs, DocsSubcommand};
use crate::support::{AppContext, emit_error};

pub fn run(ctx: &AppContext, DocsCmd { command }: DocsCmd) -> Result<()> {
    match command {
        DocsSubcommand::Read(DocsReadArgs { product, node }) => read(ctx, product, node),
    }
}

fn read(ctx: &AppContext, product: String, node: String) -> Result<()> {
    let packs = match ctx.resolve_packs() {
        Ok(p) => p,
        Err(e) => {
            emit_error(&entity_core::error::CoreError::InvalidDescriptor(
                e.to_string(),
            ));
            return Ok(());
        }
    };

    match Engine::bootstrap(packs, Some(&product)) {
        Ok((engine, _graph)) => {
            let exec = DocsExecutor::new(engine.registry());
            match exec.read(node.as_str()) {
                Ok(content) => println!("{}", content),
                Err(err) => emit_error(&err),
            }
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
