#![allow(clippy::unwrap_used, reason = "tests")]

use craft_design_lint::{Finding, Severity, analyze_with_markers};

fn run(src: &str) -> (Vec<Finding>, usize) {
    let marked = analyze_with_markers(src, false, false, &|_| false);
    (marked.findings, marked.markers)
}

fn rules(src: &str) -> Vec<&'static str> {
    run(src).0.iter().map(|f| f.rule).collect()
}

#[test]
fn end_of_line_marker_covers_its_line_only() {
    let src = "fn f(ui: &mut Ui) {\n    ui.add_space(4.0); // design: allow(optical gap)\n    ui.add_space(8.0);\n}\n";
    let (findings, markers) = run(src);
    assert_eq!(markers, 1);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 3);
}

#[test]
fn alone_marker_covers_a_multi_line_statement() {
    let src =
        "fn f(ui: &mut Ui) {\n    // design: allow(specimen)\n    let size = vec2(\n        4.0,\n        5.0,\n    );\n    ui.add_space(8.0);\n}\n";
    let (findings, _) = run(src);
    assert_eq!(findings.iter().map(|f| (f.rule, f.line)).collect::<Vec<_>>(), [("literal-size/add-space", 7)]);
}

#[test]
fn alone_marker_covers_a_whole_fn_and_skips_comment_lines() {
    let src = "// design: allow(canvas units)\n// more on the fn\n#[inline]\nfn f(ui: &mut Ui) {\n    ui.add_space(4.0);\n    let _ = Color32::WHITE;\n}\nfn g(ui: &mut Ui) { ui.add_space(1.0); }\n";
    let (findings, _) = run(src);
    assert_eq!(findings.iter().map(|f| f.line).collect::<Vec<_>>(), [8]);
}

#[test]
fn alone_marker_covers_an_impl_item_and_a_match_arm() {
    let src = "impl A {\n    // design: allow(fixed)\n    fn f(&self, ui: &mut Ui) { ui.add_space(4.0); }\n    fn g(&self, ui: &mut Ui) {\n        match x {\n            // design: allow(arm)\n            1 => ui.add_space(2.0),\n            _ => ui.add_space(3.0),\n        }\n    }\n}\n";
    let (findings, _) = run(src);
    assert_eq!(findings.iter().map(|f| f.line).collect::<Vec<_>>(), [8]);
}

#[test]
fn file_level_marker_covers_everything() {
    let src = "//! design: allow(specimen file)\nfn f(ui: &mut Ui) { ui.add_space(4.0); let _ = Color32::RED; }\n";
    let (findings, markers) = run(src);
    assert!(findings.is_empty(), "{findings:#?}");
    assert_eq!(markers, 1);
}

#[test]
fn slashes_inside_strings_are_not_markers() {
    let src = "fn f(ui: &mut Ui) {\n    let s = \"// design: allow()\";\n    let r = r#\"\n// design: allow(in a raw string)\n\"#;\n    ui.add_space(4.0); // design: allow(real) \n}\n";
    let (findings, markers) = run(src);
    assert_eq!(markers, 1);
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn doc_comments_are_not_markers() {
    let src = "/// design: allow(not a marker)\nfn f(ui: &mut Ui) { ui.add_space(4.0); }\n";
    assert_eq!(rules(src), ["literal-size/add-space"]);
}

#[test]
fn empty_and_short_reasons_are_reported_and_do_not_suppress() {
    let src = "fn f(ui: &mut Ui) {\n    ui.add_space(4.0); // design: allow()\n    ui.add_space(5.0); // design: allow(  )\n    ui.add_space(6.0); // design: allow(ab)\n}\n";
    let (findings, markers) = run(src);
    assert_eq!(markers, 3);
    assert_eq!(findings.iter().filter(|f| f.rule == "marker/empty").count(), 3);
    assert_eq!(findings.iter().filter(|f| f.rule == "literal-size/add-space").count(), 3);
    assert!(findings.iter().all(|f| f.severity == Severity::Error));
}

#[test]
fn unused_marker_is_a_warning_or_an_error_when_denied() {
    let src = "fn f() {\n    let x = 1; // design: allow(nothing here)\n}\n";
    let (findings, _) = run(src);
    assert_eq!(findings.len(), 1);
    assert_eq!((findings[0].rule, findings[0].line, findings[0].severity), ("marker/unused", 2, Severity::Warning));
    let denied = analyze_with_markers(src, false, true, &|_| false);
    assert_eq!(denied.findings[0].severity, Severity::Error);
}

#[test]
fn alone_marker_with_no_code_after_it_is_unused() {
    assert_eq!(rules("fn f() {}\n// design: allow(dangling)\n"), ["marker/unused"]);
}

#[test]
fn marker_in_a_skipped_test_range_is_not_unused() {
    let src = "fn f() {}\n#[cfg(test)]\nmod tests {\n    fn t(ui: &mut Ui) {\n        ui.add_space(4.0); // design: allow(fixture)\n    }\n}\n";
    let (findings, markers) = run(src);
    assert!(findings.is_empty(), "{findings:#?}");
    assert_eq!(markers, 1);
    let included = analyze_with_markers(src, true, false, &|_| false);
    assert!(included.findings.is_empty(), "{:#?}", included.findings);
}

#[test]
fn marker_covering_only_exempt_findings_is_unused() {
    let src = "fn f(ui: &mut Ui) {\n    ui.add_space(4.0); // design: allow(gap)\n}\n";
    let marked = analyze_with_markers(src, false, false, &|rule| rule.starts_with("literal-size"));
    assert_eq!(marked.findings.iter().map(|f| f.rule).collect::<Vec<_>>(), ["marker/unused"]);
}

#[test]
fn allow_attributes_do_not_silence_syntax_rules() {
    let src = "#[allow(clippy::disallowed_methods, reason = \"stock widget\")]\nfn f(ui: &mut Ui) { ui.add_space(4.0); }\n";
    assert_eq!(rules(src), ["literal-size/add-space"]);
}
