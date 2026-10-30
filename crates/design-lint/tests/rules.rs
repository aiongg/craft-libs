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
fn config_parses_and_rejects_unknown_keys() {
    let origin = Path::new("design-lint.toml");
    let config = Config::parse(origin, "[lint]\npaths = [\"a\", \"b/c\"]\ninclude-tests = true\n").unwrap();
    assert_eq!(config.paths, [Path::new("a"), Path::new("b/c")]);
    assert!(config.include_tests);
    assert!(Config::parse(origin, "[lint]\npaths = []\nbogus = 1\n").is_err());
}

#[test]
fn craft_ui_theme_helpers_are_checked() {
    for (src, rule) in [
        ("fn f() { let _ = theme::margin(8.0); }", "literal-size/margin"),
        ("fn f() { let _ = craft_ui::theme::margin_x(8.0, 0.0); }", "literal-size/margin"),
        ("fn f() { let _ = theme::corner_radius(4.0); }", "literal-size/corner-radius"),
        ("fn f() { let _ = theme::stroke(1.0, c); }", "literal-size/stroke"),
    ] {
        assert!(flagged(src, rule), "{src}");
    }
    assert!(clean("fn f() { let _ = theme::margin(space.pad.panel); let _ = theme::margin_x(0.0, 0.0); let _ = theme::stroke(w, c); }"));
}

#[test]
fn painter_radii_are_checked() {
    for src in [
        "fn f(p: &Painter) { p.rect_filled(r, 4.0, c); }",
        "fn f(p: &Painter) { p.rect_stroke(r, 4, s, kind); }",
        "fn f(p: &Painter) { p.rect(r, 2.0, c, s, kind); }",
        "fn f() { let _ = Shape::rect_filled(r, 3.0, c); }",
        "fn f() { let _ = RectShape::new(r, 3, c, s, kind); }",
    ] {
        assert!(flagged(src, "literal-size/corner-radius"), "{src}");
    }
    assert!(clean("fn f(p: &Painter) { p.rect_filled(r, radius, c); p.rect_filled(r, 0.0, c); p.rect_filled(r, CornerRadius::ZERO, c); }"));
}

#[test]
fn rich_text_size_is_checked() {
    assert!(flagged("fn f() { let _ = RichText::new(t).size(13.0); }", "literal-size/font-id"));
    assert!(flagged("fn f() { let _ = egui::RichText::new(t).strong().size(13.0); }", "literal-size/font-id"));
    assert!(clean("fn f() { let _ = RichText::new(t).size(size); let _ = Spinner::new().size(16.0); }"));
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
fn margin_from_and_rgba_colours() {
    assert!(flagged("fn f() { let _ = Margin::from(4.0); }", "literal-size/margin"));
    for name in ["WHITE", "BLACK", "RED", "GREEN", "BLUE"] {
        assert!(flagged(&format!("fn f() {{ let _ = Rgba::{name}; }}"), "raw-colour/const"), "{name}");
    }
    assert!(clean("fn f() { let _ = Rgba::TRANSPARENT; let _ = Rgba::from_rgb_unmultiplied(1, 2, 3); }"));
}

#[test]
fn style_mutation_is_flagged() {
    for call in [
        "ui.style_mut().spacing.item_spacing.x = 3.0",
        "ctx.style_mut(|s| s.animation_time = 0.0)",
        "ui.spacing_mut().button_padding = x",
        "ui.visuals_mut().override_text_color = None",
        "ctx.set_zoom_factor(1.5)",
        "ui.style_mut().override_font_id = None",
        "ui.set_style(style)",
        "ui.ctx().set_visuals(v)",
    ] {
        let found = rules(&format!("fn f(ui: &mut Ui, ctx: &Context) {{ {call}; }}"));
        assert!(found.contains(&"style-mutation"), "{call}: {found:?}");
    }
    let zoom = run("fn f(ctx: &Context) { ctx.set_zoom_factor(1.5); }");
    assert!(zoom.iter().any(|f| f.message.contains("InterfaceScale::apply")), "{zoom:?}");
    let src = "fn f(ui: &mut Ui) { ui.override_font_id(id); }";
    assert!(flagged(src, "style-mutation"));
    assert!(clean("fn f(ui: &Ui) { let _ = ui.style(); let _ = ui.spacing(); let _ = ctx.zoom_factor(); }"));
}

#[test]
fn stock_tooltips_name_the_tooltip_ext_replacement() {
    for (call, replacement) in [
        ("r.on_hover_text(\"x\")", "TooltipExt::tooltip"),
        ("r.on_disabled_hover_text(\"x\")", "TooltipExt::tooltip"),
        ("r.on_hover_ui(|ui| {})", "TooltipExt::tooltip_ui"),
        ("r.on_disabled_hover_ui(|ui| {})", "TooltipExt::tooltip_ui"),
        ("r.on_hover_text_at_pointer(\"x\")", "TooltipExt::tooltip_at_pointer"),
        ("r.on_hover_ui_at_pointer(|ui| {})", "TooltipExt::tooltip_ui_at_pointer"),
        ("r.show_tooltip_text(\"x\")", "TooltipExt::show_tooltip"),
        ("r.show_tooltip_ui(|ui| {})", "TooltipExt::show_tooltip_ui"),
        ("r.clone().on_hover_text(\"x\").on_hover_cursor(c)", "TooltipExt::tooltip"),
    ] {
        let found = run(&format!("fn f(r: Response) {{ {call}; }}"));
        let hit = found.iter().find(|f| f.rule == "tooltip/stock").unwrap_or_else(|| panic!("{call}: {found:?}"));
        assert!(hit.message.contains(replacement), "{call}: {}", hit.message);
    }
}

#[test]
fn stock_tooltip_construction_is_flagged_and_craft_ui_calls_are_not() {
    assert!(flagged("fn f(r: &Response) { egui::Tooltip::for_enabled(r).show(|ui| {}); }", "tooltip/stock"));
    assert!(flagged("use egui::Tooltip; fn f(r: &Response) { Tooltip::for_disabled(r); }", "tooltip/stock"));
    assert!(flagged("use egui::Tooltip; fn f() { Tooltip::always_open(a, b, c, d); }", "tooltip/stock"));
    assert!(clean(
        "fn f(r: Response, t: &UiTokens) { r.tooltip(\"x\"); r.tooltip_ui(|ui| {}); r.show_tooltip(\"x\"); let _ = Tooltip::style(t); let _ = r.is_tooltip_open(); }"
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
fn consts_in_test_modules_do_not_resolve() {
    let src = "#[cfg(test)]\nmod tests { const GAP: f32 = 4.0; }\nfn f(ui: &mut Ui) { ui.add_space(GAP); }\n";
    assert!(clean(src));
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
fn clippy_allows_in_test_code_are_not_counted() {
    let src = "#[cfg(test)]\nmod tests {\n    #![allow(clippy::disallowed_methods)]\n}\n";
    let analysis = analyze_source(src, false);
    assert_eq!((analysis.clippy_allows, analysis.findings.len()), (0, 0));
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
