use super::common::*;

use crate::ComponentsExecutor;

#[test]
fn install_mode_all_names_present_errors() {
    let (reg, _dir) = component_registry_fixture(&["SignIn"]);
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let err = exec
        .install(
            "x:comp:install",
            "all",
            Some(vec!["SignIn".into()]),
            ws.path(),
        )
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("names must be omitted for mode all"));
}

#[test]
fn install_single_copies_files() {
    let (reg, _dir) = component_registry_fixture(&["SignIn"]);
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let rep = exec
        .install(
            "x:comp:install",
            "single",
            Some(vec!["SignIn".into()]),
            ws.path(),
        )
        .unwrap();
    assert_eq!(rep.copied.len(), 1);
    assert!(
        ws.path()
            .join("entity-auth/components/SignIn/index.tsx")
            .exists()
    );
}

#[test]
fn install_multiple_validates_and_copies_many() {
    let (reg, _dir) = component_registry_fixture(&["SignIn", "UserMenu"]);
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let rep = exec
        .install(
            "x:comp:install",
            "multiple",
            Some(vec!["SignIn".into(), "UserMenu".into()]),
            ws.path(),
        )
        .unwrap();
    assert_eq!(rep.copied.len(), 2);
    assert!(
        ws.path()
            .join("entity-auth/components/SignIn/index.tsx")
            .exists()
    );
    assert!(
        ws.path()
            .join("entity-auth/components/UserMenu/index.tsx")
            .exists()
    );
}

#[test]
fn install_invalid_names_reports_list() {
    let (reg, _dir) = component_registry_fixture(&["SignIn"]);
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let err = exec
        .install(
            "x:comp:install",
            "multiple",
            Some(vec!["Nope".into(), "SignIn".into()]),
            ws.path(),
        )
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Invalid selection names"));
}

#[test]
fn install_overwrite_copies_and_counts_files() {
    let (reg, dir) = component_registry_fixture(&["AuthProvider"]);
    write_file(
        &dir.path()
            .join("pack/components/AuthProvider/nested/util.ts"),
        "export const U = 1;\n",
    );
    let exec = ComponentsExecutor::new(&reg);
    let ws = temp_dir();
    let rep1 = exec
        .install(
            "x:comp:install",
            "single",
            Some(vec!["AuthProvider".into()]),
            ws.path(),
        )
        .unwrap();
    assert_eq!(rep1.copied[0].count, 2);
    write_file(
        &ws.path()
            .join("entity-auth/components/AuthProvider/index.tsx"),
        "changed\n",
    );
    let rep2 = exec
        .install(
            "x:comp:install",
            "single",
            Some(vec!["AuthProvider".into()]),
            ws.path(),
        )
        .unwrap();
    assert_eq!(rep2.copied[0].count, 2);
}
