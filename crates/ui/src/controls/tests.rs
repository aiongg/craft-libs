use std::cell::Cell;

use craft_theme::{
    Appearance, ButtonVariant, ComponentField, Contrast, ControlSize, Mode, OverrideValue, Size, State, TextSize, ThemeDef, builtins, contrast_ratio,
    resolve,
};
use egui::epaint::Vertex;
use egui::{Color32, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, Ui, Vec2, vec2};

use super::*;
use crate::primitives::{Corner, SurfaceKind};
use crate::theme::{Base, FontFamilies, ToEgui, Tokens, TokensOf as _, UiTokens, install, scope};

pub(super) fn theme(id: &str) -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == id).unwrap().clone()
}

pub(super) fn tokens_of(theme: &ThemeDef, mode: Mode) -> UiTokens {
    let appearance = Appearance { mode, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    UiTokens::new(resolve(theme, appearance, None).tokens, appearance, mode, FontFamilies::default())
}

/// Every built-in in each mode it has.
pub(super) fn every_builtin() -> Vec<UiTokens> {
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
            assert_eq!(
                (i.badge, i.badge_edge),
                (c.icon_button.badge.get(size), egui::Stroke::new(craft_theme::Stroke::W1.width(1.0), t.color.line.mark.to_egui()))
            );

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
    // Pro's light link blue is 3.13:1 against its text; most themes' accent text is closer.
    let pro = theme("craft.pro");
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
    bevel_at(shapes, rect, rect.left_top(), rect.right_bottom())
}

/// The colours of the mesh vertices in `rect` nearest `a` and `b`.
fn bevel_at(shapes: &[Shape], rect: Rect, a: Pos2, b: Pos2) -> (Color32, Color32) {
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
    (nearest(a), nearest(b))
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

#[test]
fn a_least_width_wider_than_the_room_stops_at_the_room() {
    let ctx = context();
    let width = Cell::new(0.0);
    pass(&ctx, &theme("craft.pro"), Vec::new(), |ui| {
        ui.allocate_ui(vec2(80.0, 40.0), |ui| {
            ui.set_max_width(80.0);
            width.set(ui.add(Button::new("Export").min_width(400.0)).rect.width());
        });
    });
    assert!((width.get() - 80.0).abs() < 0.5, "{}", width.get());
}

#[test]
fn check_box_radio_and_switch_styles_come_from_the_check_tokens() {
    for t in every_builtin() {
        let c = &t.component.check;
        let sunken = crate::primitives::Surface::style(&t, SurfaceKind::Sunken);
        let (off, on) = (IndicatorColors::from(&c.off), IndicatorColors::from(&c.on));
        assert_eq!((off.mark, on.mark), (None, Some(c.on.mark.to_egui())), "off has no mark");
        let text = |s: State| if s == State::Disabled { t.color.text.disabled.to_egui() } else { t.color.text.normal.to_egui() };
        for size in SIZES {
            let b = Checkbox::style(&t, size);
            assert_eq!((b.height, b.side, b.gap, b.label, b.mark), (c.height.get(size), c.size.get(size), c.gap, c.label, c.mark.get(size)));
            assert_eq!((b.radius, b.surface, b.off, b.on), (egui::CornerRadius::from(c.radius), sunken, off, on));
            let r = RadioGroup::style(&t, size);
            assert_eq!((r.height, r.diameter, r.dot, r.surface, r.off, r.on), (b.height, b.side, c.dot.get(size), sunken, off, on));
            let s = Switch::style(&t, size);
            assert_eq!((s.width, s.track_height, s.inset), (c.track.get(size), c.size.get(size), c.thumb_inset));
            assert_eq!(s.thumb, s.track_height - 2.0 * s.inset);
            assert_eq!((s.surface, s.off, s.on), (sunken, off, IndicatorColors::from(&c.switch)));
            assert_eq!((s.thumb_fill, s.thumb_border), (c.thumb.fill.to_egui(), c.thumb.border.to_egui()));
            assert_eq!((s.travel, s.easing), (t.motion.r#move.millis as f32 / 1000.0, t.motion.r#move.easing));
            for state in State::ALL {
                assert_eq!([b.text.get(state), r.text.get(state), s.text.get(state)], [text(state); 3], "{state:?}");
            }
        }
    }
}

/// Slider and switch thumbs are knobs in both modes: a light `surface.knob` face (Classic: its
/// bevelled panel grey) on the knob surface, round on a switch. A slider thumb's ring is the
/// theme's `slider.ring`, `accent.soft` while hovered or pressed unless the theme pins it (Classic
/// pins it clear).
#[test]
fn slider_and_switch_thumbs_are_knobs_in_both_modes() {
    let mut defaults = 0;
    for def in builtins() {
        let pins_ring = [&def.pins.light, &def.pins.dark].iter().any(|p| p.colors.keys().any(|k| k.path().starts_with("slider.ring.")));
        for mode in [Mode::Light, Mode::Dark] {
            let t = tokens_of(def, mode);
            let c = &t.color;
            let face = c.surface.knob.to_egui();
            let knob = crate::primitives::Surface::style(&t, SurfaceKind::Knob);
            if matches!(knob.edge, crate::primitives::SurfaceEdge::Border(_)) {
                assert!(c.surface.knob.to_oklch().l > 0.9, "{} {mode:?}: the knob face {} is light", def.id, c.surface.knob.to_hex());
            }
            let ring = t.component.slider.ring.to_egui();
            if !pins_ring {
                let (soft, clear) = (c.accent.soft.to_egui(), Color32::TRANSPARENT);
                assert_eq!(ring, craft_theme::States::new(clear, soft, soft, clear), "{} {mode:?}: the default ring", def.id);
                defaults += 1;
            }
            for size in SIZES {
                let s = Slider::style(&t, size);
                assert_eq!([s.thumb_fill.rest, s.thumb_fill.hover, s.thumb_fill.pressed], [face; 3], "{} {mode:?}", def.id);
                assert_eq!((s.thumb_surface.fill, s.thumb_surface.edge, s.thumb_surface.shadow), (face, knob.edge, knob.shadow));
                assert_eq!((s.ring, s.ring_width), (ring, t.component.slider.ring_width));
                let w = Switch::style(&t, size);
                assert_eq!([w.thumb_fill.rest, w.thumb_fill.hover, w.thumb_fill.pressed], [face; 3], "{} {mode:?}", def.id);
                assert_eq!((w.thumb_surface.fill, w.thumb_surface.edge, w.thumb_surface.shadow), (face, knob.edge, knob.shadow));
            }
        }
    }
    assert!(defaults > 0, "some built-in keeps the default ring");
    let classic = tokens_of(&theme("craft.classic"), Mode::Light);
    assert_eq!(
        classic.component.slider.ring.to_egui(),
        craft_theme::States::new(Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::TRANSPARENT)
    );
}

/// A switch's track and thumb take `shape.radius.pill`, at most half their height: round by
/// default, square in Classic.
#[test]
fn switch_track_and_thumb_follow_the_pill_radius() {
    let round = |side: f32| egui::CornerRadius::from(side / 2.0);
    let classic = tokens_of(&theme("craft.classic"), Mode::Light);
    let rounded = tokens_of(&theme("craft.solarized-dark"), Mode::Dark);
    for size in SIZES {
        let w = Switch::style(&classic, size);
        assert_eq!((w.track_radius, w.thumb_surface.radius), (egui::CornerRadius::ZERO, egui::CornerRadius::ZERO), "classic {size:?}");
        let w = Switch::style(&rounded, size);
        assert_eq!((w.track_radius, w.thumb_surface.radius), (round(w.track_height), round(w.thumb)), "rounded {size:?}");
    }
}

/// A disabled knob lies flat: no shadow under a slider, range-slider or switch thumb. A slider's
/// ring paints only while the slider is hovered or pressed.
#[test]
fn disabled_knobs_cast_no_shadow_and_the_slider_ring_shows_on_hover_and_press() {
    let shadows = |shapes: &[Shape]| shapes.iter().filter(|s| matches!(s, Shape::Rect(r) if r.blur_width > 0.0)).count();
    for enabled in [true, false] {
        let ctx = context();
        let shapes = frame_at(&ctx, 0.0, vec![], |ui| {
            let _ = Slider::new("knob.slider", "Opacity", 0.5, 0.0..=1.0).enabled(enabled).show(ui);
            let _ = RangeSlider::new("knob.range", ("From", "To"), (0.2, 0.8), 0.0..=1.0).enabled(enabled).show(ui);
            let mut on = true;
            let _ = Switch::new("Autosave", &mut on).enabled(enabled).show(ui);
        });
        assert_eq!(shadows(&shapes), if enabled { 4 } else { 0 }, "enabled: {enabled}");
    }

    let ctx = context();
    let (rect, style) = (Cell::new(Rect::NOTHING), Cell::new(None));
    let frame = |time, events| {
        frame_at(&ctx, time, events, |ui| {
            style.set(Some(Slider::style(&Tokens::of(ui), ControlSize::Md)));
            rect.set(Slider::new("knob.ring", "Opacity", 0.5, 0.0..=1.0).show(ui).response.rect);
        })
    };
    let rings = |shapes: &[Shape], color: Color32| {
        let width = style.get().map_or(0.0, |s: SliderStyle| s.ring_width);
        shapes.iter().filter(|s| matches!(s, Shape::Rect(r) if r.stroke.color == color && r.stroke.width == width)).count()
    };
    let rest = frame(0.0, vec![]);
    let ring = style.get().map(|s| s.ring).expect("the slider is styled");
    assert_ne!(ring.hover, Color32::TRANSPARENT, "the suite default has a ring");
    assert_eq!((rings(&rest, ring.hover), rings(&rest, ring.pressed)), (0, 0), "no ring at rest");
    let center = rect.get().center();
    frame(0.016, vec![Event::PointerMoved(center)]);
    assert_eq!(rings(&frame(5.0, vec![]), ring.hover), 1, "a ring while hovered");
    frame(5.016, vec![press(center, true)]);
    assert_eq!(rings(&frame(10.0, vec![]), ring.pressed), 1, "a ring while pressed");
    frame(10.016, vec![press(center, false), Event::PointerGone]);
    assert_eq!(rings(&frame(15.0, vec![]), ring.hover), 0, "none once the pointer leaves");
}

/// A context showing the suite default, with reduced motion or not.
pub(super) fn motion_context(reduced: bool) -> Context {
    motion_context_in(craft_theme::ThemeId::suite_default().as_str(), reduced)
}

/// [`motion_context`] showing the built-in theme `id`.
pub(super) fn motion_context_in(id: &str, reduced: bool) -> Context {
    use craft_theme::{AppDefaults, ModeChoice, SuiteChoice, SystemAppearance, ThemePair};
    let defaults = AppDefaults { theme: ThemePair::same(theme(id).id), mode: ModeChoice::Light, size: Size::Compact };
    let system = SystemAppearance { reduced_motion: Some(reduced), ..Default::default() };
    let base = Base::resolve(&SuiteChoice::default().with_defaults(&defaults), system, &defaults, None, &mut |id| {
        builtins().iter().find(|t| t.id == *id).cloned()
    });
    let ctx = Context::default();
    install(&ctx, &base);
    ctx
}

/// One frame of `add` at `time` with `events`, outside any theme scope: the shapes, flattened.
pub(super) fn frame_at(ctx: &Context, time: f64, events: Vec<Event>, add: impl FnOnce(&mut Ui)) -> Vec<Shape> {
    fn flatten(shape: Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.into_iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), time: Some(time), events, ..Default::default() };
    let mut add = Some(add);
    let mut output = ctx.run_ui(input, |ui| {
        if let Some(add) = add.take() {
            add(ui);
        }
    });
    output.textures_delta.clear();
    let mut out = Vec::new();
    for clipped in output.shapes {
        flatten(clipped.shape, &mut out);
    }
    out
}

#[test]
fn a_switch_thumb_slides_from_the_reading_start_over_motion_move_and_at_once_with_reduced_motion() {
    use crate::direction::Direction;
    let t = tokens_of(&theme("craft.graphite"), Mode::Light);
    let style = Switch::style(&t, ControlSize::Md);
    let track = Rect::from_min_size(Pos2::ZERO, vec2(style.width, style.track_height));
    let reach = style.inset + style.thumb / 2.0;
    let x = |position, direction| Switch::thumb(&style, track, position, direction).x;
    assert_eq!([x(0.0, Direction::Ltr), x(0.5, Direction::Ltr), x(1.0, Direction::Ltr)], [reach, style.width / 2.0, style.width - reach]);
    assert_eq!([x(0.0, Direction::Rtl), x(1.0, Direction::Rtl)], [style.width - reach, reach], "off at the right, right to left");

    let travel = |reduced: bool| {
        let appearance = Appearance { reduced_motion: reduced, ..t.appearance };
        let tokens = UiTokens::new(resolve(&theme("craft.graphite"), appearance, None).tokens, appearance, Mode::Light, FontFamilies::default());
        Switch::style(&tokens, ControlSize::Md).travel
    };
    assert!(travel(false) > 0.0);
    assert_eq!(travel(true), 0.0);

    // Painted: the thumb is the knob's face, a filled square of the thumb's side; one frame after
    // a click it is on its way, or already at the end with reduced motion.
    for reduced in [false, true] {
        let ctx = motion_context(reduced);
        let (rect, on) = (Cell::new(Rect::NOTHING), Cell::new(false));
        let thumb_x = |shapes: &[Shape]| {
            let thumbs: Vec<f32> = shapes
                .iter()
                .filter_map(|s| match s {
                    Shape::Rect(r)
                        if r.blur_width == 0.0 && r.fill != Color32::TRANSPARENT && (r.rect.size() - Vec2::splat(style.thumb)).length() < 0.01 =>
                    {
                        Some(r.rect.center().x)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(thumbs.len(), 1, "one thumb");
            thumbs[0]
        };
        let frame = |time, events| {
            frame_at(&ctx, time, events, |ui| {
                let mut value = on.get();
                let edit = Switch::new("Autosave", &mut value).show(ui);
                rect.set(edit.response.rect);
                on.set(value);
            })
        };
        let start = thumb_x(&frame(0.0, vec![]));
        let center = rect.get().center();
        frame(0.016, vec![Event::PointerMoved(center), press(center, true)]);
        frame(0.032, vec![press(center, false)]);
        assert!(on.get());
        let next = thumb_x(&frame(0.048, vec![]));
        let end = thumb_x(&frame(5.0, vec![]));
        assert!(end > start, "on: at the reading end");
        if reduced {
            assert_eq!(next, end, "reduced motion: at once");
        } else {
            assert!(start < next && next < end, "{start} < {next} < {end}");
        }
    }
}

#[test]
fn bevel_themes_sink_check_boxes_radio_buttons_and_switch_tracks() {
    let classic = theme("craft.classic");
    let t = tokens_of(&classic, Mode::Light);
    let sunken = crate::primitives::Surface::style(&t, SurfaceKind::Sunken).edge;
    let crate::primitives::SurfaceEdge::Bevel { top_left: dark, bottom_right: light, .. } = sunken else { panic!("Classic bevels: {sunken:?}") };
    let ctx = context();
    for control in 0..3 {
        let rect = Cell::new(Rect::NOTHING);
        let shapes = pass(&ctx, &classic, vec![], |ui| {
            let response = match control {
                0 => Checkbox::bool("Grid", &mut true).show(ui).response,
                1 => {
                    let mut set = RadioSet::new("grid", "Grid", &mut 0).item(0).begin(ui);
                    let _ = set.radio(ui, 0, "Grid");
                    set.end(ui).response
                }
                _ => Switch::new("Grid", &mut false).show(ui).response,
            };
            rect.set(response.rect);
        });
        let s = Checkbox::style(&t, ControlSize::Md);
        // The box sits at the left, centred: its rect is the indicator's.
        let side = if control == 2 { vec2(Switch::style(&t, ControlSize::Md).width, s.side) } else { vec2(s.side, s.side) };
        let indicator = Rect::from_min_size(egui::pos2(rect.get().left(), rect.get().center().y - side.y / 2.0), side);
        // The radio is round: the bevel splits at 45 degrees through the corners, so compare the
        // top and bottom middle. Classic squares the switch track (`radius.pill` 0).
        let (top_left, bottom_right) = match control {
            1 => bevel_at(&shapes, indicator, indicator.center_top(), indicator.center_bottom()),
            _ => bevel_corners(&shapes, indicator),
        };
        assert_eq!((top_left, bottom_right), (dark, light), "control {control}: a sunken indicator, shaded on the top left");
    }
}

#[test]
fn every_change_is_one_set_with_the_modifiers_held_and_a_mixed_box_becomes_checked() {
    use crate::edit::EditEvent;
    let graphite = theme("craft.graphite");
    let ctx = context();
    let shift = Modifiers::SHIFT;
    let click = |pos: Pos2| {
        vec![
            Event::ModifiersChanged(shift),
            Event::PointerMoved(pos),
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: shift },
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: shift },
        ]
    };
    // Each control as a closure over its value, reporting its rect and event as text.
    let (rect, event) = (Cell::new(Rect::NOTHING), std::cell::RefCell::new(Vec::<String>::new()));
    let mut check = CheckState::Mixed;
    let mut flag = false;
    let mut unit = 0_usize;
    let mut group = 0_usize;
    let mut run = |control: usize, events: Vec<Event>| {
        pass(&ctx, &graphite, events, |ui| {
            let (response, text) = match control {
                0 => {
                    let e = Checkbox::new("Mixed", &mut check).show(ui);
                    (e.response, e.event.map(|e| format!("{e:?}")))
                }
                1 => {
                    let e = Switch::new("Autosave", &mut flag).show(ui);
                    (e.response, e.event.map(|e| format!("{e:?}")))
                }
                2 => {
                    let mut set = RadioSet::new("unit", "Unit", &mut unit).item(2).begin(ui);
                    let _ = set.radio(ui, 0, "Inches");
                    let e = set.end(ui);
                    (e.response, e.event.map(|e| format!("{e:?}")))
                }
                _ => {
                    let e = RadioGroup::new("units", "Units", &mut group).item(0, "Pixels").item(1, "Inches").show(ui);
                    (e.response, e.event.map(|e| format!("{e:?}")))
                }
            };
            rect.set(response.rect);
            event.borrow_mut().extend(text);
        });
    };
    let expected = [
        format!("{:?}", EditEvent::set(CheckState::Checked, shift)),
        format!("{:?}", EditEvent::set(true, shift)),
        format!("{:?}", EditEvent::set(2_usize, shift)),
        format!("{:?}", EditEvent::set(1_usize, shift)),
    ];
    for (control, expected) in expected.iter().enumerate() {
        run(control, vec![]);
        run(control, vec![]);
        // The radio group's second option is in its lower half.
        let r = rect.get();
        let at = if control == 3 { egui::pos2(r.left() + 4.0, r.bottom() - 4.0) } else { egui::pos2(r.left() + 4.0, r.center().y) };
        event.borrow_mut().clear();
        run(control, click(at));
        run(control, vec![]);
        assert_eq!(*event.borrow(), std::slice::from_ref(expected), "control {control}");
    }
    assert_eq!((check, flag, unit, group), (CheckState::Checked, true, 2, 1));
}

#[test]
fn text_field_styles_come_from_the_field_tokens() {
    for t in every_builtin() {
        let f = &t.component.field;
        for size in [ControlSize::Sm, ControlSize::Md] {
            let s = TextField::style(&t, size);
            assert_eq!((s.height, s.width, s.icon), (f.height.get(size), f.width.get(size), f.icon.get(size)));
        }
        let s = TextField::style(&t, ControlSize::Md);
        assert_eq!((s.pad_x, s.gap, s.role), (f.pad_x, f.gap, f.role));
        assert_eq!((s.surface.fill, s.surface.radius), (f.fill.rest.to_egui(), crate::theme::corner_radius(f.radius)));
        if let crate::primitives::SurfaceEdge::Border(stroke) = s.surface.edge {
            assert_eq!(stroke.color, f.border.rest.to_egui());
        }
        let colors = [s.fill_disabled, s.border_hover, s.border_focused, s.border_disabled, s.text, s.text_disabled, s.placeholder];
        let tokens = [f.fill.disabled, f.border.hover, f.border.focused, f.border.disabled, f.text.rest, f.text.disabled, f.placeholder.rest];
        assert_eq!(colors, tokens.map(|c| c.to_egui()));
        let colors = [s.placeholder_disabled, s.selection, s.selected_text, s.caret, s.invalid];
        assert_eq!(colors, [f.placeholder.disabled, f.selection, f.selected_text, f.caret, f.invalid].map(|c| c.to_egui()));
    }
}

/// The edge strokes of `shapes` in `color`, by width.
fn strokes_in(shapes: &[Shape], color: Color32) -> Vec<f32> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => Some(r.stroke.width),
            _ => None,
        })
        .collect()
}

/// A field in `theme`, focused when `focus`: its style and the shapes of a pass.
fn field_pass(theme: &ThemeDef, invalid: bool, focus: bool) -> (TextFieldStyle, Vec<Shape>) {
    let ctx = context();
    let id = Cell::new(egui::Id::NULL);
    let show = |ui: &mut Ui| id.set(TextField::new("edge", "Name", "text").invalid(invalid).show(ui).response.id);
    pass(&ctx, theme, vec![], show);
    if focus {
        ctx.memory_mut(|m| m.request_focus(id.get()));
    }
    let shapes = pass(&ctx, theme, vec![], show);
    (TextField::style(&tokens_of(theme, Mode::Light), ControlSize::Md), shapes)
}

/// The rects of the strokes in `color`.
fn stroke_rects(shapes: &[Shape], color: Color32) -> Vec<Rect> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => Some(r.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_field_edge_shows_focus_and_an_invalid_value_and_a_bevel_keeps_its_colours() {
    let flat = theme("craft.pro");
    let (s, shapes) = field_pass(&flat, false, true);
    assert_eq!(strokes_in(&shapes, s.border_focused), [1.0], "a focused edge in `field.border.focused`");
    let (s, shapes) = field_pass(&flat, true, false);
    assert_eq!(strokes_in(&shapes, s.invalid), [1.0]);
    let (s, shapes) = field_pass(&flat, true, true);
    assert_eq!(strokes_in(&shapes, s.border_focused), [1.0], "focused and invalid: the focused edge");
    assert_eq!(strokes_in(&shapes, s.invalid), [1.0], "and the invalid colour as a line inside it");
    let (edge, inner) = (stroke_rects(&shapes, s.border_focused), stroke_rects(&shapes, s.invalid));
    assert_eq!(inner, [edge[0].shrink(1.0)]);

    let classic = theme("craft.classic");
    let (s, shapes) = field_pass(&classic, false, false);
    assert!(matches!(s.surface.edge, crate::primitives::SurfaceEdge::Bevel { .. }));
    assert!(strokes_in(&shapes, s.border_focused).is_empty() && strokes_in(&shapes, s.invalid).is_empty());
    let (s, shapes) = field_pass(&classic, false, true);
    assert_eq!(strokes_in(&shapes, s.border_focused), [1.0], "a focused bevel draws the edge inside it");
    let (s, shapes) = field_pass(&classic, true, true);
    assert_eq!((strokes_in(&shapes, s.border_focused), strokes_in(&shapes, s.invalid)), (vec![1.0], vec![1.0]));
    let (edge, inner) = (stroke_rects(&shapes, s.border_focused), stroke_rects(&shapes, s.invalid));
    assert_eq!(inner, [edge[0].shrink(1.0)], "both inside the bevel, the invalid line innermost");
}

/// The badge dot of the icon button in `ui`: the circle filled with `color`.
fn badge_circle(shapes: &[Shape], color: Color32) -> Option<egui::epaint::CircleShape> {
    shapes.iter().find_map(|s| if let Shape::Circle(c) = s { (c.fill == color).then_some(*c) } else { None })
}

#[test]
fn badge_stays_visible_on_light_and_dark_fills() {
    use craft_theme::Srgba;
    let ctx = context();
    for t in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            let tokens = tokens_of(t, mode);
            for color in [Color32::BLACK, Color32::from_gray(128), Color32::WHITE] {
                for on in [false, true] {
                    let shapes = frame_at(&ctx, 0.0, vec![], |ui| {
                        crate::theme::scope(
                            ui,
                            |o| (o.theme, o.mode) = (Some(t.clone()), Some(mode)),
                            |ui| {
                                ui.add(IconButton::new(crate::icons::Icon::Eye, "Set point").selected(on).badge(Srgba::from(color)));
                            },
                        );
                    });
                    let dot = badge_circle(&shapes, color).expect("the badge is painted");
                    let style = IconButton::style(&tokens, ControlSize::Md);
                    assert_eq!(dot.radius, style.badge / 2.0);
                    assert_eq!(dot.stroke, style.badge_edge, "{} {mode:?}: the ring is line.mark", t.id.as_str());
                }
            }
        }
    }
}

/// The slider's marks and ticks and the icon button's badge ring are meaningful graphics: they
/// reach the non-text minimum on the control surfaces and the hover and pressed fills.
#[test]
fn marks_and_the_badge_ring_read_on_the_control_surfaces() {
    for t in builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            for (contrast, min) in [(Contrast::Standard, 3.0), (Contrast::High, 4.5)] {
                let appearance = Appearance { mode, contrast, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
                let tokens = UiTokens::new(resolve(t, appearance, None).tokens, appearance, mode, FontFamilies::default());
                let c = &tokens.color;
                let ring = IconButton::style(&tokens, ControlSize::Md).badge_edge;
                let mark = Slider::style(&tokens, ControlSize::Md).mark_color;
                assert_eq!(ring.width, 1.0, "the ring is the W1 stroke");
                for (what, colour) in [("badge ring", ring.color), ("slider mark", mark)] {
                    for bg in [c.surface.panel, c.surface.raised, c.surface.sunken, c.state.hover, c.state.pressed] {
                        let ratio = contrast_ratio(craft_theme::Srgba::from(colour).into(), bg);
                        assert!(ratio >= min, "{} {mode:?} {contrast:?}: the {what} on {} is {ratio:.2}:1", t.id.as_str(), bg.to_hex());
                    }
                }
            }
        }
    }
}

#[test]
fn badge_is_at_the_end_corner_in_both_directions() {
    use crate::direction::{self, Direction};
    let ctx = context();
    for (dir, end_is_right) in [(Direction::Ltr, true), (Direction::Rtl, false)] {
        direction::apply(&ctx, dir);
        let rect = Cell::new(Rect::NOTHING);
        let shapes = frame_at(&ctx, 0.0, vec![], |ui| {
            rect.set(ui.add(IconButton::new(crate::icons::Icon::Eye, "Set point").badge(Color32::RED)).rect);
        });
        let dot = badge_circle(&shapes, Color32::RED).unwrap();
        let rect = rect.get();
        assert!(dot.center.y > rect.center().y, "{dir:?}: the bottom half");
        assert_eq!(dot.center.x > rect.center().x, end_is_right, "{dir:?}: the reading end");
        assert!(rect.expand(0.5).contains(dot.center));
    }
}
