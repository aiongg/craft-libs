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
fn craft_libs_keeps_props_shadow_an_error() {
    let config = Config::load(&root().join("design-lint.toml")).unwrap();
    assert!(!config.warn.iter().any(|w| w == "props" || w == "props/shadow"), "{:?}", config.warn);
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

/// The linted paths, craft-window, craft-win32 and the benchmark, which the lint does not scan (every other rule would
/// apply to their egui and platform code): `unsafe` is allowed in craft-win32 and the benchmark's allocator only.
#[test]
fn only_craft_win32_and_the_benchmark_allocator_allow_unsafe_code() {
    // A lint path names a directory or a single source file.
    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        if dir.is_file() {
            out.push(dir.to_owned());
            return;
        }
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = root();
    let config = Config::load(&root.join("design-lint.toml")).unwrap();
    let mut files = Vec::new();
    for path in config.paths.iter().map(PathBuf::as_path).chain([Path::new("crates/window"), Path::new("crates/win32"), Path::new("crates/ui-bench")])
    {
        sources(&root.join(path), &mut files);
    }
    let scope = craft_design_lint::StaticScope::new();
    let mut found = Vec::new();
    for file in files {
        let shown = file.strip_prefix(&root).unwrap().to_owned();
        // The rule's own fixtures hold the forbidden text on purpose.
        if shown.starts_with("crates/design-lint") {
            continue;
        }
        let source = std::fs::read_to_string(&file).unwrap();
        found.extend(
            craft_design_lint::analyze_contract(&source, &shown, &scope, true)
                .into_iter()
                .filter(|finding| finding.rule == "contract/unsafe-allow")
                .map(|finding| format!("{}:{}", shown.display(), finding.line)),
        );
    }
    assert!(found.is_empty(), "{found:?}");
    for (file, holds) in [("crates/win32/src/lib.rs", true), ("crates/window/src/win32.rs", false)] {
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        assert_eq!(source.contains("allow(unsafe_code)"), holds, "{file}");
    }
}
