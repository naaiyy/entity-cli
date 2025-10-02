use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use engine::Engine;
use entity_core::model::GraphPackage;

#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<Option<SessionState>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session(&self) -> Option<SessionState> {
        self.inner.read().unwrap().clone()
    }

    pub fn set_session(&self, session: SessionState) {
        let mut guard = self.inner.write().unwrap();
        *guard = Some(session);
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut guard = self.inner.write().unwrap();
        *guard = None;
    }
}

#[derive(Clone)]
pub struct SessionState {
    pub engine: Engine,
    #[allow(dead_code)]
    pub graph: GraphPackage,
    #[allow(dead_code)]
    pub packs_path: PathBuf,
}

impl SessionState {
    pub fn new(engine: Engine, graph: GraphPackage, packs_path: PathBuf) -> Self {
        Self {
            engine,
            graph,
            packs_path,
        }
    }
}
