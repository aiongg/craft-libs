//! Integration tests of craft-design-lint: rules.

#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::Path;

use craft_design_lint::{Config, Finding, analyze_source, check};

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
fn same_file_consts_resolve() {
    let src = "const GAP: f32 = 4.0;\nconst DOUBLE: f32 = GAP * 2.0;\nstatic NONE: f32 = 0.0;\nfn f(ui: &mut Ui) {\n    ui.add_space(GAP);\n    ui.add_space(DOUBLE);\n    ui.add_space(NONE);\n    ui.add_space(OTHER);\n}\n";
    let found = run(src);
    assert_eq!(found.len(), 2);
    assert_eq!((found[0].line, found[1].line), (5, 6));
    assert!(found[0].message.contains("`GAP` (const, line 1)"), "{}", found[0].message);
    assert!(found[1].message.contains("`DOUBLE`"), "{}", found[1].message);
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
    assert_eq!(clean.summary_line(), "design-lint: no findings in 1 file; 0 markers, 0 exemptions, 0 clippy allows");
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
    assert_eq!(
        lines.next().unwrap(),
        "design-lint: 2 findings in 1 file (literal-size/vec2 1, raw-colour/const 1); 0 markers, 0 exemptions, 0 clippy allows"
    );
    assert_eq!(lines.next(), None);

    let table = report.summary_table();
    assert!(table.contains("literal-size/vec2") && table.contains("panels"), "{table}");
}

#[test]
fn stroke_tuples_are_checked() {
    for src in [
        "fn f(p: &Painter) { p.line_segment([a, b], (1.0, c)); }",
        "fn f(p: &Painter) { p.rect_stroke(r, radius, (1.5, c), kind); }",
        "fn f() { let _ = Frame::new().stroke((1.0, c)); }",
        "fn f() { let _: Stroke = (1.0, c).into(); }",
        "fn f() { let _ = Stroke::from((2.0, c)); }",
    ] {
        assert!(flagged(src, "literal-size/stroke"), "{src}");
    }
    assert!(clean(
        "fn f(p: &Painter) { p.line_segment([a, b], (w, c)); let _ = Frame::new().stroke((0.0, c)); let _ = (1.0, 2.0); let _ = pair((1.0, c)); }"
    ));
}

#[test]
fn consts_resolve_by_module_and_impl_scope() {
    // A module-level const of the same name in another module is a different const.
    let src = "mod a { const GAP: f32 = 0.0; fn f(ui: &mut Ui) { ui.add_space(GAP); } }\nmod b { const GAP: f32 = 4.0; fn g(ui: &mut Ui) { ui.add_space(GAP); } }\n";
    let found = run(src);
    assert_eq!(found.iter().map(|f| f.line).collect::<Vec<_>>(), [2]);

    // Shadowing: the innermost scope wins, so a zero outer const does not hide a non-zero inner one.
    let src =
        "const GAP: f32 = 0.0;\nmod m { const GAP: f32 = 4.0; fn f(ui: &mut Ui) { ui.add_space(GAP); } }\nfn g(ui: &mut Ui) { ui.add_space(GAP); }\n";
    assert_eq!(run(src).iter().map(|f| f.line).collect::<Vec<_>>(), [2]);

    // Self::X and Type::X name the impl's associated const; the message shows the path as written.
    let src = "struct A; struct B;\nimpl A { const PAD: f32 = 6.0; fn f(&self, ui: &mut Ui) { ui.add_space(Self::PAD); } }\nimpl B { const PAD: f32 = 0.0; fn f(&self, ui: &mut Ui) { ui.add_space(Self::PAD); ui.add_space(A::PAD); } }\n";
    let found = run(src);
    assert_eq!(found.iter().map(|f| f.line).collect::<Vec<_>>(), [2, 3]);
    assert!(found[0].message.contains("`Self::PAD` (const, line 2)"), "{}", found[0].message);
    assert!(found[1].message.contains("`A::PAD`"), "{}", found[1].message);

    // mod::X, self::X, super::X and crate::X.
    let src = "const TOP: f32 = 1.0;\nmod sizes { pub const BIG: f32 = 9.0; }\nmod ui { fn f(ui: &mut Ui) { ui.add_space(super::TOP); ui.add_space(crate::TOP); ui.add_space(crate::sizes::BIG); ui.add_space(self::NONE); } }\nconst NONE: f32 = 0.0;\n";
    assert_eq!(run(src).len(), 3);
}

#[test]
fn an_ambiguous_const_is_flagged_when_any_candidate_is_non_zero() {
    let src = "fn a(ui: &mut Ui) { const SIZE: f32 = 0.0; ui.add_space(SIZE); }\nfn b(ui: &mut Ui) { const SIZE: f32 = 8.0; ui.add_space(SIZE); }\n";
    assert_eq!(run(src).len(), 2);
    let zero = "fn a(ui: &mut Ui) { const SIZE: f32 = 0.0; ui.add_space(SIZE); }\nfn b(ui: &mut Ui) { const SIZE: f32 = 0.0; ui.add_space(SIZE); }\n";
    assert!(clean(zero));
}

#[test]
fn clippy_allow_attributes_are_counted_and_inner_ones_flagged() {
    let src = "#![allow(clippy::disallowed_methods, reason = \"stock widgets\")]\n#[expect(clippy::disallowed_types)]\nstruct S;\n#[allow(clippy::too_many_arguments)]\nfn f() {}\nmod m {\n    #![expect(clippy::disallowed_methods)]\n}\n";
    let analysis = analyze_source(src, false);
    assert_eq!(analysis.clippy_allows, 3);
    assert_eq!(analysis.findings.iter().map(|f| (f.rule, f.line)).collect::<Vec<_>>(), [("clippy/inner-allow", 1), ("clippy/inner-allow", 7)]);
    assert!(analysis.findings[0].message.contains("[[exempt]]"), "{}", analysis.findings[0].message);
}

#[test]
fn imported_names_resolve_through_use_items() {
    let margin = "literal-size/margin";
    for (src, rule) in [
        ("use craft_ui::theme::margin;\nfn f() { let _ = margin(4.0); }", margin),
        ("use craft_ui::theme::{margin, stroke};\nfn f() { let _ = margin(4.0); }", margin),
        ("use craft_ui::theme::{margin, stroke};\nfn f() { let _ = stroke(1.0, c); }", "literal-size/stroke"),
        ("use craft_ui::theme::margin as m;\nfn f() { let _ = m(4.0); }", margin),
        ("use craft_ui::theme::{self as t, corner_radius as cr};\nfn f() { let _ = t::margin(4.0); }", margin),
        ("use craft_ui::theme::{self as t, corner_radius as cr};\nfn f() { let _ = cr(4.0); }", "literal-size/corner-radius"),
        ("use craft_ui::theme::*;\nfn f() { let _ = margin(4.0); }", margin),
        ("use craft_ui::theme;\nfn f() { let _ = theme::margin(4.0); }", margin),
        ("use craft_ui::{theme::margin_x};\nfn f() { let _ = margin_x(4.0, 0.0); }", margin),
        ("use craft_ui::*;\nfn f() { let _ = theme::margin(4.0); }", margin),
        ("use egui::Margin as M;\nfn f() { let _ = M::from(4.0); }", margin),
        ("use egui::Margin as M;\nfn f() { let _ = M { left: 4, right: 0, top: 0, bottom: 0 }; }", margin),
        ("use egui::vec2 as v;\nfn f() { let _ = v(4.0, 4.0); }", "literal-size/vec2"),
        ("use egui::{Vec2 as V};\nfn f() { let _ = V::splat(4.0); }", "literal-size/vec2"),
        ("use egui::Color32 as C;\nfn f() { let _ = C::WHITE; }", "raw-colour/const"),
        ("use egui::Color32::WHITE;\nfn f() { let _ = WHITE; }", "raw-colour/const"),
        ("mod m { use craft_ui::theme::margin; fn f() { let _ = margin(4.0); } }", margin),
    ] {
        assert!(flagged(src, rule), "{src}: {:?}", rules(src));
    }
}

#[test]
fn local_fns_and_unimported_names_stay_unflagged() {
    assert!(clean("fn margin(px: f32) -> f32 { px }\nfn f() { let _ = margin(4.0); }"));
    assert!(clean("use craft_ui::theme::*;\nfn margin(px: f32) -> f32 { px }\nfn f() { let _ = margin(4.0); }"));
    assert!(clean("fn f() { let _ = margin(4.0); let _ = m(4.0); }"));
    assert!(clean("use other::margin;\nuse craft_ui::theme::stroke;\nfn f() { let _ = margin(4.0); }"));
    // Imports are scoped to their module.
    assert!(clean("mod a { use craft_ui::theme::margin; }\nmod b { fn f() { let _ = margin(4.0); } }"));
    assert!(clean("use craft_ui::theme::margin as m;\nfn f() { let _ = margin(4.0); }"));
    // A zero literal is still accepted through an import.
    assert!(clean("use craft_ui::theme::margin;\nfn f() { let _ = margin(0.0); let _ = margin(gap); }"));
}
