use super::common::*;
use std::fs;

use crate::BridgeExecutor;

#[test]
fn scaffold_and_state_roundtrip() {
    let packs = temp_dir();
    let workspace = temp_dir();
    let template = packs.path().join("bridge/test");
    write_file(&template.join("README.md"), "hello");
    let runner_path = template.join("runner.js");
    write_file(&runner_path, "console.log('noop');");
    let config_path = template.join("config.json");
    write_file(&config_path, "{}");
    let logs_parent = template.join("logs");
    fs::create_dir_all(&logs_parent).unwrap();
    let logs_path = logs_parent.join("output.log");
    write_file(&logs_path, "");

    let node = bridge_node(
        "x:bridge:test",
        Some(&template),
        Some(&runner_path),
        Some(&config_path),
        Some(&logs_path),
    );

    let reg = bridge_registry(node);
    let exec = BridgeExecutor::new(&reg);

    let report = exec.scaffold("x:bridge:test", workspace.path()).unwrap();
    assert_eq!(report.copied.len(), 1);

    let info = exec.spawn_descriptor("x:bridge:test").unwrap();
    let packs_root = packs.path().to_path_buf();
    exec.persist_state(
        "x:bridge:test",
        info,
        workspace.path(),
        packs_root.clone(),
        "state-123",
    )
    .unwrap();

    let state = BridgeExecutor::read_state(workspace.path(), "x:bridge:test")
        .unwrap()
        .expect("state");
    assert_eq!(state.id, "state-123");
    assert_eq!(state.status, "pending");

    BridgeExecutor::attach_pid(
        workspace.path(),
        "x:bridge:test",
        1234,
        Some("running"),
        Some("up"),
    )
    .unwrap();
    let state = BridgeExecutor::read_state(workspace.path(), "x:bridge:test")
        .unwrap()
        .expect("state");
    assert_eq!(state.pid, Some(1234));
    assert_eq!(state.status, "running");
    assert_eq!(state.status_message.as_deref(), Some("up"));

    BridgeExecutor::heartbeat(
        workspace.path(),
        "x:bridge:test",
        Some("healthy"),
        Some("ok"),
    )
    .unwrap();
    let state = BridgeExecutor::read_state(workspace.path(), "x:bridge:test")
        .unwrap()
        .expect("state");
    assert_eq!(state.status, "healthy");
    assert_eq!(state.status_message.as_deref(), Some("ok"));

    BridgeExecutor::complete(
        workspace.path(),
        "x:bridge:test",
        Some(0),
        Some("exited"),
        Some("done"),
    )
    .unwrap();
    let state = BridgeExecutor::read_state(workspace.path(), "x:bridge:test")
        .unwrap()
        .expect("state");
    assert_eq!(state.exit_code, Some(0));
    assert_eq!(state.pid, None);

    let stop = BridgeExecutor::stop(workspace.path(), "x:bridge:test")
        .unwrap()
        .expect("stop");
    assert_eq!(stop.status, "stopped");
}
