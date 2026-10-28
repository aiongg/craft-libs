#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{APP_DEFAULT, SUITE_DEFAULT, ThemeId};

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

fn builtin(s: &str) -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == s).unwrap().clone()
}

fn lookup(id: &ThemeId) -> Option<ThemeDef> {
    builtins().iter().find(|t| &t.id == id).cloned()
}

fn defaults() -> AppDefaults {
    AppDefaults { theme: ThemePair { light: id("craft.pro-light"), dark: id("craft.pro-dark") }, mode: ModeChoice::Dark, size: Size::Compact }
}

fn selection(light: &str, dark: &str, mode: ModeChoice) -> Selection {
    Selection {
        theme: ThemePair { light: id(light), dark: id(dark) },
        mode,
        contrast: ContrastChoice::System,
        size: Size::Compact,
        text_size: TextSize::Default,
    }
}

fn show(sel: &Selection, system: SystemAppearance) -> Shown {
    resolve_selection(sel, system, Mode::Light, &defaults(), None, &mut lookup)
}

#[test]
fn picking_a_single_variant_theme_sets_only_its_slot() {
    let mut suite = SuiteChoice::default();
    suite.pick_theme(&builtin("craft.solarized-dark"));
    assert_eq!((suite.theme_light.as_ref(), suite.theme_dark.as_ref()), (None, Some(&id("craft.solarized-dark"))));
    suite.pick_theme(&builtin("craft.classic"));
    assert_eq!((suite.theme_light.as_ref(), suite.theme_dark.as_ref()), (Some(&id("craft.classic")), Some(&id("craft.solarized-dark"))));
    suite.pick_theme(&builtin("craft.studio"));
    assert_eq!((suite.theme_light.as_ref(), suite.theme_dark.as_ref()), (Some(&id("craft.studio")), Some(&id("craft.studio"))));

    let mut pair = defaults().theme;
    pair.pick(&builtin("craft.soft-light"));
    assert_eq!(pair, ThemePair { light: id("craft.soft-light"), dark: id("craft.pro-dark") });
    pair.pick(&builtin("craft.graphite"));
    assert_eq!(pair, ThemePair::same(id("craft.graphite")));
}

#[test]
fn app_choice_over_suite_over_defaults_per_field() {
    let defaults = defaults();
    let empty = SuiteChoice::default();
    assert!(empty.is_empty());
    assert_eq!(
        empty.with_defaults(&defaults),
        Selection {
            theme: defaults.theme.clone(),
            mode: ModeChoice::Dark,
            contrast: ContrastChoice::System,
            size: Size::Compact,
            text_size: TextSize::Default
        }
    );

    let suite = SuiteChoice {
        theme_dark: Some(id("craft.studio")),
        contrast: Some(ContrastChoice::High),
        text_size: Some(TextSize::Large),
        ..SuiteChoice::default()
    };
    let followed = suite.with_defaults(&defaults);
    assert_eq!(
        followed,
        Selection {
            theme: ThemePair { light: id("craft.pro-light"), dark: id("craft.studio") },
            mode: ModeChoice::Dark,
            contrast: ContrastChoice::High,
            size: Size::Compact,
            text_size: TextSize::Large,
        }
    );
    assert_eq!(AppChoice::FOLLOW_SUITE.apply(&followed), followed);

    let own_size = AppChoice { theme: None, size: Some(Size::Comfortable) };
    assert_eq!(own_size.apply(&followed), Selection { size: Size::Comfortable, ..followed.clone() });
    let own_theme = AppChoice { theme: Some(ThemePair::same(id("craft.breeze"))), size: None };
    assert_eq!(own_theme.apply(&followed), Selection { theme: ThemePair::same(id("craft.breeze")), ..followed.clone() });

    let full = followed.clone();
    assert_eq!(SuiteChoice::from(full.clone()).with_defaults(&defaults), full);
}

#[test]
fn system_choices_take_the_os_answer_else_the_fallbacks() {
    let mut sel = selection("craft.graphite", "craft.graphite", ModeChoice::System);
    let os = SystemAppearance { mode: Some(Mode::Dark), contrast: Some(Contrast::High), reduced_motion: Some(true) };
    let shown = show(&sel, os);
    assert_eq!((shown.appearance.mode, shown.appearance.contrast, shown.appearance.reduced_motion), (Mode::Dark, Contrast::High, true));
    assert!(shown.notices.is_empty());
    assert_eq!(shown.resolved.tokens.motion.hover.millis, 0);

    let shown = show(&sel, SystemAppearance::default());
    assert_eq!((shown.appearance.mode, shown.appearance.contrast, shown.appearance.reduced_motion), (Mode::Light, Contrast::Standard, false));
    assert_eq!(shown.notices, [Fallback::SystemSilent]);

    sel.mode = ModeChoice::Dark;
    sel.contrast = ContrastChoice::Standard;
    let shown = show(&sel, os);
    assert_eq!((shown.appearance.mode, shown.appearance.contrast), (Mode::Dark, Contrast::Standard));
}

#[test]
fn theme_falls_back_to_the_app_default_then_the_suite_default() {
    let shown = show(&selection("craft.studio", "craft.solarized-dark", ModeChoice::Dark), SystemAppearance::default());
    assert_eq!((shown.theme.as_str(), shown.notices.as_slice()), ("craft.solarized-dark", &[][..]));
    assert_eq!(shown.resolved, resolve(&builtin("craft.solarized-dark"), shown.appearance, None));

    let shown = show(&selection("craft.studio", APP_DEFAULT, ModeChoice::Dark), SystemAppearance::default());
    assert_eq!((shown.theme.as_str(), shown.notices.as_slice()), ("craft.pro-dark", &[][..]));

    let shown = show(&selection("user.gone", "craft.studio", ModeChoice::Light), SystemAppearance::default());
    assert_eq!(shown.theme.as_str(), "craft.pro-light");
    assert_eq!(shown.notices, [Fallback::UnknownTheme(id("user.gone"))]);

    let broken = AppDefaults { theme: ThemePair::same(id("app.missing")), ..defaults() };
    let sel = selection("user.gone", "user.gone", ModeChoice::Light);
    let shown = resolve_selection(&sel, SystemAppearance::default(), Mode::Light, &broken, None, &mut lookup);
    assert_eq!(shown.theme.as_str(), SUITE_DEFAULT);
    assert_eq!(shown.notices, [Fallback::UnknownTheme(id("user.gone")), Fallback::UnknownTheme(id("app.missing"))]);

    let sel = selection(APP_DEFAULT, APP_DEFAULT, ModeChoice::Light);
    let none = AppDefaults { theme: ThemePair::same(ThemeId::app_default()), ..defaults() };
    let shown = resolve_selection(&sel, SystemAppearance::default(), Mode::Light, &none, None, &mut |_| None);
    assert_eq!((shown.theme.as_str(), shown.notices.as_slice()), (SUITE_DEFAULT, &[][..]));
}

#[test]
fn single_variant_theme_in_the_other_slot_shows_its_own_variant() {
    let sel = selection("craft.soft-dark", "craft.pro-dark", ModeChoice::System);
    let shown = show(&sel, SystemAppearance::default());
    assert_eq!(shown.theme.as_str(), "craft.soft-dark");
    assert_eq!((shown.appearance.mode, shown.resolved.variant_used), (Mode::Light, Mode::Dark));
    assert_eq!(shown.notices, [Fallback::SystemSilent, Fallback::OnlyVariant(Mode::Dark)]);
}

#[cfg(feature = "serde")]
#[test]
fn serde_forms_round_trip() {
    let sel = Selection {
        theme: ThemePair { light: id("craft.pro-light"), dark: ThemeId::app_default() },
        mode: ModeChoice::System,
        contrast: ContrastChoice::High,
        size: Size::Comfortable,
        text_size: TextSize::Larger,
    };
    let json = serde_json::to_string(&sel).unwrap();
    assert_eq!(
        json,
        r#"{"theme":{"light":"craft.pro-light","dark":"craft.app-default"},"mode":"system","contrast":"high","size":"comfortable","text_size":"larger"}"#
    );
    assert_eq!(serde_json::from_str::<Selection>(&json).unwrap(), sel);

    let suite = SuiteChoice {
        theme_dark: Some(id("craft.studio")),
        mode: Some(ModeChoice::Dark),
        text_size: Some(TextSize::Default),
        ..SuiteChoice::default()
    };
    let json = serde_json::to_string(&suite).unwrap();
    assert_eq!(json, r#"{"theme_light":null,"theme_dark":"craft.studio","mode":"dark","contrast":null,"size":null,"text_size":"default"}"#);
    assert_eq!(serde_json::from_str::<SuiteChoice>(&json).unwrap(), suite);
    assert_eq!(
        serde_json::from_str::<SuiteChoice>(r#"{"size":"compact"}"#).unwrap(),
        SuiteChoice { size: Some(Size::Compact), ..SuiteChoice::default() }
    );

    let app = AppChoice { theme: Some(ThemePair::same(id("craft.breeze"))), size: None };
    let json = serde_json::to_string(&app).unwrap();
    assert_eq!(json, r#"{"theme":{"light":"craft.breeze","dark":"craft.breeze"},"size":null}"#);
    assert_eq!(serde_json::from_str::<AppChoice>(&json).unwrap(), app);
    assert!(
        serde_json::from_str::<Selection>(
            r#"{"theme":{"light":"Bad Id","dark":"craft.studio"},"mode":"dark","contrast":"system","size":"compact","text_size":"default"}"#
        )
        .is_err()
    );
}
