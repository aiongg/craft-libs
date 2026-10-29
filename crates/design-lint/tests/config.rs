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
    assert_eq!((report.markers, report.exemptions.len()), (2, 1));
    assert!(report.summary_line().ends_with("2 markers, 1 exemption, 0 clippy allows"), "{}", report.summary_line());
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

fn tree(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    for (rel, src) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, src).unwrap();
    }
    root
}

#[test]
fn exemptions_never_hide_parse_findings() {
    let root = tree("exempt-parse", &[("src/canvas/broken.rs", "fn f( {")]);
    let config = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nreason = \"document units\"\n").unwrap();
    let report = check(&config, &root).unwrap();
    assert!(report.findings.iter().any(|f| f.rule == "parse"), "{:#?}", report.findings);
    assert_eq!(report.exit_code(), 1);

    let err = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src\"\nrules = [\"parse\"]\nreason = \"because\"\n").unwrap_err();
    assert!(err.contains("parse"), "{err}");
}

#[test]
fn unused_exemptions_are_reported() {
    let root = tree("exempt-unused", &[("src/canvas/a.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n"), ("src/panels/b.rs", "fn f() {}\n")]);
    let text = "[lint]\npaths = [\"src\"]\n\
        [[exempt]]\npath = \"src/canvas\"\nrules = [\"literal-size\"]\nreason = \"document units\"\n\
        [[exempt]]\npath = \"src/panels\"\nreason = \"nothing to hide\"\n\
        [[exempt]]\npath = \"src/gone\"\nreason = \"deleted directory\"\n";
    let mut config = parse(text).unwrap();
    let report = check(&config, &root).unwrap();
    let found: Vec<(String, &str, String)> = report.findings.iter().map(|f| (f.path.display().to_string(), f.rule, f.message.clone())).collect();
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!((found[0].0.as_str(), found[0].1), ("src/gone", "exemption/unused"));
    assert!(found[0].2.contains("matches no checked file"), "{}", found[0].2);
    assert_eq!((found[1].0.as_str(), found[1].1), ("src/panels", "exemption/unused"));
    assert!(found[1].2.contains("silences no finding"), "{}", found[1].2);
    assert!(report.findings.iter().all(|f| f.severity == craft_design_lint::Severity::Warning));
    assert_eq!(report.exit_code(), 0);

    config.deny_unused = true;
    assert_eq!(check(&config, &root).unwrap().exit_code(), 1);
}

#[test]
fn report_lists_each_exemption_with_its_rules_and_reason() {
    let root = tree("exempt-list", &[("src/canvas/a.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); let _ = Color32::RED; }\n")]);
    let text = "[lint]\npaths = [\"src\"]\n\
        [[exempt]]\npath = \"src/canvas\"\nrules = [\"literal-size\", \"raw-colour\"]\nreason = \"document units\"\n";
    let report = check(&parse(text).unwrap(), &root).unwrap();
    let out = report.render();
    assert!(out.contains("exemption: src/canvas (literal-size, raw-colour): document units\n"), "{out}");
    assert!(out.lines().last().unwrap().starts_with("design-lint: no findings"), "{out}");
    let all = parse("[lint]\npaths = [\"src\"]\n[[exempt]]\npath = \"src/canvas\"\nreason = \"document units\"\n").unwrap();
    assert!(check(&all, &root).unwrap().render().contains("exemption: src/canvas (all rules): document units\n"));
}

#[test]
fn summary_prints_after_the_findings() {
    let root = tree("summary-order", &[("src/a.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n")]);
    let out = check(&Config::new(["src"]), &root).unwrap().render_with_table();
    let finding = out.find("src/a.rs:1:").unwrap();
    let table = out.find("findings\n").unwrap();
    let line = out.rfind("design-lint: 1 finding").unwrap();
    assert!(finding < table && table < line, "{out}");
}
