mod bridge;
mod components;
mod docs;
mod setup;

mod util;

pub use bridge::{
    BridgeExecutor, BridgeProcessInfo, BridgeProcessState, BridgeProcessStateProcess,
    BridgeScaffoldReport, BridgeStopResult,
};
pub use components::{ComponentsExecutor, CopyItemReport, CopyReport};
pub use docs::DocsExecutor;
pub use setup::{SetupExecutor, SetupReport};

#[cfg(test)]
mod tests;
