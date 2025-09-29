use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn bin_cmd() -> Command {
    Command::cargo_bin("entity-cli").unwrap()
}

#[test]
fn packs_not_found_yields_json_error() {
    let mut cmd = bin_cmd();
    cmd.arg("init").arg("entity-auth").arg("--packs").arg("/definitely/not/found");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"code\": \"PACKS_NOT_FOUND\"").and(predicate::str::contains("/definitely/not/found")));
}

#[test]
fn ui_install_missing_mode_yields_missing_selections() {
    let tmp = tempfile::tempdir().unwrap();
    let packs = tempfile::tempdir().unwrap();
    // Create minimal valid packs layout with empty nodes
    let pack_root = packs.path().join("entity-auth");
    let docs_dir = pack_root.join("docs");
    let comps_dir = pack_root.join("components");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&comps_dir).unwrap();
    fs::write(docs_dir.join("nodes.json"), "[]").unwrap();
    fs::write(comps_dir.join("nodes.json"), "[]").unwrap();

    let mut cmd = bin_cmd();
    cmd.current_dir(tmp.path());
    cmd.arg("ui").arg("install").arg("entity-auth")
        .arg("--packs").arg(packs.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"code\": \"MISSING_SELECTIONS\"")
            .and(predicate::str::contains("selection.mode"))
            .and(predicate::str::contains("selection.names")));
}

#[test]
fn docs_read_happy_path_cli() {
    let packs = tempfile::tempdir().unwrap();
    // layout: packs/entity-auth/{docs,components}
    let pack_root = packs.path().join("entity-auth");
    let docs_dir = pack_root.join("docs");
    let comps_dir = pack_root.join("components");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&comps_dir).unwrap();
    // write content file and nodes.json
    let content_dir = docs_dir.join("content");
    fs::create_dir_all(&content_dir).unwrap();
    let doc_path = content_dir.join("getting-started.md");
    fs::write(&doc_path, "hello cli").unwrap();
    let nodes = serde_json::json!([
        {
            "id": "entityauth:docs:getting-started",
            "kind": "doc",
            "title": "Getting Started",
            "meta": {"section":"Setup","tags":["intro","setup"]},
            "prerequisites": [],
            "payload": { "contentPath": doc_path.to_string_lossy() }
        }
    ]);
    fs::write(docs_dir.join("nodes.json"), nodes.to_string()).unwrap();
    fs::write(comps_dir.join("nodes.json"), "[]").unwrap();

    let mut cmd = bin_cmd();
    cmd.arg("docs").arg("read").arg("entity-auth")
        .arg("--node").arg("entityauth:docs:getting-started")
        .arg("--packs").arg(packs.path());
    cmd.assert().success().stdout(predicate::str::contains("hello cli"));
}

#[test]
fn ui_install_happy_path_cli_single() {
    let packs = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();

    // layout: packs/entity-auth/components
    let pack_root = packs.path().join("entity-auth");
    let docs_dir = pack_root.join("docs");
    let comps_dir = pack_root.join("components");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&comps_dir).unwrap();
    fs::write(docs_dir.join("nodes.json"), "[]").unwrap();

    // create component source tree
    let ui_root = comps_dir.join("ui");
    let sign_in = ui_root.join("SignIn");
    fs::create_dir_all(sign_in.join("nested")).unwrap();
    fs::write(sign_in.join("index.tsx"), "export const A = 1;\n").unwrap();
    fs::write(sign_in.join("nested").join("util.ts"), "export const U = 1;\n").unwrap();

    // nodes.json for components
    let nodes = serde_json::json!([
        {
            "id": "entityauth:components:install",
            "kind": "component",
            "title": "Install UI Components",
            "meta": { "mode": ["single","multiple","all"], "names": ["SignIn"] },
            "prerequisites": [
                { "key": "selection.mode", "schema": { "enum": ["single","multiple","all"] } },
                { "key": "selection.names", "schema": { "type": "array", "items": { "enum": ["SignIn"] } }, "optional": true }
            ],
            "payload": { "sourceRoot": ui_root.to_string_lossy() }
        }
    ]);
    fs::write(comps_dir.join("nodes.json"), nodes.to_string()).unwrap();

    let mut cmd = bin_cmd();
    cmd.current_dir(workspace.path());
    cmd.arg("ui").arg("install").arg("entity-auth")
        .arg("--mode").arg("single")
        .arg("--names").arg("SignIn")
        .arg("--packs").arg(packs.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("copied").and(predicate::str::contains("notes")));

    // verify files copied into deterministic layout
    let dest = PathBuf::from(workspace.path())
        .join("entity-auth").join("components").join("SignIn");
    assert!(dest.join("index.tsx").exists());
    assert!(dest.join("nested").join("util.ts").exists());
}


