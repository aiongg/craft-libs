#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, check};

const ORIGIN: &str = "design-lint.toml";

fn parse(text: &str) -> Result<Config, String> {
    Config::parse(Path::new(ORIGIN), text).map_err(|e| e.to_string())
}

#[test]
fn exemption_needs_a_reason() {
    let err = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\n").unwrap_err();
    assert!(err.contains("src/canvas") && err.contains("reason"), "{err}");
    assert!(parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nreason = \"  \"\n").is_err());
    assert!(parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nreason = \"document units\"\n").is_ok());
}

#[test]
fn unknown_rule_in_an_exemption_is_an_error() {
    let err = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"a\"\nrules = [\"literal-size/nope\"]\nreason = \"because\"\n").unwrap_err();
    assert!(err.contains("literal-size/nope"), "{err}");
    let ok = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"a\"\nrules = [\"literal-size\", \"raw-colour/const\"]\nreason = \"because\"\n")
        .unwrap();
    assert_eq!(ok.exempt[0].rules.len(), 2);
}

#[test]
fn exemption_matches_path_components_and_rule_families() {
    let config =
        parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nrules = [\"literal-size\"]\nreason = \"document units\"\n").unwrap();
    let e = &config.exempt[0];
    assert!(e.covers(Path::new("src/canvas/a.rs"), "literal-size/vec2"));
    assert!(e.covers(Path::new("src/canvas"), "literal-size/stroke"));
    assert!(!e.covers(Path::new("src/canvas/a.rs"), "raw-colour/const"));
    assert!(!e.covers(Path::new("src/canvas2/a.rs"), "literal-size/vec2"));
    assert!(!e.covers(Path::new("src/panels/a.rs"), "literal-size/vec2"));
}

#[test]
fn check_applies_exemptions_markers_and_counts() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-exempt");
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, src: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, src).unwrap();
    };
    let bad = "fn f(ui: &mut Ui) { ui.add_space(4.0); let _ = Color32::RED; }\n";
    write("src/canvas/a.rs", bad);
    write("src/panels/b.rs", bad);
    write("src/panels/c.rs", "fn f(ui: &mut Ui) {\n    ui.add_space(4.0); // design: allow(gap)\n    let x = 1; // design: allow(stale)\n}\n");
    let text = "[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nrules = [\"literal-size\"]\nreason = \"document units\"\n";
    let config = Config::parse(Path::new(ORIGIN), text).unwrap();
    let report = check(&config, &root).unwrap();
    let found: Vec<(String, &str)> = report.findings.iter().map(|f| (f.path.display().to_string(), f.rule)).collect();
    assert_eq!(
        found,
        [
            ("src/canvas/a.rs".to_owned(), "raw-colour/const"),
            ("src/panels/b.rs".to_owned(), "literal-size/add-space"),
            ("src/panels/b.rs".to_owned(), "raw-colour/const"),
            ("src/panels/c.rs".to_owned(), "marker/unused"),
        ]
    );
    assert_eq!((report.markers, report.exemptions), (2, 1));
    assert!(report.summary_line().ends_with("2 markers, 1 exemption"), "{}", report.summary_line());
}

#[test]
fn unused_markers_fail_only_when_denied() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-deny-unused");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "fn f() {\n    let x = 1; // design: allow(stale)\n}\n").unwrap();
    let mut config = Config::new(["src"]);
    let report = check(&config, &root).unwrap();
    assert_eq!((report.findings.len(), report.exit_code()), (1, 0));
    assert!(report.render().contains("warning[marker/unused]"));
    config.deny_unused = true;
    assert_eq!(check(&config, &root).unwrap().exit_code(), 1);
    let parsed = Config::parse(Path::new(ORIGIN), "[lint]\npaths = [\"src\"]\ndeny-unused = true\n").unwrap();
    assert!(parsed.deny_unused);
}
