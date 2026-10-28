//! Swatch, Badge, Kbd, Spinner, Progress and Tooltip: styles from tokens and what they paint.

use std::cell::Cell;

use craft_theme::{BadgeStyle, BadgeTone, ControlSize, Mode};
use egui::os::OperatingSystem;
use egui::{Color32, Context, Event, Key, KeyboardShortcut, ModifierNames, Modifiers, Pos2, RawInput, Rect, Shape, Ui, ViewportId, vec2};

use super::tests::{every_builtin, frame_at, motion_context, theme, tokens_of};
use super::*;
use crate::icons::IconSize;
use crate::primitives::{Content, Corner, Surface, SurfaceKind};
use crate::theme::{ToEgui, Tokens, TokensOf as _};

#[test]
fn small_control_styles_are_their_tokens() {
    for t in every_builtin() {
        let c = &t.component;
        for size in SIZES {
            let s = Swatch::style(&t, size);
            assert_eq!((s.side, s.radius), (c.swatch.size.get(size), crate::theme::corner_radius(c.swatch.radius)));
            assert_eq!((s.edge_outer, s.edge_inner), (c.swatch.edge_outer.to_egui(), c.swatch.edge_inner.to_egui()));
            assert_eq!(s.checker, [c.swatch.checker_a.to_egui(), c.swatch.checker_b.to_egui()]);
            assert_eq!((s.selected.color, s.selected.width), (c.swatch.selected.to_egui(), c.swatch.selected_width));
            assert_eq!((s.mixed_fill, s.mixed_mark), (c.swatch.mixed_fill.to_egui(), c.swatch.mixed_mark.to_egui()));
        }
        for tone in BadgeTone::ALL {
            for style in BadgeStyle::ALL {
                let look = Badge::look(&t, tone, style);
                let colors = c.badge.colors(tone, style);
                assert_eq!((look.fill, look.text, look.border), (colors.fill.to_egui(), colors.text.to_egui(), colors.border.to_egui()));
                assert_eq!((look.height, look.pad_x, look.label), (c.badge.height, c.badge.pad_x, c.badge.label));
            }
        }
        // Status tones read the status tokens straight.
        let good = Badge::look(&t, BadgeTone::Good, BadgeStyle::Soft);
        assert_eq!(
            (good.fill, good.text, good.border),
            (t.color.status.good.bg.to_egui(), t.color.status.good.fg.to_egui(), t.color.status.good.border.to_egui())
        );
        let bad = Badge::look(&t, BadgeTone::Bad, BadgeStyle::Solid);
        assert_eq!((bad.fill, bad.text), (t.color.status.bad.solid.to_egui(), t.color.text.on_status.to_egui()));

        let k = Kbd::style(&t);
        assert_eq!((k.height, k.pad_x, k.text), (c.kbd.height, c.kbd.pad_x, c.kbd.text.to_egui()));
        let raised = Surface::style(&t, SurfaceKind::Raised);
        assert_eq!((k.keycap.edge, k.keycap.fill), (raised.edge, c.kbd.fill.to_egui()), "a keycap is a raised surface");

        let p = Progress::style(&t);
        assert_eq!((p.height, p.fill, p.track.fill), (c.progress.track_height, c.progress.fill.to_egui(), c.progress.track.to_egui()));
        assert_eq!(p.track.edge, Surface::style(&t, SurfaceKind::Sunken).edge);

        let tip = Tooltip::style(&t);
        assert_eq!(tip.max_width, c.tooltip.max_width);
        assert_eq!(tip.max_width, t.style.spacing.tooltip_width, "stock tooltips wrap at the same width");
        assert_eq!((tip.fill, tip.border.color), (t.color.surface.raised.to_egui(), t.color.line.border.to_egui()));
        assert_eq!((tip.radius, tip.shadow), (Corner::Popup.radius(&t), t.shadow.popup));
        assert_eq!(tip.margin, crate::theme::margin(t.space.pad.control));
        assert_eq!(tip.text, t.color.text.normal.to_egui());
    }
}

#[test]
fn a_chord_is_written_in_each_platforms_notation() {
    let all = Modifiers { alt: true, ctrl: true, shift: true, mac_cmd: true, command: false };
    let chord = KeyboardShortcut::new(all, Key::K);
    let text = |os| Kbd::text(&Kbd::parts(&chord, os), os);
    assert_eq!(text(OperatingSystem::Mac), "⌃⇧⌥⌘K");
    assert_eq!(text(OperatingSystem::Windows), "Ctrl+Alt+Shift+K");
    assert_eq!(Kbd::parts(&chord, OperatingSystem::Nix), ["Ctrl", "Alt", "Shift", "K"]);
    // `command` is Cmd on macOS and Ctrl elsewhere.
    let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
    assert_eq!(Kbd::parts(&save, OperatingSystem::Mac), ["⌘", "S"]);
    assert_eq!(Kbd::parts(&save, OperatingSystem::Nix), ["Ctrl", "S"]);
    assert_eq!(Kbd::text(&Kbd::parts(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape), OperatingSystem::Mac), OperatingSystem::Mac), "Escape");
    // The notation is egui's own.
    for os in [OperatingSystem::Mac, OperatingSystem::Nix] {
        let mac = matches!(os, OperatingSystem::Mac);
        let names = if mac { ModifierNames::SYMBOLS } else { ModifierNames::NAMES };
        for shortcut in [chord, save] {
            assert_eq!(Kbd::text(&Kbd::parts(&shortcut, os), os), shortcut.format(&names, mac), "{os:?}");
        }
    }
}

/// The shapes of a swatch well in `style` over a 24 point rect.
fn well(style: &SwatchStyle, color: Option<Color32>, selected: bool, enabled: bool) -> Vec<Shape> {
    Swatch::shapes(style, Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(24.0, 24.0)), color, selected, enabled, 1.0)
}

#[test]
fn a_swatch_shows_mixed_and_selected_and_its_rings_for_any_colour() {
    let t = tokens_of(&theme("craft.graphite"), Mode::Light);
    let style = Swatch::style(&t, ControlSize::Md);
    let red = Color32::from_rgb(200, 30, 30);
    let plain = well(&style, Some(red), false, true);
    assert_ne!(well(&style, None, false, true), plain, "mixed");
    assert_ne!(well(&style, Some(red), true, true), plain, "selected");
    assert_eq!(well(&style, Some(red), true, true).len(), plain.len() + 1, "the selection ring is one more shape");
    assert_ne!(well(&style, Some(red), false, false), plain, "disabled fades the rings");

    // Opaque colours have no checkerboard; translucent ones do.
    let clear = Color32::from_rgba_unmultiplied(200, 30, 30, 90);
    let strokes = |shapes: &[Shape]| -> Vec<Color32> {
        shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (!r.stroke.is_empty()).then_some(r.stroke.color) } else { None }).collect()
    };
    assert!(well(&style, Some(clear), false, true).len() > plain.len());

    // A colour equal to the panel: the outer ring separates the well from the panel (it differs
    // from it) and the inner ring from the colour.
    let panel = t.color.surface.panel.to_egui();
    let shapes = well(&style, Some(panel), false, true);
    assert_eq!(strokes(&shapes), [style.edge_inner, style.edge_outer]);
    assert_ne!(style.edge_outer, panel);
    assert_ne!(style.edge_inner, style.edge_outer);
    let outer = shapes.iter().rev().find_map(|s| if let Shape::Rect(r) = s { Some(r) } else { None }).unwrap();
    assert_eq!(outer.rect, Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(24.0, 24.0)), "the outer ring is on the outline");

    assert_eq!(Swatch::hex(Color32::from_rgb(0x3e, 0x63, 0xdd)), "#3e63dd");
    assert_eq!(Swatch::hex(Color32::from_rgba_unmultiplied(0xff, 0x00, 0x00, 0x80)), "#ff000080");
}

/// One frame at `time`; the shapes and whether it asked for another frame at once.
fn frame(ctx: &Context, time: f64, add: impl FnOnce(&mut Ui)) -> (Vec<Shape>, bool) {
    let mut add = Some(add);
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), time: Some(time), ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| {
        if let Some(add) = add.take() {
            add(ui);
        }
    });
    output.textures_delta.clear();
    let repaint = output.viewport_output.get(&ViewportId::ROOT).is_some_and(|v| v.repaint_delay.is_zero());
    let mut out = Vec::new();
    fn flatten(shape: Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.into_iter().for_each(|s| flatten(s, out)),
            shape => out.push(shape),
        }
    }
    output.shapes.into_iter().for_each(|c| flatten(c.shape, &mut out));
    (out, repaint)
}

#[test]
fn a_spinner_turns_and_asks_for_frames_unless_motion_is_reduced() {
    let spin = |reduced: bool| {
        let ctx = motion_context(reduced);
        // The first frames carry egui's own repaint requests.
        for time in [0.0, 0.1, 0.2] {
            frame(&ctx, time, |ui| {
                ui.add(Spinner::new().size(IconSize::Lg));
            });
        }
        let a = frame(&ctx, 1.0, |ui| {
            ui.add(Spinner::new().size(IconSize::Lg));
        });
        let b = frame(&ctx, 1.3, |ui| {
            ui.add(Spinner::new().size(IconSize::Lg));
        });
        (a, b)
    };
    let (a, b) = spin(false);
    assert!(a.1 && b.1, "moving: a repaint every frame");
    assert_ne!(a.0, b.0, "moving: the ring turns");
    let (a, b) = spin(true);
    assert!(!a.1 && !b.1, "reduced motion: no repaint");
    assert_eq!(a.0, b.0, "reduced motion: the same ring");
    assert!(a.0.iter().any(|s| matches!(s, Shape::Path(_))), "a ring is drawn");

    // Out of view it asks for nothing.
    let ctx = motion_context(false);
    let far = |ui: &mut Ui| {
        ui.add_space(500.0);
        ui.add(Spinner::new());
    };
    for time in [0.0, 0.1, 0.2] {
        frame(&ctx, time, far);
    }
    let (_, repaint) = frame(&ctx, 0.5, far);
    assert!(!repaint, "a spinner nobody sees does not keep the app awake");
}

#[test]
fn a_button_draws_its_loading_ring_through_the_spinner() {
    let ctx = motion_context(true);
    let ring = |loading: bool| {
        let (shapes, _) = frame(&ctx, 0.0, |ui| {
            ui.add(Button::new("Export").loading(loading));
        });
        shapes.into_iter().filter_map(|s| if let Shape::Path(p) = s { Some(p) } else { None }).count()
    };
    assert_eq!((ring(false), ring(true)), (0, 1));
    // The ring is the spinner's own polyline.
    let t = tokens_of(&theme("craft.graphite"), Mode::Light);
    let icon = Spinner::style(&t, IconSize::Md, Content::Muted);
    assert_eq!(Spinner::points(Rect::from_min_size(Pos2::ZERO, vec2(icon.points, icon.points)), 0.0).len(), 25);
}

#[test]
fn a_progress_fill_is_the_clamped_fraction_of_the_track_and_nan_draws_empty() {
    let ctx = motion_context(true);
    // The widths of the rects filled with the bar's fill colour: the fill, if there is one.
    let fills = |bar: Progress| -> Vec<f32> {
        let color = Cell::new(Color32::PLACEHOLDER);
        let (shapes, _) = frame(&ctx, 0.0, |ui| {
            color.set(Progress::style(&Tokens::of(ui)).fill);
            ui.add(bar.width(200.0));
        });
        shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (r.fill == color.get()).then_some(r.rect.width()) } else { None }).collect()
    };
    assert_eq!(fills(Progress::new(0.25)), [50.0]);
    assert_eq!(fills(Progress::new(1.0)), [200.0]);
    assert_eq!(fills(Progress::new(1.7)), [200.0], "clamped");
    assert!(fills(Progress::new(-3.0)).is_empty(), "negative draws empty");
    assert!(fills(Progress::new(f32::NAN)).is_empty(), "NaN draws empty");
    assert_eq!((Progress::clamped(f32::NAN), Progress::clamped(0.5)), (0.0, 0.5));
}

#[test]
fn an_indeterminate_bar_moves_and_stands_still_as_dashes_with_reduced_motion() {
    let bar = |reduced: bool, time: f64| {
        let ctx = motion_context(reduced);
        for settle in [0.0, 0.1, 0.2] {
            frame(&ctx, settle, |ui| {
                ui.add(Progress::indeterminate().width(200.0));
            });
        }
        frame(&ctx, time, |ui| {
            ui.add(Progress::indeterminate().width(200.0));
        })
    };
    // The track is the first rect; the rest is the fill.
    let fills =
        |shapes: &[Shape]| -> Vec<Rect> { shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r.rect) } else { None }).skip(1).collect() };
    let (a, repaint) = bar(false, 0.4);
    let (b, _) = bar(false, 0.9);
    assert!(repaint);
    assert_ne!(fills(&a), fills(&b), "the segment moves");
    let (still_a, repaint) = bar(true, 0.4);
    let (still_b, _) = bar(true, 0.9);
    assert!(!repaint, "reduced motion: no repaint");
    assert_eq!(fills(&still_a), fills(&still_b));
    assert!(fills(&still_a).len() > 1, "a pattern of dashes, not a solid fill");
}

#[test]
fn tooltips_draw_the_same_frame_stock_or_helper_and_disabled_controls_keep_theirs() {
    let tip = Cell::new(None);
    let hover = |add: &dyn Fn(&mut Ui) -> egui::Response| {
        let ctx = motion_context(true);
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        let rect = Cell::new(Rect::NOTHING);
        let run = |time: f64, events: Vec<Event>| {
            frame_at(&ctx, time, events, |ui| {
                tip.set(Some(Tooltip::style(&Tokens::of(ui))));
                rect.set(add(ui).rect);
            })
        };
        run(0.0, vec![]);
        let center = rect.get().center();
        run(0.1, vec![Event::PointerMoved(center)]);
        run(0.2, vec![]);
        run(0.4, vec![])
    };
    let frames = |shapes: &[Shape]| -> Vec<(Color32, egui::Stroke, egui::CornerRadius)> {
        let tip = tip.get().unwrap();
        shapes
            .iter()
            .filter_map(|s| {
                if let Shape::Rect(r) = s {
                    (r.fill == tip.fill && !r.stroke.is_empty()).then_some((r.fill, r.stroke, r.corner_radius))
                } else {
                    None
                }
            })
            .collect()
    };
    let stock = hover(&|ui| ui.add(Button::new("Target").id_salt("tip")).on_hover_text("Moves the layer"));
    let helper = hover(&|ui| ui.add(Button::new("Target").id_salt("tip")).tooltip("Moves the layer"));
    let tip = tip.get().unwrap();
    assert_eq!(frames(&stock), [(tip.fill, tip.border, tip.radius)], "stock: the popup frame from the tokens");
    assert_eq!(frames(&helper), frames(&stock), "the helper draws the stock frame");
    // Disabled controls keep theirs.
    let disabled = hover(&|ui| ui.add(Button::new("Target").enabled(false).id_salt("tip")).tooltip("Moves the layer"));
    assert_eq!(frames(&disabled), frames(&stock));
}
