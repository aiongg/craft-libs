//! Integration tests of craft-design-lint: the egui-ban and API-shape rules.
//!
//! Each rule has a reject fixture that must fail with the rule's id; rules with an "allowed in" location
//! also pass when the same source sits in that location.

#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, StaticScope, analyze_contract, check_with_scope};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/egui-rules");
const ALLOWED_DIR: &str = "src/runtime/egui";

fn fixture(dir: &str, name: &str) -> String {
    std::fs::read_to_string(Path::new(FIXTURES).join(dir).join(name)).unwrap()
}

/// The scope that allows every egui call and target check in `runtime/egui`, and lists the crate as egui-linked.
fn scope() -> StaticScope {
    ["contract", "test/kit"]
        .into_iter()
        .fold(StaticScope::new(), |s, k| s.allow(k, ALLOWED_DIR))
        .allow("contract/platform:target", ALLOWED_DIR)
        .allow("contract/motion:clock", ALLOWED_DIR)
        .egui_linked("crates/x")
        .harness_free("crates/x/tests/it")
        .closed("Politeness")
}

fn rules_at(src: &str, path: &str, scope: &StaticScope) -> Vec<&'static str> {
    let mut rules: Vec<_> = analyze_contract(src, Path::new(path), scope, true).iter().map(|f| f.rule).collect();
    rules.dedup();
    rules
}

/// Reject fixtures: file, the rule it must trigger, the path it is checked at.
const REJECT: [(&str, &str, &str); 20] = [
    ("contract-egui-import.rs", "contract/egui-import", "src/ui/a.rs"),
    ("contract-allocate.rs", "contract/allocate", "src/ui/a.rs"),
    ("contract-egui-widget.rs", "contract/egui-widget", "src/ui/a.rs"),
    ("contract-access.rs", "contract/access", "src/ui/a.rs"),
    ("contract-layers.rs", "contract/layers", "src/ui/a.rs"),
    ("contract-layers-swallow.rs", "contract/layers", "src/ui/a.rs"),
    ("contract-memory.rs", "contract/memory", "src/ui/a.rs"),
    ("contract-keys.rs", "contract/keys", "src/ui/a.rs"),
    ("contract-pointer.rs", "contract/pointer", "src/ui/a.rs"),
    ("contract-motion.rs", "contract/motion", "src/ui/a.rs"),
    ("contract-motion-clock.rs", "contract/motion", "src/ui/a.rs"),
    ("contract-containers.rs", "contract/containers", "src/ui/a.rs"),
    ("contract-containers-scroll.rs", "contract/containers", "src/ui/a.rs"),
    ("contract-platform.rs", "contract/platform", "src/ui/a.rs"),
    ("test-layout.rs", "test/layout", "crates/x/tests/other.rs"),
    ("test-kit.rs", "test/kit", "crates/x/src/lib.rs"),
    ("test-harness.rs", "test/harness", "crates/x/tests/it/main.rs"),
    ("test-cfg-test.rs", "test/cfg-test", "crates/x/src/lib.rs"),
    ("test-literal-size.rs", "test/literal-size", "crates/x/tests/it/main.rs"),
    ("test-ignored.rs", "test/ignored", "src/ui/a.rs"),
];

#[test]
fn every_reject_fixture_fails_with_its_rule() {
    for (file, rule, path) in REJECT {
        let rules = rules_at(&fixture("reject", file), path, &scope());
        assert!(rules.contains(&rule), "{file}: expected {rule}, got {rules:?}");
    }
    let rules = rules_at(&fixture("reject", "api-non-exhaustive.rs"), "src/lib.rs", &scope());
    assert_eq!(rules, ["api/non-exhaustive"]);
}

#[test]
fn api_rule_reports_each_shape() {
    let findings = analyze_contract(&fixture("reject", "api-non-exhaustive.rs"), Path::new("src/lib.rs"), &scope(), false);
    let messages: Vec<_> = findings.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(messages.len(), 5, "{messages:#?}");
}

#[test]
fn allowed_location_passes_the_same_source() {
    let allowed_only = REJECT
        .iter()
        .filter(|(file, _, _)| !file.starts_with("test-") && !matches!(*file, "contract-layers-swallow.rs" | "contract-containers-scroll.rs"));
    for (file, rule, _) in allowed_only {
        let rules = rules_at(&fixture("reject", file), &format!("{ALLOWED_DIR}/a.rs"), &scope());
        assert!(!rules.contains(rule), "{file}: {rule} fired in the allowed location");
    }
    let rules = rules_at(&fixture("reject", "test-kit.rs"), &format!("{ALLOWED_DIR}/a.rs"), &scope());
    assert!(rules.is_empty(), "{rules:?}");
}

#[test]
fn rules_with_no_allowed_location_fire_everywhere() {
    for file in ["contract-layers-swallow.rs", "contract-containers-scroll.rs"] {
        let rules = rules_at(&fixture("reject", file), &format!("{ALLOWED_DIR}/a.rs"), &scope());
        assert_eq!(rules.len(), 1, "{file}: {rules:?}");
    }
}

#[test]
fn sub_permissions_are_separate_keys() {
    let target_only = StaticScope::new().allow("contract/platform:target", "src/host");
    let src = "#[cfg(target_os = \"linux\")]\nfn a() {}\nfn f(ctx: &Context) { ctx.copy_text(t); }";
    assert_eq!(rules_at(src, "src/host/a.rs", &target_only), ["contract/platform"]);
    let findings = analyze_contract(src, Path::new("src/host/a.rs"), &target_only, true);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("copy_text"));
}

#[test]
fn layer_zero_unit_tests_may_assert_literals() {
    let src = fixture("reject", "test-literal-size.rs");
    let layered = StaticScope::new().egui_linked("crates/x").layer("crates/x/tests", 0);
    assert!(rules_at(&src, "crates/x/tests/it/main.rs", &layered).is_empty());
}

#[test]
fn craft_types_that_share_egui_names_are_not_flagged() {
    let rules = rules_at(&fixture("accept", "clean.rs"), "src/ui/a.rs", &scope());
    assert!(rules.is_empty(), "{rules:?}");
}

#[test]
fn glob_and_alias_imports_resolve_to_egui() {
    assert_eq!(rules_at("use egui::*;\nfn f() { let _ = Button::new(\"x\"); }", "src/a.rs", &StaticScope::new()), ["contract/egui-widget"]);
    assert_eq!(rules_at("use egui::ScrollArea as S;\nfn f() { S::vertical(); }", "src/a.rs", &StaticScope::new()), ["contract/containers"]);
    assert!(rules_at("use crate::ui::*;\nfn f() { let _ = Button::new(\"x\"); }", "src/a.rs", &StaticScope::new()).is_empty());
}

#[test]
fn check_with_scope_applies_scope_markers_and_exemptions() {
    let root = Path::new(FIXTURES).join("tree");
    let scope = StaticScope::new().allow("contract", "src/runtime/egui");
    let report = check_with_scope(&Config::new(["src"]), &root, &scope).unwrap();
    let hits: Vec<_> = report.findings.iter().map(|f| (f.path.display().to_string(), f.rule, f.line)).collect();
    assert_eq!(hits, [("src/ui/bad.rs".to_owned(), "contract/egui-widget", 2)]);
}

#[test]
fn older_rules_defer_to_the_scoped_rules() {
    let root = Path::new(FIXTURES).join("older");
    let scope = StaticScope::new().allow("style-mutation", "src/runtime/egui/style.rs");
    let mut config = Config::new(["src", "tests"]);
    config.include_tests = true;
    let report = check_with_scope(&config, &root, &scope).unwrap();
    let hits: Vec<_> = report.findings.iter().map(|f| (f.path.display().to_string(), f.rule, f.line)).collect();
    assert_eq!(hits, [("src/other.rs".to_owned(), "style-mutation", 2), ("src/other.rs".to_owned(), "literal-size/vec2", 6)]);
}

#[test]
fn literal_sizes_may_fill_the_testing_module_like_tests() {
    let root = Path::new(FIXTURES).join("testing-module");
    let scope = StaticScope::new().egui_linked("crates/x");
    let mut config = Config::new(["."]);
    config.include_tests = true;
    let report = check_with_scope(&config, &root, &scope).unwrap();
    let hits: Vec<_> = report.findings.iter().map(|f| (f.path.display().to_string(), f.rule, f.line)).collect();
    assert_eq!(
        hits,
        [("src/other.rs".to_owned(), "literal-size/vec2", 2), ("src/testing/examples/sheet.rs".to_owned(), "literal-size/vec2", 2)],
        "the kit may fill literal sizes; the examples may not"
    );
}

#[test]
fn exemptions_and_markers_accept_the_new_rule_ids() {
    let text =
        "[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/ui\"\nrules = [\"contract\", \"test\", \"api/non-exhaustive\"]\nreason = \"legacy\"\n";
    assert!(Config::parse(Path::new("design-lint.toml"), text).is_ok());
}

#[test]
fn layer_one_names_accesskit_data_types_and_no_other_layer_does() {
    let src = fixture("accept", "accesskit-data.rs");
    let layered = StaticScope::new().layer("crates/data", 1);
    assert!(rules_at(&src, "crates/data/src/intent.rs", &layered).is_empty());
    assert_eq!(rules_at(&src, "crates/data/src/intent.rs", &StaticScope::new().layer("crates/data", 2)), ["contract/egui-import"]);
    assert_eq!(rules_at(&src, "crates/data/src/intent.rs", &StaticScope::new()), ["contract/egui-import"]);
}

#[test]
fn family_modules_neither_import_runtime_egui_nor_call_ui_text_shortcuts() {
    let src = fixture("reject", "family-raw-egui.rs");
    let layered = StaticScope::new().layer("crates/x/src/text", 5).layer("crates/x/src/shell", 7);
    let findings = analyze_contract(&src, Path::new("crates/x/src/text/mod.rs"), &layered, false);
    let hits: Vec<_> = findings.iter().map(|f| (f.rule, f.line)).collect();
    assert_eq!(hits, [("contract/runtime-egui", 1), ("contract/egui-widget", 5), ("contract/runtime-egui", 7)]);
    let shell = analyze_contract(&src, Path::new("crates/x/src/shell/app.rs"), &layered, false);
    assert!(shell.iter().all(|f| f.rule != "contract/runtime-egui"), "layer 7 hosts the adapter");
}

#[test]
fn text_shortcuts_on_builders_are_not_ui_calls() {
    let src = "fn f(field: Field) { let _ = field.label(\"x\"); }";
    assert!(rules_at(src, "src/a.rs", &StaticScope::new()).is_empty());
}
