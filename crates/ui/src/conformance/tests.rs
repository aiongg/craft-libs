use std::panic::{AssertUnwindSafe, catch_unwind};

use craft_theme::{InterfaceFont, ModeChoice, Selection, Size};
use egui::{Color32, Context, ThemePreference};

use super::compare::Diff;
use super::report::{Details, Section};
use super::*;

fn no_fonts(_: &Context, _: InterfaceFont) {}

/// The reference path itself, with `size` in place of the case's.
fn setup(ctx: &Context, case: &Case, size: Size) {
    direction::apply(ctx, case.dir());
    let selection = Selection { size, ..case.selection().clone() };
    install(ctx, &Base::resolve(&selection, case.system_appearance(), case.app_defaults(), case.app_id(), &mut case.lookup()));
}

fn reference(ctx: &Context, case: &Case) {
    setup(ctx, case, case.selection().size);
}

fn dark() -> Case {
    Case::new("craft.pro").mode(ModeChoice::Dark)
}

fn expect_report(check: &Conformance<'_>, case: &Case) -> Report {
    check.check(case).expect_err("the app should not conform")
}

#[test]
fn the_matrix_and_axes_have_the_planned_cases() {
    assert_eq!(Case::matrix().len(), 68);
    assert_eq!(Case::axes().len(), 5);
    let mut names: Vec<String> = Case::matrix().iter().chain(&Case::axes()).map(Case::name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 73, "case names are unique");
}

#[test]
fn reference_conforms_to_itself() {
    Conformance::new(reference, no_fonts).assert_all(Case::matrix().into_iter().chain(Case::axes()));
}

#[test]
fn a_style_changed_after_install_is_reported_by_key() {
    let check = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            ctx.global_style_mut(|s| s.visuals.panel_fill = Color32::RED);
        },
        no_fonts,
    );
    let report = expect_report(&check, &dark());
    let keys: Vec<String> = report.keys().collect();
    assert_eq!(keys, ["style (dark): visuals.panel_fill"], "{report}");
    let text = report.to_string();
    assert!(text.contains("tokens (dark): same") && text.contains("ui (dark): same"), "{text}");
    assert!(text.contains("changed the Style after `install`"), "{text}");
}

#[test]
fn a_wrong_selection_is_reported_as_token_differences() {
    let check = Conformance::new(|ctx, case| setup(ctx, case, Size::Comfortable), no_fonts);
    let report = expect_report(&check, &dark());
    let keys: Vec<String> = report.keys().collect();
    assert!(keys.iter().any(|k| k.starts_with("tokens (dark): ")), "{report}");
    assert!(keys.iter().any(|k| k.starts_with("style (dark): ")), "{report}");
    assert!(keys.contains(&"ui (dark): appearance.size".to_owned()), "{report}");
    assert!(report.to_string().contains("resolved a different theme, size"), "{report}");
}

#[test]
fn the_theme_preference_is_compared() {
    let check = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            ctx.set_theme(ThemePreference::System);
        },
        no_fonts,
    );
    let report = expect_report(&check, &dark());
    assert_eq!(report.keys().collect::<Vec<_>>(), ["context: theme_preference"], "{report}");
    assert!(report.to_string().contains("theme_preference  Dark         System"), "{report}");
    assert!(report.to_string().contains("`set_theme`"), "{report}");
}

#[test]
fn a_different_direction_is_compared() {
    let check = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            direction::apply(ctx, crate::direction::Direction::Rtl);
        },
        no_fonts,
    );
    let report = expect_report(&check, &dark());
    let keys: Vec<String> = report.keys().collect();
    assert!(keys.contains(&"context: direction".to_owned()) && keys.contains(&"style (dark): right_to_left".to_owned()), "{report}");
}

fn restyled(ctx: &Context, case: &Case) {
    reference(ctx, case);
    ctx.global_style_mut(|s| s.visuals.panel_fill = Color32::RED);
}

#[test]
fn allowances_hide_the_difference_and_stale_ones_fail() {
    let allowed = Conformance::new(restyled, no_fonts).allow("style.visuals.panel_fill", "tinted panels");
    assert!(allowed.check(&dark()).is_ok());

    // The difference is allowed and listed; a second difference still fails.
    let both = Conformance::new(
        |ctx, case| {
            restyled(ctx, case);
            ctx.set_theme(ThemePreference::System);
        },
        no_fonts,
    )
    .allow("style.visuals.panel_fill", "tinted panels");
    let text = expect_report(&both, &dark()).to_string();
    assert!(text.contains("allowed (1):") && text.contains("style.visuals.panel_fill  \"tinted panels\""), "{text}");
    assert!(text.contains("context: 1 difference"), "{text}");

    let stale = Conformance::new(reference, no_fonts).allow("style.visuals.panel_fill", "tinted panels");
    let text = expect_report(&stale, &dark()).to_string();
    assert!(text.contains("unused allowance: style.visuals.panel_fill"), "{text}");

    // Over many cases an allowance only has to match in one of them.
    let some = Conformance::new(
        |ctx, case| {
            if case.dir() == crate::direction::Direction::Rtl {
                restyled(ctx, case);
            } else {
                reference(ctx, case);
            }
        },
        no_fonts,
    )
    .allow("style.visuals.panel_fill", "tinted panels");
    some.assert_all(Case::axes());
    let failure = catch_unwind(AssertUnwindSafe(|| stale.assert_all(Case::axes()))).expect_err("a stale allowance fails");
    let message = failure.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("unused allowance"), "{message}");
}

#[test]
fn allowances_need_a_reason_and_a_style_or_context_key() {
    let no_reason = catch_unwind(AssertUnwindSafe(|| Conformance::new(reference, no_fonts).allow("style.visuals", " ")));
    assert!(no_reason.is_err());
    let tokens = catch_unwind(AssertUnwindSafe(|| Conformance::new(reference, no_fonts).allow("tokens.color", "mine")));
    assert!(tokens.is_err());
}

#[test]
fn assert_all_lists_each_failed_case_once() {
    let check = Conformance::new(restyled, no_fonts);
    let failure = catch_unwind(AssertUnwindSafe(|| check.assert_all([dark(), Case::new("craft.pro")]))).expect_err("both cases differ");
    let message = failure.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.starts_with("conformance: 2 of 2 cases failed"), "{message}");
    assert!(message.contains("conformance failed: pro-dark-compact") && message.contains("conformance failed: pro-light-compact"), "{message}");
}

fn diff(key: &str, reference: &str, app: &str) -> Diff {
    Diff { key: key.to_owned(), reference: Some(reference.to_owned()), app: Some(app.to_owned()) }
}

fn report(sections: Vec<Section>) -> Report {
    Report::new(Details {
        case: "pro-dark-high-compact".to_owned(),
        mode: "mode dark, shown dark".to_owned(),
        sections,
        allowed: vec![("style.visuals.striped".to_owned(), "PhotoCraft tables stripe rows".to_owned(), 1)],
        unused: Vec::new(),
        hints: vec!["the app changed the Style after `install`.".to_owned()],
    })
}

#[test]
fn failure_text_is_stable() {
    let style = Section {
        title: "style (dark)".to_owned(),
        diffs: vec![
            diff("visuals.panel_fill", "#1a1a1cff", "#202020ff"),
            diff("spacing.item_spacing", "8 4", "8 6"),
            Diff { key: "spacing.scroll.floating_width".to_owned(), reference: Some("6".to_owned()), app: None },
        ],
    };
    let same = Section { title: "tokens (dark)".to_owned(), diffs: Vec::new() };
    let expected = "\
conformance failed: pro-dark-high-compact (mode dark, shown dark)
style (dark): 3 differences
  visuals.panel_fill             #1a1a1cff    #202020ff
  spacing.item_spacing           8 4          8 6
  spacing.scroll.floating_width  6            <missing>
                                 ^ reference  ^ app
tokens (dark): same
allowed (1):
  style.visuals.striped  \"PhotoCraft tables stripe rows\"  (1 key)
hint: the app changed the Style after `install`.
";
    assert_eq!(report(vec![style, same]).to_string(), expected);

    let many = Section { title: "style (dark)".to_owned(), diffs: (0..32).map(|i| diff(&format!("k{i:02}"), "1", "2")).collect() };
    let text = report(vec![many]).to_string();
    assert!(text.contains("style (dark): 32 differences\n"), "{text}");
    assert!(text.contains("  k29  1            2\n"), "{text}");
    assert!(!text.contains("k30") && text.contains("  … 2 more\n"), "{text}");
}
