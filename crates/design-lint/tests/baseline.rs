#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Baseline, Config, check};

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

const TWO: &str = "fn f(ui: &mut Ui) { ui.add_space(4.0); ui.add_space(5.0); }\n";
const THREE: &str = "fn f(ui: &mut Ui) { ui.add_space(4.0); ui.add_space(5.0); ui.add_space(6.0); }\n";

#[test]
fn baseline_round_trips_and_fails_only_when_a_count_rises() {
    let root = tree("baseline", &[("src/a.rs", TWO), ("src/b.rs", "fn f() { let _ = Color32::RED; }\n")]);
    let config = Config::new(["src"]);
    let report = check(&config, &root).unwrap();
    let baseline = Baseline::from_report(&report);
    assert_eq!(baseline.total(), 3);

    let text = baseline.to_toml();
    assert!(text.contains("[\"src/a.rs\"]") && text.contains("\"literal-size/add-space\" = 2"), "{text}");
    assert_eq!(Baseline::parse(Path::new("baseline.toml"), &text).unwrap(), baseline);

    let same = baseline.check(&report);
    assert_eq!((same.exit_code(), same.increases.len()), (0, 0));

    // A count that falls passes and suggests lowering the baseline.
    std::fs::write(root.join("src/a.rs"), "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n").unwrap();
    let fewer = baseline.check(&check(&config, &root).unwrap());
    assert_eq!(fewer.exit_code(), 0);
    assert!(fewer.render().contains("--write-baseline"), "{}", fewer.render());

    // A count that rises fails and names the file, the rule and both counts.
    std::fs::write(root.join("src/a.rs"), THREE).unwrap();
    let more = baseline.check(&check(&config, &root).unwrap());
    assert_eq!(more.exit_code(), 1);
    assert_eq!(
        (more.increases[0].path.as_path(), more.increases[0].rule.as_str(), more.increases[0].baseline, more.increases[0].now),
        (Path::new("src/a.rs"), "literal-size/add-space", 2, 3)
    );
    assert_eq!(more.new_findings.len(), 3);
    assert!(more.render().contains("src/a.rs: [literal-size/add-space] 3 findings, baseline 2"), "{}", more.render());
}

#[test]
fn a_new_file_or_rule_is_a_rise_and_parse_errors_are_never_baselined() {
    let root = tree("baseline-new", &[("src/a.rs", TWO)]);
    let config = Config::new(["src"]);
    let baseline = Baseline::from_report(&check(&config, &root).unwrap());

    std::fs::write(root.join("src/c.rs"), "fn f() { let _ = Color32::RED; }\n").unwrap();
    let newer = baseline.check(&check(&config, &root).unwrap());
    assert_eq!(newer.increases.len(), 1);
    assert_eq!((newer.increases[0].baseline, newer.increases[0].now), (0, 1));

    std::fs::remove_file(root.join("src/c.rs")).unwrap();
    std::fs::write(root.join("src/broken.rs"), "fn f( {").unwrap();
    let broken = Baseline::from_report(&check(&config, &root).unwrap());
    assert_eq!(broken.total(), 2);
    let result = broken.check(&check(&config, &root).unwrap());
    assert_eq!((result.exit_code(), result.increases.len(), result.unbaselined.len()), (1, 0, 1));
}

#[test]
fn baseline_text_errors_are_toml_errors() {
    assert!(Baseline::parse(Path::new("b.toml"), "[\"a.rs\"]\n\"literal-size/vec2\" = \"many\"\n").is_err());
}
