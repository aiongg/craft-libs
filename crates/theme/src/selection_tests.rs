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
    AppDefaults {
        theme: ThemePair { light: id("craft.pro"), dark: id("craft.pro") },
        mode: ModeChoice::Dark,
        size: Size::Compact,
        sizes: AppSizes::default(),
    }
}

fn selection(light: &str, dark: &str, mode: ModeChoice) -> Selection {
    Selection {
        theme: ThemePair { light: id(light), dark: id(dark) },
        mode,
        contrast: ContrastChoice::System,
        motion: MotionChoice::System,
        size: Size::Compact,
        text_size: TextSize::Small,
    }
}

fn show(sel: &Selection, system: SystemAppearance) -> Shown {
    resolve_selection(sel, system, Mode::Light, &defaults(), None, &mut lookup)
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
    assert_eq!((shown.theme.as_str(), shown.notices.as_slice()), ("craft.pro", &[][..]));

    let shown = show(&selection("user.gone", "craft.studio", ModeChoice::Light), SystemAppearance::default());
    assert_eq!(shown.theme.as_str(), "craft.pro");
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
fn serde_forms_round_trip() {
    let sel = Selection {
        theme: ThemePair { light: id("craft.pro"), dark: ThemeId::app_default() },
        mode: ModeChoice::System,
        contrast: ContrastChoice::High,
        motion: MotionChoice::System,
        size: Size::Comfortable,
        text_size: TextSize::Large,
    };
    let json = serde_json::to_string(&sel).unwrap();
    assert_eq!(
        json,
        r#"{"theme":{"light":"craft.pro","dark":"craft.app-default"},"mode":"system","contrast":"high","motion":"system","size":"comfortable","text_size":"large"}"#
    );
    assert_eq!(serde_json::from_str::<Selection>(&json).unwrap(), sel);

    let suite = SuiteChoice {
        theme_dark: Some(id("craft.studio")),
        mode: Some(ModeChoice::Dark),
        text_size: Some(TextSize::Small),
        ..SuiteChoice::default()
    };
    let json = serde_json::to_string(&suite).unwrap();
    assert_eq!(
        json,
        r#"{"theme_light":null,"theme_dark":"craft.studio","mode":"dark","contrast":null,"motion":null,"size":null,"text_size":"small"}"#
    );
    assert_eq!(serde_json::from_str::<SuiteChoice>(&json).unwrap(), suite);
    assert_eq!(
        serde_json::from_str::<SuiteChoice>(r#"{"size":"compact"}"#).unwrap(),
        SuiteChoice { size: Some(Size::Compact), ..SuiteChoice::default() }
    );

    let app = AppChoice::FOLLOW_SUITE.with_theme(ThemePair::same(id("craft.breeze"))).with_mode(ModeChoice::System);
    let json = serde_json::to_string(&app).unwrap();
    assert_eq!(json, r#"{"theme":{"light":"craft.breeze","dark":"craft.breeze"},"mode":"system","size":null}"#);
    assert_eq!(serde_json::from_str::<AppChoice>(&json).unwrap(), app);
    // Choices stored before `mode` existed still load.
    let legacy = r#"{"theme":{"light":"craft.breeze","dark":"craft.breeze"},"size":null}"#;
    assert_eq!(serde_json::from_str::<AppChoice>(legacy).unwrap(), AppChoice { mode: None, ..app.clone() });
    assert!(
        serde_json::from_str::<Selection>(
            r#"{"theme":{"light":"Bad Id","dark":"craft.studio"},"mode":"dark","contrast":"system","size":"compact","text_size":"small"}"#
        )
        .is_err()
    );
}
