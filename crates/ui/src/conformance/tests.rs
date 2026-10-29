use std::panic::{AssertUnwindSafe, catch_unwind};

use craft_theme::{InterfaceFont, ModeChoice, Selection, Size};
use egui::{Color32, Context, ThemePreference};

use super::compare::{Diff, Side};
use super::report::{Details, Section};
use super::*;
use crate::fonts::{Family, I18nScripts, SystemFonts, UiFonts};

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
    assert_eq!(Case::axes().len(), 9);
    let mut names: Vec<String> = Case::matrix().iter().chain(&Case::axes()).map(Case::name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 77, "case names are unique");
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
fn allowances_cover_their_key_and_the_keys_under_it() {
    let allowance = |prefix: &str| Allowance { prefix: prefix.to_owned(), reason: "r".to_owned() };
    let scroll = allowance("style.spacing.scroll");
    assert!(scroll.covers("style.spacing.scroll") && scroll.covers("style.spacing.scroll.bar_width"));
    assert!(!scroll.covers("style.spacing.scroll_other") && !scroll.covers("style.spacing"));

    // A longer key sharing the prefix is not hidden by the allowance.
    let check = Conformance::new(restyled, no_fonts).allow("style.visuals.panel", "tinted panels");
    let text = expect_report(&check, &dark()).to_string();
    assert!(text.contains("style (dark): 1 difference") && text.contains("unused allowance: style.visuals.panel"), "{text}");
}

#[test]
fn allowances_reject_bare_sections_and_trailing_dots() {
    for key in ["style.", "context.", "style.visuals.", "style", "context"] {
        let result = catch_unwind(AssertUnwindSafe(|| Conformance::new(reference, no_fonts).allow(key, "mine")));
        assert!(result.is_err(), "`{key}` should be rejected");
    }
    Conformance::new(reference, no_fonts).allow("style.visuals", "mine").allow("context.zoom_factor", "mine");
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

#[test]
fn only_the_interact_size_differing_points_at_the_font_stack() {
    let check = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            ctx.global_style_mut(|s| s.spacing.interact_size.y += 3.0);
        },
        no_fonts,
    );
    let report = expect_report(&check, &dark());
    assert_eq!(report.keys().collect::<Vec<_>>(), ["style (dark): spacing.interact_size"], "{report}");
    let text = report.to_string();
    assert!(text.contains("only `spacing.interact_size` differs") && text.contains("fit_interact_size"), "{text}");
    assert!(!text.contains("changed the Style after `install`"), "{text}");
}

#[test]
fn converted_values_alone_point_at_the_font_stack() {
    let check = Conformance::new(
        |ctx, case| {
            UiFonts::new(&I18nScripts)
                .app_face("ui", egui::FontData::from_static(&[]), &[egui::FontFamily::Proportional])
                .app_face("semibold", egui::FontData::from_static(&[]), &[egui::FontFamily::Name(crate::fonts::SEMIBOLD.into())])
                .family(Family::named(crate::fonts::SEMIBOLD).weight(600))
                .system_fonts(SystemFonts::Off)
                .install(ctx);
            reference(ctx, case);
        },
        no_fonts,
    );
    let report = expect_report(&check, &dark());
    let keys: Vec<String> = report.keys().collect();
    assert!(keys.contains(&"ui (dark): families.ui_semibold".to_owned()), "{report}");
    assert!(!keys.iter().any(|k| k.starts_with("tokens")), "{report}");
    let text = report.to_string();
    assert!(text.contains("the tokens match but the converted values differ"), "{text}");
}

#[test]
fn the_context_settings_the_style_does_not_hold_are_compared() {
    let zoom = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            ctx.set_zoom_factor(1.25);
        },
        no_fonts,
    );
    assert_eq!(expect_report(&zoom, &dark()).keys().collect::<Vec<_>>(), ["context: zoom_factor"]);

    let overrides = Conformance::new(
        |ctx, case| {
            reference(ctx, case);
            ctx.global_style_mut(|s| {
                s.override_font_id = Some(egui::FontId::monospace(20.0));
                s.override_text_style = Some(egui::TextStyle::Heading);
            });
        },
        no_fonts,
    );
    let keys: Vec<String> = expect_report(&overrides, &dark()).keys().collect();
    assert_eq!(keys, ["style (dark): override_font_id", "style (dark): override_text_style"]);
}

/// The reference stack is checked against the case's interface font: the system face leads the
/// stack only where the OS has one.
#[test]
fn the_reference_font_stack_follows_the_case_font() {
    let system_face = crate::fonts::system_ui_face().is_some();
    let installs_system = |ctx: &Context, _: InterfaceFont| {
        UiFonts::new(&I18nScripts)
            .app_face("ui", egui::FontData::from_static(&[]), &[egui::FontFamily::Proportional])
            .interface_font(InterfaceFont::System)
            .install(ctx);
    };
    // The case asks for the suite font; a `fonts` setup that installs the system face anyway
    // differs wherever that face exists.
    let ignores_font = Conformance::new(reference, installs_system);
    let case = Case::new("craft.pro");
    if system_face {
        let report = ignores_font.check(&case).expect_err("the stack ignores the case font");
        assert!(report.keys().any(|k| k == "fonts: families.optical_size"), "{report}");
        assert!(report.to_string().contains("does not follow the case's interface font"), "{report}");
    } else {
        ignores_font.check(&case).expect("no system face, nothing to differ");
    }
}

/// An app whose build embeds faces must install a stack; one without embedded faces need not.
#[test]
fn a_build_with_embedded_faces_must_install_a_font_stack() {
    let case = Case::new("craft.pro");
    let report = expect_report(&Conformance::new(reference, no_fonts).faces_embedded(true), &case);
    assert!(report.keys().any(|k| k == "fonts: families.installed"), "{report}");
    assert!(report.to_string().contains("no font stack installed"), "{report}");
    Conformance::new(reference, no_fonts).check(&case).expect("no embedded faces, no stack to expect");
}

#[test]
fn row_keys_are_unique_in_every_group() {
    let ctx = Context::default();
    reference(&ctx, &dark());
    // A named text style that differs from a built-in one only by case.
    ctx.global_style_mut(|s| {
        s.text_styles.insert(egui::TextStyle::Name("Body".into()), egui::FontId::proportional(13.0));
        s.text_styles.insert(egui::TextStyle::Name("body".into()), egui::FontId::proportional(14.0));
    });
    let side = Side::read(&ctx, 1);
    let unique = |what: &str, rows: &[(String, String)]| {
        let mut keys: Vec<&str> = rows.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before, "{what}: a key names more than one row");
    };
    for m in &side.modes {
        unique("tokens", &m.tokens);
        unique("ui", &m.ui);
        unique("style", &m.style);
    }
    unique("context", &side.context);
    assert!(side.modes.iter().any(|m| m.style.iter().any(|(k, _)| k == "text_styles.name.Body")));
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

#[test]
fn the_excerpt_keeps_the_hints() {
    let many = Section { title: "style (dark)".to_owned(), diffs: (0..32).map(|i| diff(&format!("k{i:02}"), "1", "2")).collect() };
    let excerpt = report(vec![many]).excerpt(10);
    assert!(excerpt.contains("… ") && excerpt.contains(" more lines\n"), "{excerpt}");
    assert!(!excerpt.contains("k29"), "{excerpt}");
    assert!(excerpt.ends_with("hint: the app changed the Style after `install`."), "{excerpt}");
}
