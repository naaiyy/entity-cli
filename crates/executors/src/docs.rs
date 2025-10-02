use std::fs;
use std::io::Read;

use entity_core::error::{CoreError, CoreResult};
use entity_core::model::{NodeKind, NodePayload};
use entity_core::registry::Registry;

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
