use super::common::*;

use crate::DocsExecutor;
use entity_core::registry::Registry;

#[test]
fn read_happy_path() {
    let dir = temp_dir();
    let doc_path = dir.path().join("doc.md");
    write_file(&doc_path, "hello");

    let node = doc_node("x:doc:one", &doc_path);
    let reg = Registry::new(vec![node]).unwrap();
    let exec = DocsExecutor::new(&reg);

    let out = exec.read("x:doc:one").unwrap();
    assert_eq!(out, "hello");
}
