//! Integration tests of craft-design-lint: config.

#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, check};

const ORIGIN: &str = "design-lint.toml";

fn parse(text: &str) -> Result<Config, String> {
    Config::parse(Path::new(ORIGIN), text).map_err(|e| e.to_string())
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

