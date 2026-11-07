//! Integration tests of craft-design-lint: the contract-shape rules (`behavior/*`, `props/*`, `style/*`, `contract/entry`,
//! `contract/shadow`, `contract/widget`, `contract/overlay-flag`).
//!
//! Every rule has a reject fixture that must fail with the rule's id. The accept fixtures hold the nearest legal forms
//! and must produce no finding. Each rule's "allowed in" location, and the layers it reads, have their own cases.

#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, Severity, StaticScope, analyze_contract, check_with_scope};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/contract-rules");

fn fixture(dir: &str, name: &str) -> String {
    std::fs::read_to_string(Path::new(FIXTURES).join(dir).join(name)).unwrap()
}

/// Path of a component-family module (layer 5) and of a pattern module (layer 6).
const FAMILY: &str = "crates/x/src/chips/mod.rs";

fn scope() -> StaticScope {
    StaticScope::new().layer("crates/x/src/chips", 5).layer("crates/x/src/patterns", 6).layer("crates/x/src/shell", 7)
}

fn rules_at(src: &str, path: &str, scope: &StaticScope) -> Vec<&'static str> {
    let mut rules: Vec<_> = analyze_contract(src, Path::new(path), scope, false).iter().map(|f| f.rule).collect();
    rules.dedup();
    rules
}

/// Reject fixtures: file and the rules it must trigger (all of them, nothing else).
const REJECT: [(&str, &[&str]); 17] = [
    ("behavior-pipeline-only.rs", &["behavior/pipeline-only"]),
    ("props-reserved-setter.rs", &["props/reserved-setter"]),
    ("props-hand-setter.rs", &["props/hand-setter"]),
    ("props-own-field.rs", &["props/own-field"]),
    ("props-shadow.rs", &["props/shadow"]),
    ("props-literal-size.rs", &["props/literal-size"]),
    ("style-state-pick.rs", &["style/state-pick"]),
    ("style-match-state.rs", &["style/match-state"]),
    ("style-states-field.rs", &["style/states-field"]),
    ("style-size-arm.rs", &["style/size-arm"]),
    ("style-token-read.rs", &["style/token-read"]),
    ("style-instance-setter.rs", &["style/instance-setter"]),
    ("style-to-egui.rs", &["style/to-egui"]),
    ("contract-entry.rs", &["contract/entry"]),
    ("contract-shadow.rs", &["contract/shadow"]),
    ("contract-widget.rs", &["contract/widget"]),
    ("contract-overlay-flag.rs", &["contract/overlay-flag"]),
];

#[test]
fn every_reject_fixture_fails_with_exactly_its_rules() {
    for (file, expected) in REJECT {
        let mut rules = rules_at(&fixture("reject", file), FAMILY, &scope());
        rules.sort_unstable();
        assert_eq!(rules, expected, "{file}");
    }
}

#[test]
fn each_reject_fixture_reports_every_shape() {
    let count = |file: &str, rule: &str| {
        analyze_contract(&fixture("reject", file), Path::new(FAMILY), &scope(), false).iter().filter(|f| f.rule == rule).count()
    };
    assert_eq!(count("behavior-pipeline-only.rs", "behavior/pipeline-only"), 4);
    assert_eq!(count("props-own-field.rs", "props/own-field"), 3);
    assert_eq!(count("props-shadow.rs", "props/shadow"), 2, "a field named like a setter, and a `name = ..` override");
    assert_eq!(count("props-literal-size.rs", "props/literal-size"), 2);
    assert_eq!(count("style-states-field.rs", "style/states-field"), 3);
    assert_eq!(count("style-token-read.rs", "style/token-read"), 2);
    assert_eq!(count("style-instance-setter.rs", "style/instance-setter"), 2, "by name and by a style-typed parameter");
    assert_eq!(count("contract-entry.rs", "contract/entry"), 2, "an inherent `show` and a free function");
    assert_eq!(count("contract-widget.rs", "contract/widget"), 2, "imported and qualified");
}

#[test]
fn accept_fixtures_pass() {
    for file in ["behavior-pipeline-only.rs", "props.rs", "style.rs", "entry.rs"] {
        let findings = analyze_contract(&fixture("accept", file), Path::new(FAMILY), &scope(), false);
        assert!(findings.is_empty(), "{file}: {findings:?}");
    }
}

#[test]
fn layer_rules_read_only_their_layers() {
    let pipeline = fixture("reject", "behavior-pipeline-only.rs");
    for (path, hit) in [
        ("crates/x/src/chips/a.rs", true),
        ("crates/x/src/patterns/a.rs", true),
        ("crates/x/src/shell/a.rs", true),
        ("crates/x/src/contract/a.rs", false),
    ] {
        let layered = scope().layer("crates/x/src/contract", 4);
        assert_eq!(!rules_at(&pipeline, path, &layered).is_empty(), hit, "{path}");
    }
    let entry = fixture("reject", "contract-entry.rs");
    let app = scope().layer("crates/app/src", 8);
    assert_eq!(rules_at(&entry, "crates/x/src/patterns/a.rs", &app), ["contract/entry"]);
    assert!(rules_at(&entry, "crates/app/src/panel.rs", &app).is_empty(), "layer 8 may write panels as functions");
    for file in ["style-state-pick.rs", "style-match-state.rs", "style-size-arm.rs"] {
        let src = fixture("reject", file);
        assert_eq!(rules_at(&src, "crates/app/src/panel.rs", &app).len(), 1, "{file}: layer 8 is checked");
        let runtime = scope().layer("crates/x/src/runtime", 2);
        assert!(rules_at(&src, "crates/x/src/runtime/a.rs", &runtime).is_empty(), "{file}: layer 2 is not checked");
    }
}

#[test]
fn allowed_locations_pass_the_same_source() {
    let allowed = scope().layer("crates/x/src/edit", 5).allow("behavior/pipeline-only", "crates/x/src/edit");
    assert!(rules_at(&fixture("reject", "behavior-pipeline-only.rs"), "crates/x/src/edit/a.rs", &allowed).is_empty());

    let style_home = scope().layer("crates/x/src/chips", 5).allow("style", "crates/x/src/chips");
    for (file, _) in REJECT.iter().filter(|(f, _)| f.starts_with("style-")) {
        assert!(rules_at(&fixture("reject", file), FAMILY, &style_home).is_empty(), "{file}");
    }
}

#[test]
fn shape_rules_ignore_test_code() {
    let src = format!("#[cfg(test)]\nmod tests {{\n{}\n}}", fixture("reject", "contract-shadow.rs"));
    assert!(rules_at(&src, FAMILY, &scope()).is_empty());
}

fn shadow_findings(config: &Config) -> Vec<Severity> {
    let root = Path::new(FIXTURES).join("tree");
    let report = check_with_scope(config, &root, &scope()).unwrap();
    report.findings.iter().filter(|f| f.rule == "props/shadow").map(|f| f.severity).collect()
}

#[test]
fn props_shadow_is_an_error_unless_the_config_warns() {
    let mut config = Config::new(["src"]);
    assert_eq!(shadow_findings(&config), [Severity::Error]);
    config.warn = vec!["props/shadow".to_owned()];
    assert_eq!(shadow_findings(&config), [Severity::Warning]);
}

#[test]
fn config_warn_names_known_rules() {
    let ok = "[lint]\npaths = [\"src\"]\nwarn = [\"props/shadow\", \"style\"]\n";
    assert_eq!(Config::parse(Path::new("design-lint.toml"), ok).unwrap().warn, ["props/shadow", "style"]);
    let bad = "[lint]\npaths = [\"src\"]\nwarn = [\"props/nope\"]\n";
    assert!(Config::parse(Path::new("design-lint.toml"), bad).is_err());
}
