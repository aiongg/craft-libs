#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, Finding, Severity, analyze_source, check};

fn run(src: &str) -> Vec<Finding> {
    analyze_source(src, false).findings
}

fn rules(src: &str) -> Vec<&'static str> {
    run(src).iter().map(|f| f.rule).collect()
}

fn flagged(src: &str, rule: &str) -> bool {
    rules(src) == [rule]
}

fn clean(src: &str) -> bool {
    let findings = run(src);
    findings.is_empty() || panic!("unexpected findings: {findings:#?}")
}

#[test]
fn vec2_forms() {
    assert!(flagged("fn f() { let _ = vec2(28.0, 24.0); }", "literal-size/vec2"));
    assert!(flagged("fn f() { let _ = egui::Vec2::new(4.0, w); }", "literal-size/vec2"));
    assert!(flagged("fn f() { let _ = Vec2::splat(8.0); }", "literal-size/vec2"));
    assert!(flagged("fn f() { let _ = Vec2 { x: 3.0, y: h }; }", "literal-size/vec2"));
    assert!(clean("fn f() { let _ = vec2(0.0, 0.0); let _ = Vec2::splat(0.0); let _ = vec2(w, h); let _ = Vec2 { x: w, y: 0.0 }; }"));
}

#[test]
fn literal_shapes() {
    for arg in ["-4.0", "(4.0)", "4 as f32", "4.0_f32", "2.0 * 3.0", "-(1.0 + 2.0)", "8"] {
        let src = format!("fn f(ui: &mut Ui) {{ ui.add_space({arg}); }}");
        assert!(flagged(&src, "literal-size/add-space"), "{arg}");
    }
    for arg in ["0.0", "-0.0", "0", "0.0 * 5.0", "gap", "self.gap", "tokens.space.sm", "pad()", "w / 2.0"] {
        let src = format!("fn f(ui: &mut Ui) {{ ui.add_space({arg}); }}");
        assert!(clean(&src), "{arg}");
    }
}

#[test]
fn corner_radius_forms() {
    for src in [
        "fn f() { let _ = CornerRadius::same(4); }",
        "fn f() { let _ = CornerRadius::from(6.0); }",
        "fn f() { let _ = CornerRadius { nw: 2, ne: r, sw: r, se: r }; }",
        "fn f() { let _ = Frame::new().corner_radius(3.0); }",
    ] {
        assert!(flagged(src, "literal-size/corner-radius"), "{src}");
    }
    assert!(clean("fn f() { let _ = CornerRadius::same(0); let _ = Frame::new().corner_radius(r); let _ = CornerRadius::ZERO; }"));
}

#[test]
fn stroke_checks_the_width_only() {
    assert!(flagged("fn f() { let _ = Stroke::new(1.0, c); }", "literal-size/stroke"));
    assert!(flagged("fn f() { let _ = PathStroke::new(2.0, c); }", "literal-size/stroke"));
    assert!(flagged("fn f() { let _ = Stroke { width: 1.5, color: c }; }", "literal-size/stroke"));
    assert!(clean("fn f() { let _ = Stroke::new(w, c); let _ = Stroke::new(0.0, c); let _ = Stroke::new(w, 5.0); }"));
}

#[test]
fn margin_forms() {
    for src in [
        "fn f() { let _ = Margin::same(4); }",
        "fn f() { let _ = Margin::symmetric(8, 0); }",
        "fn f() { let _ = Margin { left: 1, right: 0, top: 0, bottom: 0 }; }",
        "fn f() { let _ = Frame::new().inner_margin(6.0); }",
        "fn f() { let _ = Frame::new().outer_margin(6.0); }",
    ] {
        assert!(flagged(src, "literal-size/margin"), "{src}");
    }
    assert!(clean("fn f() { let _ = Margin::same(0); let _ = Margin::symmetric(x, 0); let _ = Frame::new().inner_margin(m); }"));
}

#[test]
fn font_id_checks_the_size_only() {
    assert!(flagged("fn f() { let _ = FontId::proportional(13.0); }", "literal-size/font-id"));
    assert!(flagged("fn f() { let _ = FontId::monospace(12.0); }", "literal-size/font-id"));
    assert!(flagged("fn f() { let _ = FontId::new(11.0, family); }", "literal-size/font-id"));
    assert!(clean("fn f() { let _ = FontId::proportional(size); let _ = FontId::new(size, FontFamily::Name(\"x\".into())); }"));
}

#[test]
fn same_file_consts_resolve() {
    let src = "const GAP: f32 = 4.0;\nconst DOUBLE: f32 = GAP * 2.0;\nstatic NONE: f32 = 0.0;\nfn f(ui: &mut Ui) {\n    ui.add_space(GAP);\n    ui.add_space(DOUBLE);\n    ui.add_space(NONE);\n    ui.add_space(OTHER);\n}\n";
    let found = run(src);
    assert_eq!(found.len(), 2);
    assert_eq!((found[0].line, found[1].line), (5, 6));
    assert!(found[0].message.contains("`GAP` (const, line 1)"), "{}", found[0].message);
    assert!(found[1].message.contains("`DOUBLE`"), "{}", found[1].message);
}

#[test]
fn non_literal_consts_are_accepted() {
    assert!(clean("const SIZE: f32 = compute();\nconst NAME: &str = \"a\";\nfn f(ui: &mut Ui) { ui.add_space(SIZE); }"));
}

#[test]
fn named_colour_consts() {
    for name in ["WHITE", "BLACK", "RED", "DARK_GRAY", "KHAKI", "LIGHT_BLUE"] {
        for path in ["Color32", "egui::Color32", "ecolor::Color32"] {
            let src = format!("fn f() {{ let _ = {path}::{name}; }}");
            assert!(flagged(&src, "raw-colour/const"), "{path}::{name}");
        }
    }
    assert!(clean("fn f() { let _ = Color32::TRANSPARENT; let _ = Other::WHITE; let _ = Color32::from_rgb(1, 2, 3); }"));
}

#[test]
fn macro_arguments_are_checked() {
    assert!(flagged("fn f() { let _ = vec![vec2(4.0, 4.0)]; }", "literal-size/vec2"));
    assert!(flagged("fn f() { let _ = vec![vec2(4.0, 4.0); 3]; }", "literal-size/vec2"));
    assert!(flagged("fn f() { println!(\"{}\", Color32::WHITE); }", "raw-colour/const"));
    assert!(clean("fn f() { weird!(=> => vec2(4.0, 4.0)); }"));
}

#[test]
fn test_code_is_skipped() {
    let src = "#[cfg(test)]\nmod tests {\n    fn f() { let _ = vec2(4.0, 4.0); }\n}\n#[test]\nfn t() { let _ = vec2(4.0, 4.0); }\n#[cfg(not(test))]\nfn g() { let _ = vec2(4.0, 4.0); }\n";
    let analysis = analyze_source(src, false);
    assert_eq!(analysis.findings.len(), 1);
    assert_eq!(analysis.findings[0].line, 8);
    assert_eq!(analysis.skipped, [1..=4, 5..=6]);
    assert_eq!(analyze_source(src, true).findings.len(), 3);
}

#[test]
fn finding_position_and_message() {
    let found = run("fn f() {\n    let _ = vec2(28.0, 24.0);\n    ui.add_space(4.0);\n}\n");
    assert_eq!((found[0].line, found[0].column), (2, 13));
    assert!(found[0].message.starts_with("`vec2(28.0, 24.0)` uses literal sizes; use a size token"), "{}", found[0].message);
    assert_eq!((found[1].line, found[1].column), (3, 8));
    assert!(found[1].message.starts_with("`add_space(4.0)` uses literal sizes"), "{}", found[1].message);
}

#[test]
fn parse_error_is_a_finding() {
    let found = run("fn f( {");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "parse");
    assert_eq!(found[0].severity, Severity::Error);
}

#[test]
fn check_walks_directories_and_skips_test_files() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-walk");
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, src: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, src).unwrap();
    };
    let bad = "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n";
    write("src/lib.rs", bad);
    write("src/panels/a.rs", bad);
    write("src/panels/a_tests.rs", bad);
    write("src/tests.rs", bad);
    write("src/tests/b.rs", bad);
    write("src/broken.rs", "fn f( {");

    let report = check(&Config::new(["src"]), &root).unwrap();
    let paths: Vec<String> = report.findings.iter().map(|f| f.path.display().to_string()).collect();
    assert_eq!(paths, ["src/broken.rs", "src/lib.rs", "src/panels/a.rs"]);
    assert_eq!(report.files, 3);
    assert_eq!(report.exit_code(), 1);
    let areas: Vec<&str> = report.findings.iter().map(|f| f.area.as_str()).collect();
    assert_eq!(areas, ["(root)", "(root)", "panels"]);

    let mut all = Config::new(["src"]);
    all.include_tests = true;
    assert_eq!(check(&all, &root).unwrap().files, 6);

    write("src/broken.rs", "fn f() {}");
    std::fs::remove_file(root.join("src/lib.rs")).unwrap();
    std::fs::remove_file(root.join("src/panels/a.rs")).unwrap();
    let clean = check(&Config::new(["src"]), &root).unwrap();
    assert_eq!(clean.exit_code(), 0);
    assert_eq!(clean.summary_line(), "design-lint: no findings in 1 file; 0 markers, 0 exemptions");
}

#[test]
fn report_output_format() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("report-format");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/panels")).unwrap();
    std::fs::write(root.join("src/panels/toolbar.rs"), "fn f(ui: &mut Ui) {\n    let _ = vec2(28.0, 24.0);\n    let _ = Color32::WHITE;\n}\n")
        .unwrap();
    let report = check(&Config::new(["src"]), &root).unwrap();
    let out = report.render();
    let mut lines = out.lines();
    assert_eq!(
        lines.next().unwrap(),
        "src/panels/toolbar.rs:2:13: error[literal-size/vec2]: `vec2(28.0, 24.0)` uses literal sizes; use a size token (`Tokens::of(ui).size.control.md`, `space.gap.inline`) or mark `// design: allow(<reason>)`"
    );
    assert_eq!(
        lines.next().unwrap(),
        "src/panels/toolbar.rs:3:13: error[raw-colour/const]: `Color32::WHITE`; take the colour from a semantic token (`Tokens::of(ui).color.text.normal.to_egui()`) or mark `// design: allow(<reason>)`"
    );
    assert_eq!(lines.next().unwrap(), "design-lint: 2 findings in 1 file (literal-size/vec2 1, raw-colour/const 1); 0 markers, 0 exemptions");
    assert_eq!(lines.next(), None);

    let table = report.summary_table();
    assert!(table.contains("literal-size/vec2") && table.contains("panels"), "{table}");
}

#[test]
fn config_parses_and_rejects_unknown_keys() {
    let origin = Path::new("design-lint.toml");
    let config = Config::parse(origin, "[lint]\npaths = [\"a\", \"b/c\"]\ninclude-tests = true\n").unwrap();
    assert_eq!(config.paths, [Path::new("a"), Path::new("b/c")]);
    assert!(config.include_tests);
    assert!(Config::parse(origin, "[lint]\npaths = []\nbogus = 1\n").is_err());
}
