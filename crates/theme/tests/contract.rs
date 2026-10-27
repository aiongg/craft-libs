//! The suite-theme contract as apps see it: built-ins, resolution, derived states and advisory
//! contrast diagnostics.

use std::cell::RefCell;

use craft_theme::{
    APP_DEFAULT, AppChoice, CoreRoles, DEFAULT_SUITE_THEME, Fallback, Mode, Origin, Resolution, Rgba, Role, RoleMap, Selection, SuiteTheme, ThemeId,
    Variant,
    contrast::{self, Need, Paint},
    derive, lookup_builtin, resolve,
};
use proptest::prelude::*;

fn id(s: &str) -> ThemeId {
    ThemeId::parse(s).unwrap()
}

fn role_map(roles: &CoreRoles) -> RoleMap {
    Role::ALL.into_iter().map(|r| (r.key().to_owned(), roles.get(r))).collect()
}

fn graphite(v: Variant) -> CoreRoles {
    *craft_theme::builtin(&id("craft.graphite")).unwrap().variant(v).unwrap()
}

#[test]
fn builtins_have_both_variants_and_no_diagnostics() {
    let ids: Vec<_> = craft_theme::builtins().iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["craft.artcraft", "craft.graphite", "craft.high-contrast"]);
    for t in craft_theme::builtins() {
        assert_eq!(t.origin, Origin::BuiltIn);
        assert_eq!(ThemeId::parse(t.id.as_str()).as_ref(), Ok(&t.id));
        assert!(t.id.is_builtin() && !t.id.is_app_default());
        assert!(t.light.is_some() && t.dark.is_some(), "{}", t.id);
        for (v, roles) in t.variants() {
            assert!(Role::ALL.iter().all(|&r| roles.get(r).is_opaque()), "{} {v:?}", t.id);
        }
        // Covers every derived state in the painted-pair table, hover and pressed included.
        assert_eq!(contrast::diagnose(t), [], "{}", t.id);
    }
}

#[test]
fn default_selection_is_artcraft_following_the_system() {
    let sel = Selection::default();
    assert_eq!(sel.theme.as_str(), DEFAULT_SUITE_THEME);
    assert_eq!(sel.mode, Mode::System);
    assert!(craft_theme::builtin(&sel.theme).is_some());
    assert_eq!(AppChoice::default(), AppChoice::FollowSuite);
    let own = Selection { theme: ThemeId::app_default(), mode: Mode::Dark };
    assert_eq!(AppChoice::FollowSuite.effective(&sel), &sel);
    assert_eq!(AppChoice::Own(own.clone()).effective(&sel), &own);
}

/// One user theme with only a dark variant, alongside the built-ins.
fn lookup_with_dark_only(id: &ThemeId) -> Option<SuiteTheme> {
    if id.as_str() == "night" {
        let dark = role_map(&graphite(Variant::Dark));
        return Some(SuiteTheme::from_maps("night", "Night", 1, None, Some(&dark)).unwrap().0);
    }
    lookup_builtin(id)
}

#[test]
fn resolution_table() {
    let themes = [APP_DEFAULT, "craft.artcraft", "craft.graphite", "craft.high-contrast", "night", "gone"];
    let systems = [None, Some(Variant::Light), Some(Variant::Dark)];
    for theme in themes {
        for mode in Mode::ALL {
            for system in systems {
                for fallback in Variant::ALL {
                    let sel = Selection { theme: id(theme), mode };
                    let calls = RefCell::new(Vec::new());
                    let lookup = |i: &ThemeId| {
                        calls.borrow_mut().push(i.clone());
                        lookup_with_dark_only(i)
                    };
                    let got = resolve(&sel, system, fallback, &lookup);
                    let case = format!("{theme} {mode:?} {system:?} {fallback:?}");
                    // Pure: the same inputs give the same answer, and only the selected id is looked up.
                    assert_eq!(resolve(&sel, system, fallback, &lookup), got, "{case}");
                    assert!(calls.borrow().iter().all(|i| *i == sel.theme), "{case}");

                    let wanted = match mode {
                        Mode::Light => Variant::Light,
                        Mode::Dark => Variant::Dark,
                        Mode::System => system.unwrap_or(fallback),
                    };
                    let silent = (mode == Mode::System && system.is_none()).then_some(Fallback::SystemSilent);
                    match (theme, &got) {
                        (APP_DEFAULT, Resolution::AppDefault { variant, why }) => {
                            assert_eq!((*variant, why), (wanted, &silent), "{case}");
                        }
                        ("gone", Resolution::AppDefault { variant, why }) => {
                            assert_eq!((*variant, why), (wanted, &Some(Fallback::UnknownTheme(id("gone")))), "{case}");
                        }
                        ("night", Resolution::Theme(r)) => {
                            assert_eq!(r.variant, Variant::Dark, "{case}");
                            let note = if wanted == Variant::Dark { silent } else { Some(Fallback::OnlyVariant(Variant::Dark)) };
                            assert_eq!(r.note, note, "{case}");
                            assert_eq!(r.roles, graphite(Variant::Dark), "{case}");
                        }
                        (_, Resolution::Theme(r)) if id(theme).is_builtin() => {
                            let builtin = craft_theme::builtin(&r.theme).unwrap();
                            assert_eq!((r.theme.as_str(), r.variant, &r.note), (theme, wanted, &silent), "{case}");
                            assert_eq!(&r.roles, builtin.variant(wanted).unwrap(), "{case}");
                            assert_eq!(r.derived, derive(&r.roles, r.variant), "{case}");
                        }
                        _ => panic!("{case}: {got:?}"),
                    }
                }
            }
        }
    }
}

#[test]
fn low_contrast_user_theme_is_reported_but_kept() {
    let mut light = graphite(Variant::Light);
    light.text = light.surface;
    let (theme, missing) = SuiteTheme::from_maps("flat", "Flat", 1, Some(&role_map(&light)), None).unwrap();
    assert_eq!(missing, []);
    let diagnostics = contrast::diagnose(&theme);
    let flat = diagnostics.iter().find(|d| d.fg == Paint::Role(Role::Text) && d.bg == Paint::Role(Role::Surface)).unwrap();
    assert_eq!((flat.variant, flat.need), (Variant::Light, Need::Text));
    assert!((flat.ratio - 1.0).abs() < 1e-6);
    assert_eq!(theme.light, Some(light));

    let sel = Selection { theme: theme.id.clone(), mode: Mode::Light };
    let Resolution::Theme(r) = resolve(&sel, None, Variant::Light, &|_| Some(theme.clone())) else { panic!() };
    assert_eq!((r.roles, r.note), (light, None));
}

fn lighter(a: Rgba, b: Rgba) -> bool {
    contrast::ratio(a, Rgba::BLACK) > contrast::ratio(b, Rgba::BLACK)
}

#[test]
fn derived_states_step_away_from_text_when_toward_would_fail() {
    // ArtCraft dark's mid-grey surface: lighter steps would take text_muted below 4.5:1, so hover
    // and pressed darken.
    let artcraft = craft_theme::builtin(&id(DEFAULT_SUITE_THEME)).unwrap().dark.unwrap();
    let d = derive(&artcraft, Variant::Dark);
    assert!(lighter(artcraft.surface, d.hover) && lighter(d.hover, d.pressed));
    // Toward text where that keeps the pairs: the dark selection lightens, light hover darkens.
    assert!(lighter(d.selected_hover, artcraft.selection));
    let light = graphite(Variant::Light);
    assert!(lighter(light.surface, derive(&light, Variant::Light).hover));

    let mut grey = light;
    (grey.background, grey.surface, grey.raised, grey.text, grey.text_muted) =
        (Rgba::from_rgb_u32(0xC4C4C4), Rgba::from_rgb_u32(0xC4C4C4), Rgba::from_rgb_u32(0xC4C4C4), Rgba::BLACK, Rgba::from_rgb_u32(0x4E4E4E));
    let d = derive(&grey, Variant::Light);
    assert!(lighter(d.hover, grey.surface) && lighter(d.pressed, grey.surface));
    for bg in [d.hover, d.pressed] {
        assert!(contrast::ratio(grey.text_muted, bg) >= 4.5);
    }
}

fn opaque() -> impl Strategy<Value = Rgba> {
    any::<[u8; 3]>().prop_map(|[r, g, b]| Rgba::rgb(r, g, b))
}

fn channel_range(lo: u8, hi: u8) -> impl Strategy<Value = Rgba> {
    [lo..=hi, lo..=hi, lo..=hi].prop_map(|[r, g, b]| Rgba::rgb(r, g, b))
}

/// Mostly plausible light themes, so the preconditions below hold often, plus arbitrary colours.
fn core_roles() -> impl Strategy<Value = CoreRoles> {
    (
        [channel_range(190, 255), channel_range(190, 255), channel_range(190, 255)],
        [channel_range(0, 100), channel_range(0, 100)],
        [opaque(), opaque(), opaque(), opaque(), opaque(), opaque(), opaque(), opaque()],
    )
        .prop_map(|([background, surface, raised], [text, text_muted], [line, accent, on_accent, selection, focus, good, warn, bad])| {
            CoreRoles { background, surface, raised, text, text_muted, line, accent, on_accent, selection, focus, good, warn, bad }
        })
}

proptest! {
    #[test]
    fn derived_states_keep_pairs_their_core_roles_allow(roles in core_roles(), dark in any::<bool>()) {
        let v = if dark { Variant::Dark } else { Variant::Light };
        let d = derive(&roles, v);
        let text_ok = |fg| [roles.background, roles.surface, roles.raised].iter().all(|&bg| contrast::ratio(fg, bg) >= 4.5);
        let theme = SuiteTheme::from_maps("random", "Random", 1, Some(&role_map(&roles)), None).unwrap().0;
        let failing: Vec<_> = contrast::diagnose(&theme).into_iter().map(|d| (d.fg, d.bg)).collect();
        let fails = |name: &'static str| failing.iter().any(|&(fg, bg)| fg == Paint::Derived(name) || bg == Paint::Derived(name));
        if text_ok(roles.text) {
            prop_assert!(!fails("accent_text") && !fails("control_edge"));
        }
        if text_ok(roles.text_muted) {
            prop_assert!(!fails("text_faint"));
        }
        if contrast::ratio(roles.on_accent, roles.accent) >= 4.5 {
            prop_assert!(!fails("accent_hover") && !fails("accent_pressed"));
        }
        // Resolution is pure and hands back the theme's colours with `derive`'s states.
        let sel = Selection { theme: theme.id.clone(), mode: if dark { Mode::Dark } else { Mode::Light } };
        let lookup = |_: &ThemeId| Some(theme.clone());
        let got = resolve(&sel, None, v, &lookup);
        prop_assert_eq!(&got, &resolve(&sel, None, v, &lookup));
        let Resolution::Theme(r) = got else { return Err(TestCaseError::fail("not a theme")) };
        prop_assert_eq!(r.roles, roles);
        prop_assert_eq!(r.derived, derive(&roles, Variant::Light));
        prop_assert_eq!(d.accent_soft, roles.surface.lerp_gamma(roles.accent, 0.14));
    }
}
