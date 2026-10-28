use std::cell::Cell;

use craft_theme::{
    Appearance, ButtonVariant, ComponentField, Contrast, ControlSize, Mode, OverrideValue, Size, State, TextSize, ThemeDef, builtins, contrast_ratio,
    resolve,
};
use egui::epaint::Vertex;
use egui::{Color32, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, Ui, vec2};

use super::*;
use crate::primitives::{Corner, SurfaceKind};
use crate::theme::{Base, FontFamilies, ToEgui, UiTokens, install, scope};

fn theme(id: &str) -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == id).unwrap().clone()
}

fn tokens_of(theme: &ThemeDef, mode: Mode) -> UiTokens {
    let appearance = Appearance { mode, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    UiTokens::new(resolve(theme, appearance, None).tokens, appearance, mode, FontFamilies::default())
}

/// Every built-in in each mode it has.
fn every_builtin() -> Vec<UiTokens> {
    builtins().iter().flat_map(|t| [Mode::Light, Mode::Dark].map(|mode| tokens_of(t, mode))).collect()
}

#[test]
fn button_styles_are_their_button_tokens() {
    for t in every_builtin() {
        let b = &t.component.button;
        for variant in ButtonVariant::ALL {
            let colors = b.colors(variant);
            for size in SIZES {
                let s = Button::style(&t, variant, size);
                for state in State::ALL {
                    assert_eq!(s.fill.get(state), colors.fill.get(state).to_egui(), "{variant:?} fill {state:?}");
                    assert_eq!(s.border.get(state), colors.border.get(state).to_egui(), "{variant:?} border {state:?}");
                    assert_eq!(s.text.get(state), colors.text.get(state).to_egui(), "{variant:?} text {state:?}");
                }
                assert_eq!((s.height, s.pad_x, s.icon, s.gap, s.label), (b.height.get(size), b.pad_x.get(size), b.icon.get(size), b.gap, b.label));
                assert_eq!(s.radius, Corner::Button.radius(&t));
                let surface = if variant == ButtonVariant::Ghost { None } else { Some(SurfaceKind::Raised) };
                assert_eq!(s.surface, surface);
            }
        }
    }
}

#[test]
fn a_button_radius_override_reaches_button_corners() {
    let mut def = theme("craft.graphite");
    def.components.insert(ComponentField::from_path("button.radius").unwrap(), OverrideValue::Length(3.0));
    let t = tokens_of(&def, Mode::Dark);
    assert_ne!(t.shape.radius.button, 3.0);
    assert_eq!(Button::style(&t, ButtonVariant::Primary, ControlSize::Md).radius, egui::CornerRadius::same(3));
    assert_eq!(ToggleButton::style(&t, ControlSize::Sm).off.radius, egui::CornerRadius::same(3));
}

#[test]
fn icon_button_toggle_button_and_segmented_styles_come_from_their_tokens() {
    for t in every_builtin() {
        let c = &t.component;
        for size in SIZES {
            let i = IconButton::style(&t, size);
            assert_eq!((i.side, i.icon, i.radius), (c.icon_button.size.get(size), c.icon_button.icon.get(size), Corner::Control.radius(&t)));
            assert_eq!((i.fill, i.text), (c.button.ghost.fill.to_egui(), c.button.ghost.text.to_egui()));
            assert_eq!((i.selected_fill, i.selected_text), (c.icon_button.selected.fill.to_egui(), c.icon_button.selected.text.to_egui()));

            let toggle = ToggleButton::style(&t, size);
            assert_eq!(toggle.off, Button::style(&t, ButtonVariant::Secondary, size));
            let pressed = c.button.secondary.fill.pressed.to_egui();
            assert_eq!(toggle.on_fill.get(State::Rest), pressed, "on, the toggle rests pressed");
            assert_eq!(toggle.on_fill.get(State::Hover), pressed);
            assert_eq!(toggle.on_text.get(State::Rest), c.button.secondary.text.pressed.to_egui());
            let on_disabled = [&toggle.on_fill, &toggle.on_border, &toggle.on_text].map(|s| s.get(State::Disabled));
            let tokens = craft_theme::ButtonPart::ALL.map(|part| c.toggle_button.on_disabled(part).to_egui());
            assert_eq!(on_disabled, tokens, "on and disabled: toggle_button.on_disabled.*");
            assert_ne!(toggle.on_fill.get(State::Disabled), toggle.off.fill.get(State::Disabled), "on and disabled still looks on");

            let s = Segmented::style(&t, size);
            let seg = &c.segmented;
            assert_eq!((s.height, s.pad_x, s.icon, s.inset), (seg.height.get(size), seg.pad_x.get(size), seg.icon.get(size), seg.inset));
            assert_eq!(s.track.fill, seg.track.to_egui());
            let sunken = crate::primitives::Surface::style(&t, SurfaceKind::Sunken);
            assert_eq!((s.track.edge, s.track.radius), (sunken.edge, sunken.radius), "the track's edge is the sunken surface's");
            assert_eq!((s.fill, s.text), (seg.segment.fill.to_egui(), seg.segment.text.to_egui()));
            assert_eq!((s.selected_fill, s.selected_text), (seg.selected.fill.to_egui(), seg.selected.text.to_egui()));
        }
    }
}

#[test]
fn links_underline_at_rest_only_when_accent_text_is_close_to_normal_text() {
    for t in every_builtin() {
        let style = Link::style(&t);
        let close = contrast_ratio(t.color.accent.text, t.color.text.normal) < link::DISTINCT_FROM_TEXT;
        assert_eq!(style.underline == LinkUnderline::Always, close);
        assert_eq!((style.color, style.stroke.color), (t.color.accent.text.to_egui(), t.color.accent.text.to_egui()));
    }
    // Pro Light's link blue is 3.13:1 against its text; most themes' accent text is closer.
    let pro = theme("craft.pro-light");
    let light = tokens_of(&pro, Mode::Light);
    let ratio = contrast_ratio(light.color.accent.text, light.color.text.normal);
    assert_eq!(Link::style(&light).underline, LinkUnderline::Hover, "{ratio:.2}");
    let mut close = pro;
    close.pins.light = [(craft_theme::ColorToken::AccentText, light.color.text.muted)].into_iter().collect();
    assert_eq!(Link::style(&tokens_of(&close, Mode::Light)).underline, LinkUnderline::Always);
}

/// One pass of `add` in `theme` (light), with `events`: the shapes, flattened.
fn pass(ctx: &Context, theme: &ThemeDef, events: Vec<Event>, add: impl FnOnce(&mut Ui)) -> Vec<Shape> {
    fn flatten(shape: Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.into_iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), events, ..Default::default() };
    let mut add = Some(add);
    let mut output = ctx.run_ui(input, |ui| {
        scope(
            ui,
            |o| {
                o.theme = Some(theme.clone());
                o.mode = Some(Mode::Light);
            },
            |ui| add.take().map(|add| add(ui)),
        );
    });
    output.textures_delta.clear();
    let mut out = Vec::new();
    for clipped in output.shapes {
        flatten(clipped.shape, &mut out);
    }
    out
}

fn context() -> Context {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn a_theme_override_of_the_primary_rest_fill_reaches_the_painted_fill() {
    // Breeze pins `button.primary.fill.rest` away from its accent.
    let breeze = theme("craft.breeze");
    let t = tokens_of(&breeze, Mode::Light);
    let pinned = t.component.button.primary.fill.rest.to_egui();
    assert_ne!(pinned, t.color.accent.solid.to_egui());
    let ctx = context();
    let shapes = pass(&ctx, &breeze, vec![], |ui| {
        ui.add(Button::new("Export").variant(ButtonVariant::Primary));
    });
    let filled = shapes.iter().any(|s| matches!(s, Shape::Rect(r) if r.fill == pinned));
    assert!(filled, "no rect filled with the pinned {pinned:?}");
}

/// The colours of the vertices nearest the top-left and bottom-right corners of `rect` among
/// the meshes, i.e. the bevel's two sides.
fn bevel_corners(shapes: &[Shape], rect: Rect) -> (Color32, Color32) {
    let vertices: Vec<&Vertex> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Mesh(mesh) => Some(mesh.vertices.iter()),
            _ => None,
        })
        .flatten()
        .filter(|v| rect.expand(1.0).contains(v.pos))
        .collect();
    let nearest = |corner: Pos2| vertices.iter().min_by(|a, b| a.pos.distance(corner).total_cmp(&b.pos.distance(corner))).unwrap().color;
    (nearest(rect.left_top()), nearest(rect.right_bottom()))
}

#[test]
fn bevel_themes_bevel_buttons_and_press_them_in() {
    let classic = theme("craft.classic");
    let t = tokens_of(&classic, Mode::Light);
    let raised = crate::primitives::Surface::style(&t, SurfaceKind::Raised).edge;
    let crate::primitives::SurfaceEdge::Bevel { top_left: light, bottom_right: dark, .. } = raised else { panic!("Classic bevels: {raised:?}") };
    let ctx = context();
    let rect = Cell::new(Rect::NOTHING);
    let frame = |events| {
        pass(&ctx, &classic, events, |ui| {
            rect.set(ui.add(Button::new("Apply").variant(ButtonVariant::Primary)).rect);
        })
    };
    frame(vec![]);
    let center = rect.get().center();
    let rest = frame(vec![Event::PointerMoved(center)]);
    assert_eq!(bevel_corners(&rest, rect.get()), (light, dark), "raised: lit on the top left");
    let pressed = frame(vec![press(center, true)]);
    assert_eq!(bevel_corners(&pressed, rect.get()), (dark, light), "pressed: lit on the bottom right");
    let released = frame(vec![press(center, false)]);
    assert_eq!(bevel_corners(&released, rect.get()), (light, dark));
}

#[test]
fn loading_keeps_the_width_and_does_not_activate() {
    let graphite = theme("craft.graphite");
    // The spinner takes the icon's place, or the label's without an icon.
    for icon in [Some(crate::icons::Icon::Save), None] {
        let ctx = context();
        let width = |loading: bool| {
            let width = Cell::new(0.0);
            pass(&ctx, &graphite, vec![], |ui| {
                let mut button = Button::new("Export").loading(loading);
                if let Some(icon) = icon {
                    button = button.icon(icon);
                }
                width.set(ui.add(button).rect.width());
            });
            width.get()
        };
        assert_eq!(width(true), width(false), "loading keeps the width with icon {icon:?}");
    }

    let ctx = context();
    let (rect, clicks) = (Cell::new(Rect::NOTHING), Cell::new(0));
    let frame = |loading: bool, events| {
        pass(&ctx, &graphite, events, |ui| {
            let response = ui.add(Button::new("Export").icon(crate::icons::Icon::Save).loading(loading).id_salt("export"));
            rect.set(response.rect);
            clicks.set(clicks.get() + usize::from(response.clicked()));
        });
    };
    frame(false, vec![]);
    let idle = rect.get();
    frame(true, vec![]);
    assert_eq!(rect.get(), idle, "loading keeps the size");
    let center = idle.center();
    for events in [vec![Event::PointerMoved(center)], vec![press(center, true)], vec![press(center, false)], vec![]] {
        frame(true, events);
    }
    assert_eq!(clicks.get(), 0, "a click while loading does nothing");
    // egui hit-tests against last frame's widgets: one idle frame first.
    for events in [vec![], vec![press(center, true)], vec![press(center, false)], vec![]] {
        frame(false, events);
    }
    assert_eq!(clicks.get(), 1, "the same click activates when idle");
}

#[test]
fn turning_a_control_off_keeps_the_ids_of_the_widgets_after_it() {
    let ctx = context();
    let graphite = theme("craft.graphite");
    let mut ids = Vec::new();
    for enabled in [true, false] {
        pass(&ctx, &graphite, vec![], |ui| {
            ui.add(Button::new("Export").enabled(enabled));
            let mut choice = 0;
            Segmented::new("mode", "Mode", &mut choice).item(0, "New").item(1, "Add").enabled(enabled).show(ui);
            ids.push(ui.add(egui::Button::new("Next")).id);
        });
    }
    assert_eq!(ids[0], ids[1], "the next automatic id");
}
