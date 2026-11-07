//! The design checks over this repository's mapped crates: structure and egui-ban rules, zero findings.

#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::{Path, PathBuf};

use craft_design_lint::{Config, Metadata, RuleScope, WorkspaceScope, check_workspace};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn mapped_crates_have_no_findings() {
    let root = root();
    let config = Config::load(&root.join("design-lint.toml")).unwrap();
    let findings = check_workspace(&root, &config, &Metadata::run(&root).unwrap()).unwrap();
    let listing = findings.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    assert!(findings.is_empty(), "{listing}");
}

#[test]
fn scope_reads_layer_map_manifests_and_spec_allowances() {
    let root = root();
    let scope = WorkspaceScope::load(&root, &Metadata::run(&root).unwrap()).unwrap();
    let alloc = Path::new("crates/ui/src/runtime/egui/alloc.rs");
    assert!(scope.allows("contract/allocate", alloc));
    assert!(!scope.allows("contract/allocate", Path::new("crates/ui/src/shell/mod.rs")));
    assert!(!scope.allows("contract/layers", alloc), "layers are allowed only in `runtime::egui::areas`");
    assert!(scope.allows("contract/layers", Path::new("crates/ui/src/runtime/egui/areas.rs")));
    assert!(!scope.allows("contract/containers:never", alloc));
    assert_eq!(scope.layer(alloc), Some(2));
    assert_eq!(scope.layer(Path::new("crates/ui/src/shell/mod.rs")), Some(7));
    assert_eq!(scope.layer(Path::new("crates/ui/src/testing/mod.rs")), None);
    assert!(scope.egui_linked(alloc));
    assert!(!scope.egui_linked(Path::new("crates/behavior/src/lib.rs")));
    assert!(scope.harness_free(Path::new("crates/ui/tests/it/egui_surface.rs")));
    assert!(!scope.harness_free(Path::new("crates/design-lint/tests/workspace.rs")));
    assert!(scope.is_closed("StateMix"));
    // Rules whose only allowed location is derive output, which is not source text, allow no file.
    for key in ["props/shadow", "props/hand-setter", "contract/shadow", "contract/widget", "contract/overlay-flag", "contract/entry"] {
        assert!(!scope.allows(key, alloc), "{key}");
    }
    assert!(scope.allows("style/token-read", Path::new("crates/ui/src/style/mod.rs")));
    assert!(!scope.allows("style/token-read", Path::new("crates/ui/src/buttons/mod.rs")));
    assert!(scope.allows("behavior/pipeline-only", Path::new("crates/ui/src/edit/mod.rs")));
    assert!(!scope.allows("behavior/pipeline-only", Path::new("crates/ui/src/buttons/mod.rs")));
}
