use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_theme::{
    AppDefaults, Appearance, Contrast, ContrastChoice, Mode, ModeChoice, Selection, Size, SystemAppearance, TextSize, ThemeDef, ThemeId, ThemePair,
    builtins, resolve,
};
use egui::{Context, Popup, RawInput, Theme, Ui};

use super::*;

fn theme(id: &str) -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == id).cloned().unwrap()
}

fn base(id: &str, mode: ModeChoice) -> Base {
    let pair = ThemePair::same(theme(id).id);
    let sel = Selection { theme: pair, mode, contrast: ContrastChoice::Standard, size: Size::Compact, text_size: TextSize::Default };
    let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode, size: Size::Compact };
    Base::resolve(&sel, SystemAppearance::default(), &defaults, None, &mut |id| builtins().iter().find(|t| t.id == *id).cloned())
}

fn frame(ctx: &Context, mut f: impl FnMut(&mut Ui)) {
    ctx.run_ui(RawInput::default(), |ui| f(ui)).drop_without_applying_deltas();
}

fn mode_of(ctx: &Context) -> Mode {
    if ctx.theme() == Theme::Dark { Mode::Dark } else { Mode::Light }
}

#[test]
fn style_follows_the_tokens() {
    let fonts = FontFamilies::default();
    let [studio, solarized] = [("craft.studio", Mode::Dark), ("craft.solarized-dark", Mode::Dark)].map(|(id, mode)| {
        let appearance = Appearance { mode, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
        resolve(&theme(id), appearance, None).tokens
    });
    // `Style`'s number formatter compares by address, so compare the printed form.
    assert_eq!(format!("{:?}", style_for(&studio, &fonts)), format!("{:?}", style_for(&studio, &fonts)), "deterministic");

    for tokens in [&studio, &solarized] {
        let style = style_for(tokens, &fonts);
        let v = &style.visuals;
        assert!(v.dark_mode);
        assert_eq!(v.panel_fill, tokens.color.surface.panel.to_egui());
        assert_eq!(v.widgets.inactive.weak_bg_fill, tokens.component.button.secondary.fill.rest.to_egui());
        assert_eq!(v.widgets.hovered.bg_fill, tokens.component.button.secondary.fill.hover.to_egui());
        assert_eq!(v.widgets.noninteractive.fg_stroke.color, tokens.color.text.normal.to_egui());
        assert_eq!(style.spacing.interact_size.y, tokens.size.control.md);
        assert_eq!(style.text_styles[&egui::TextStyle::Heading].size, tokens.typography.heading.size);
    }
    let (a, b) = (style_for(&studio, &fonts), style_for(&solarized, &fonts));
    assert_ne!(studio.color.surface.panel, solarized.color.surface.panel);
    assert_ne!(a.visuals.panel_fill, b.visuals.panel_fill);
    assert_ne!(a.visuals.widgets.inactive.weak_bg_fill, b.visuals.widgets.inactive.weak_bg_fill);
}

#[test]
fn tokens_come_from_the_innermost_scope_then_the_base() {
    let ctx = Context::default();
    frame(&ctx, |ui| {
        let mode = mode_of(ui.ctx());
        assert_eq!(**Tokens::of(ui).tokens(), Base::suite_default().shown(mode).resolved.tokens, "suite default without install");
    });

    let studio = base("craft.studio", ModeChoice::Dark);
    install(&ctx, &studio);
    assert_eq!(ctx.theme(), Theme::Dark);
    let dark = &studio.dark.resolved.tokens;
    frame(&ctx, |ui| {
        assert_eq!(*Tokens::of(ui).tokens().as_ref(), *dark);
        scope(
            ui,
            |o| o.size = Some(Size::Comfortable),
            |ui| {
                let outer = Tokens::of(ui);
                assert_eq!(outer.appearance.size, Size::Comfortable);
                assert_eq!(ui.spacing().interact_size.y, outer.size.control.md, "the child Ui has the scope's style");
                scope(
                    ui,
                    |o| o.contrast = Some(Contrast::High),
                    |ui| {
                        let inner = Tokens::of(ui);
                        assert_eq!((inner.appearance.size, inner.appearance.contrast), (Size::Comfortable, Contrast::High), "scopes nest");
                    },
                );
                assert!(Arc::ptr_eq(&Tokens::of(ui), &outer), "the inner scope popped");
            },
        );
        scope(ui, |o| o.mode = Some(Mode::Light), |ui| assert_eq!(*Tokens::of(ui).tokens().as_ref(), studio.light.resolved.tokens));
        assert_eq!(*Tokens::of(ui).tokens().as_ref(), *dark, "base after the scope");

        let panicked = catch_unwind(AssertUnwindSafe(|| scope(ui, |o| o.size = Some(Size::Comfortable), |_| panic!("inside the scope"))));
        assert!(panicked.is_err());
        assert_eq!(*Tokens::of(ui).tokens().as_ref(), *dark, "base after a panic");
    });

    // The second frame reuses the first frame's resolution.
    let first = std::cell::RefCell::new(None);
    for _ in 0..2 {
        frame(&ctx, |ui| {
            scope(
                ui,
                |o| o.contrast = Some(Contrast::High),
                |ui| {
                    let tokens = Tokens::of(ui);
                    let first = first.borrow_mut().get_or_insert_with(|| Arc::clone(&tokens)).clone();
                    assert!(Arc::ptr_eq(&tokens, &first), "cached");
                },
            );
        });
    }
}

#[test]
fn popups_take_the_scope_they_open_in() {
    let ctx = Context::default();
    let studio = base("craft.studio", ModeChoice::Dark);
    install(&ctx, &studio);
    let light_fill = studio.light.resolved.tokens.color.surface.panel.to_egui();
    let dark_fill = studio.dark.resolved.tokens.color.surface.panel.to_egui();
    let (plain, menu) = (std::cell::RefCell::new(Vec::new()), std::cell::RefCell::new(Vec::new()));
    for _ in 0..2 {
        frame(&ctx, |ui| {
            scope(
                ui,
                |o| o.mode = Some(Mode::Light),
                |ui| {
                    let field = ui.button("Open");
                    Popup::from_response(&field).id(egui::Id::unique("plain")).open(true).show(|ui| {
                        let before = ui.visuals().panel_fill;
                        apply_popup_style(ui);
                        plain.borrow_mut().push((before, ui.visuals().panel_fill));
                    });
                    #[allow(deprecated, reason = "the deprecated menu takes the theme scope's style")]
                    crate::combo::Menu::new(ui, &field).open(&mut true).show_ui(|ui| menu.borrow_mut().push(ui.visuals().panel_fill));
                },
            );
        });
    }
    let (plain, menu) = (plain.into_inner(), menu.into_inner());
    assert!(!plain.is_empty() && !menu.is_empty());
    assert!(plain.iter().all(|p| *p == (dark_fill, light_fill)), "egui's popup starts from the global style; apply_popup_style scopes it: {plain:?}");
    assert!(menu.iter().all(|fill| *fill == light_fill), "craft-ui's menu takes the scope: {menu:?}");
}

#[test]
fn install_repaints_only_on_change() {
    let ctx = Context::default();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    ctx.set_repaint_observer(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    let studio = base("craft.studio", ModeChoice::Dark);
    install(&ctx, &studio);
    assert!(requests.swap(0, Ordering::Relaxed) > 0, "first install");
    install(&ctx, &studio);
    assert_eq!(requests.load(Ordering::Relaxed), 0, "same base");
    install(&ctx, &base("craft.studio", ModeChoice::Light));
    assert!(requests.load(Ordering::Relaxed) > 0, "changed base");
    assert_eq!(ctx.theme(), Theme::Light);
}

#[test]
fn every_role_maps_to_its_slot_family_size_and_line_height() {
    use craft_theme::{FontSlot, TypeRole};
    use egui::{Color32, FontFamily};

    let semibold = FontFamily::Name(crate::fonts::SEMIBOLD.into());
    let families = FontFamilies { ui: FontFamily::Proportional, ui_semibold: semibold.clone(), mono: FontFamily::Monospace, optical_size: false };
    let appearance =
        Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    let tokens = resolve(&theme("craft.studio"), appearance, None).tokens;
    let ui_tokens = UiTokens::new(tokens.clone(), appearance, Mode::Light, families.clone());
    for role in [TypeRole::Caption, TypeRole::Label, TypeRole::Body, TypeRole::BodyStrong, TypeRole::Title, TypeRole::Heading, TypeRole::Mono] {
        let style = tokens.typography.get(role);
        let expected = match style.font {
            FontSlot::Ui => FontFamily::Proportional,
            FontSlot::UiSemibold => semibold.clone(),
            FontSlot::Mono => FontFamily::Monospace,
        };
        let font = ui_tokens.font(role);
        assert_eq!((font.size, font.family.clone()), (style.size, expected.clone()), "{role:?}");
        let format = role_format(&ui_tokens, role, Color32::RED);
        assert_eq!(format.font_id, font, "{role:?}");
        assert_eq!(format.line_height, Some(style.line_height), "{role:?}");
        assert_eq!(format.color, Color32::RED);
        assert!(format.font_features.is_empty());
        assert!(format.coords.as_ref().is_empty(), "{role:?}: the suite's faces keep their optical size");
        // With the system face, the UI slots draw at the style's optical size; mono doesn't.
        let optical = super::style_format(&FontFamilies { optical_size: true, ..families.clone() }, style, Color32::RED);
        let opsz = optical.coords.as_ref().iter().find(|(tag, _)| *tag == egui::epaint::text::Tag::new(b"opsz")).map(|(_, size)| *size);
        assert_eq!(opsz, (style.font != FontSlot::Mono).then_some(style.size), "{role:?}");
    }
    // Without a semibold family in the stack, those roles use the proportional family.
    assert_eq!(FontFamilies::default().family(FontSlot::UiSemibold), &FontFamily::Proportional);
    assert!(!tabular(egui::TextFormat::default()).font_features.is_empty());
}

#[test]
fn the_scale_module_is_reachable_from_craft_ui() {
    assert_eq!(scale::Space::S4.px(), craft_theme::scale::Space::S4.px());
    assert_eq!(scale::Radius::R4.px(), craft_theme::scale::Radius::R4.px());
}

#[test]
fn stroke_ext_matches_width_for_every_step_at_1_and_2_ppp() {
    for ppp in [1.0, 2.0] {
        let ctx = Context::default();
        ctx.set_pixels_per_point(ppp);
        ctx.run_ui(RawInput::default(), |_| {}).drop_without_applying_deltas();
        for step in scale::Stroke::ALL {
            let s = step.egui(&ctx, egui::Color32::RED);
            assert_eq!(s.width, step.width(ppp), "{step:?} at {ppp}");
            assert_eq!(s.color, egui::Color32::RED);
        }
        assert_eq!(scale::Stroke::Hairline.egui(&ctx, egui::Color32::RED).width, 1.0 / ppp);
    }
}
