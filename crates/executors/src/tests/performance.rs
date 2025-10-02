use std::time::Instant;

use super::common::*;

use crate::{ComponentsExecutor, DocsExecutor};
use entity_core::registry::Registry;

#[test]
fn smoke_targets() {
    let dir = temp_dir();
    let doc_path = dir.path().join("doc.md");
    write_file(&doc_path, "hello perf");
    let doc_node = doc_node("x:doc:perf", &doc_path);

    let root = dir.path().join("pack/ui/SignIn");
    for i in 0..10 {
        write_file(&root.join(format!("f{}/a.txt", i)), "x");
    }
    let comp_node = component_node(
        "x:comp:perf",
        dir.path().join("pack/ui").as_path(),
        &["SignIn"],
    );

    let reg = Registry::new(vec![doc_node, comp_node]).unwrap();
    let docs = DocsExecutor::new(&reg);
    let comps = ComponentsExecutor::new(&reg);
    let ws = temp_dir();

    let t0 = Instant::now();
    let _ = docs.read("x:doc:perf").unwrap();
    assert!(t0.elapsed().as_millis() <= 10, "docs read too slow");

    let t1 = Instant::now();
    let rep = comps
        .install(
            "x:comp:perf",
            "single",
            Some(vec!["SignIn".into()]),
            ws.path(),
        )
        .unwrap();
    assert!(rep.copied[0].count >= 10);
    assert!(t1.elapsed().as_millis() <= 300, "components copy too slow");
}
