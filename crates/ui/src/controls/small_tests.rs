//! Swatch, Badge, Kbd, Spinner, Progress and Tooltip: styles from tokens and what they paint.

use std::cell::Cell;

use craft_theme::{BadgeStyle, BadgeTone, ControlSize, Mode, Rgba, contrast_ratio};
use egui::os::OperatingSystem;
use egui::{Color32, Context, Event, Key, KeyboardShortcut, ModifierNames, Modifiers, Pos2, RawInput, Rect, Shape, Ui, ViewportId, vec2};

use super::tests::{every_builtin, frame_at, motion_context, motion_context_in, theme, tokens_of};
use super::*;
use crate::icons::IconSize;
use crate::primitives::{Content, Surface, SurfaceKind};
use crate::theme::{ToEgui, Tokens, TokensOf as _};

#[test]
fn small_control_styles_are_their_tokens() {
    for t in every_builtin() {
        let c = &t.component;
        for size in SIZES {
            let s = Swatch::style(&t, size);
            assert_eq!((s.side, s.radius), (c.swatch.size.get(size), crate::theme::corner_radius(c.swatch.radius)));
            assert_eq!((s.edge, s.inset), (c.swatch.edge.to_egui(), c.swatch.selected_gap + c.swatch.selected_width));
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
        assert_eq!((tip.fill, tip.border.color), (c.tooltip.fill.to_egui(), c.tooltip.border.to_egui()));
        assert_eq!((tip.radius, tip.shadow), (crate::theme::corner_radius(c.tooltip.radius), t.shadow.popup));
        assert_eq!(tip.margin, crate::theme::margin(t.space.pad.control));
        assert_eq!(tip.text, c.tooltip.text.to_egui());
    }
}

#[test]
fn a_chord_is_written_in_each_platforms_notation() {
    let all = Modifiers { alt: true, ctrl: true, shift: true, mac_cmd: true, command: false };
    let chord = KeyboardShortcut::new(all, Key::K);
    let text = |shortcut: &KeyboardShortcut, os| Kbd::new(shortcut).text(os);
    assert_eq!(text(&chord, OperatingSystem::Mac), "⌃⌥⇧⌘K", "Apple's order");
    assert_eq!(text(&chord, OperatingSystem::Windows), "Ctrl+Alt+Shift+K");
    assert_eq!(Kbd::parts(&chord, OperatingSystem::Nix), ["Ctrl", "Alt", "Shift", "K"]);
    // `command` is Cmd on macOS and Ctrl elsewhere.
    let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
    assert_eq!(Kbd::parts(&save, OperatingSystem::Mac), ["⌘", "S"]);
    assert_eq!(Kbd::parts(&save, OperatingSystem::Nix), ["Ctrl", "S"]);
    assert_eq!(text(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape), OperatingSystem::Mac), "⎋", "the suite's glyphs for editing keys on a Mac");
    assert_eq!(text(&KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp), OperatingSystem::Mac), "↑");
    assert_eq!(text(&KeyboardShortcut::new(Modifiers::NONE, Key::ArrowUp), OperatingSystem::Nix), "Up");
    assert_eq!(text(&KeyboardShortcut::new(Modifiers::NONE, Key::PageDown), OperatingSystem::Nix), "Page Down");
    // Keys the caller names are joined with `+` on every platform.
    for os in [OperatingSystem::Mac, OperatingSystem::Nix] {
        assert_eq!(Kbd::keys(&["Shift", "Click"]).text(os), "Shift+Click", "{os:?}");
    }
    // Elsewhere the notation is egui's own; on a Mac the modifiers take Apple's order, not egui's
    // ⌃⇧⌥⌘, in the symbols and in the names AccessKit reads.
    for shortcut in [chord, save] {
        assert_eq!(text(&shortcut, OperatingSystem::Nix), shortcut.format(&ModifierNames::NAMES, false));
    }
    let option_shift = KeyboardShortcut::new(Modifiers { alt: true, shift: true, ..Modifiers::COMMAND }, Key::S);
    assert_eq!(text(&option_shift, OperatingSystem::Mac), "⌥⇧⌘S");
    assert_eq!(Kbd::new(&chord).spoken(OperatingSystem::Mac), "Ctrl+Option+Shift+Cmd+K");
    assert_eq!(Kbd::new(&chord).spoken(OperatingSystem::Windows), "Ctrl+Alt+Shift+K");
}

/// The tokens in effect in `ctx`.
fn tokens_in(ctx: &Context) -> std::sync::Arc<crate::theme::UiTokens> {
    let tokens = std::cell::RefCell::new(None);
    frame(ctx, 0.0, |ui| *tokens.borrow_mut() = Some(Tokens::of(ui)));
    tokens.into_inner().unwrap()
}

/// The colour of each text shape `add` paints.
fn text_colors(ctx: &Context, add: impl FnOnce(&mut Ui)) -> Vec<Color32> {
    let (shapes, _) = frame(ctx, 0.0, add);
    shapes.iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.galley.job.sections[0].format.color) } else { None }).collect()
}

#[test]
fn plain_keys_take_the_content_colour_and_keycaps_their_own() {
    let ctx = motion_context(true);
    let chord = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
    let t = tokens_in(&ctx);
    assert_eq!(text_colors(&ctx, |ui| _ = ui.add(Kbd::new(&chord))), [t.color.text.muted.to_egui()], "muted by default");
    assert_eq!(text_colors(&ctx, |ui| _ = ui.add(Kbd::new(&chord).content(Content::OnSelected))), [t.color.text.on_selected.to_egui()]);
    let keycaps = text_colors(&ctx, |ui| _ = ui.add(Kbd::new(&chord).keycaps(true).content(Content::OnSelected)));
    assert!(keycaps.iter().all(|&c| c == t.component.kbd.text.to_egui()), "{keycaps:?}");
}

/// Badges and keycaps grow with the Text size setting: at the largest, the label still sits inside
/// the pill and each key inside its keycap.
#[test]
fn badge_and_keycap_labels_fit_at_the_largest_text_size() {
    use craft_theme::{AppDefaults, ModeChoice, Size, SuiteChoice, SystemAppearance, TextSize, ThemeId, ThemePair};
    let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::Light, size: Size::Comfortable };
    let mut choice = SuiteChoice::default().with_defaults(&defaults);
    choice.text_size = TextSize::Large;
    let base = crate::theme::Base::resolve(&choice, SystemAppearance::default(), &defaults, None, &mut |id| {
        craft_theme::builtins().iter().find(|t| t.id == *id).cloned()
    });
    let ctx = Context::default();
    crate::theme::install(&ctx, &base);
    assert_eq!(tokens_in(&ctx).appearance.text_size, TextSize::Large);
    let boxes_and_texts = |add: &dyn Fn(&mut Ui)| {
        let (shapes, _) = frame(&ctx, 0.0, add);
        let boxes: Vec<Rect> = shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (r.fill.a() > 0).then_some(r.rect) } else { None }).collect();
        let texts: Vec<Rect> = shapes.iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.visual_bounding_rect()) } else { None }).collect();
        (boxes, texts)
    };
    let (pills, labels) = boxes_and_texts(&|ui| _ = ui.add(Badge::count(1234).tone(BadgeTone::Bad).style(BadgeStyle::Solid)));
    let (keycaps, keys) = boxes_and_texts(&|ui| _ = ui.add(Kbd::keys(&["Ctrl", "Shift", "Y"]).keycaps(true)));
    for (boxes, texts, what) in [(pills, labels, "badge"), (keycaps, keys, "keycap")] {
        assert!(!texts.is_empty(), "{what}");
        for text in texts {
            assert!(boxes.iter().any(|b| b.contains_rect(text)), "{what}: {text:?} outside {boxes:?}");
        }
    }
}

/// The shapes of a swatch in `style` over a 30 point rect.
fn well(style: &SwatchStyle, color: Option<Rgba>, selected: bool, enabled: bool) -> Vec<Shape> {
    Swatch::shapes(style, Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(30.0, 30.0)), color, selected, enabled, 1.0)
}

/// The colours of the stroked rects among `shapes`, in paint order.
fn strokes(shapes: &[Shape]) -> Vec<Color32> {
    shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { (!r.stroke.is_empty()).then_some(r.stroke.color) } else { None }).collect()
}

#[test]
fn a_swatch_shows_mixed_and_selected_and_its_rings_for_any_colour() {
    let t = tokens_of(&theme("craft.graphite"), Mode::Light);
    let style = Swatch::style(&t, ControlSize::Md);
    let red = Rgba::rgb(200, 30, 30);
    let plain = well(&style, Some(red), false, true);
    assert_ne!(well(&style, None, false, true), plain, "mixed");
    assert_ne!(well(&style, Some(red), true, true), plain, "selected");
    assert_eq!(well(&style, Some(red), true, true).len(), plain.len() + 1, "the selection ring is one more shape");
    assert_ne!(well(&style, Some(red), false, false), plain, "disabled fades the rings");

    // Opaque colours have no checkerboard; translucent ones do.
    assert!(well(&style, Some(red.with_alpha(90)), false, true).len() > plain.len());

    // The well sits inside the selection ring's room, and every ring is concentric with it.
    let outline = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(30.0, 30.0));
    let rects: Vec<_> = well(&style, Some(red), true, true).into_iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r) } else { None }).collect();
    let [.., inner, edge, selected] = rects.as_slice() else { panic!("{rects:?}") };
    assert_eq!(edge.rect, outline.shrink(style.inset), "the edge is on the well's outline");
    assert_eq!((inner.rect, edge.corner_radius, selected.rect), (edge.rect.shrink(1.0), style.radius, outline));
    assert_eq!(selected.corner_radius, Swatch::outline_radius(&style));
    let grown = (style.inset.round() as u8) + style.radius.nw;
    assert_eq!(selected.corner_radius.nw, grown, "concentric");
    assert!(style.inset > style.selected.width, "a gap between the well and the selection ring");

    // A colour equal to the panel: the inner ring stands out from it by a measured ratio, in a
    // light and a dark theme.
    for (id, mode) in [("craft.graphite", Mode::Light), ("craft.pro", Mode::Dark)] {
        let t = tokens_of(&theme(id), mode);
        let style = Swatch::style(&t, ControlSize::Md);
        let panel = t.color.surface.panel;
        let ring = strokes(&well(&style, Some(panel), false, true))[0];
        let [r, g, b, a] = ring.to_srgba_unmultiplied();
        let ratio = contrast_ratio(over(Rgba([r, g, b, a]), panel), panel);
        assert!(ratio >= 3.0, "{id}: the inner ring on the panel colour is {ratio:.2}:1");
    }

    // The hex keeps the straight channels at any alpha.
    assert_eq!(Swatch::hex(Rgba::rgb(0x3e, 0x63, 0xdd)), "#3e63dd");
    assert_eq!(Swatch::hex(Rgba([0xff, 0x00, 0x00, 0x00])), "#ff000000");
    assert_eq!(Swatch::hex(Rgba([0x12, 0x34, 0x56, 0x03])), "#12345603");
    assert_eq!(Swatch::hex(Rgba([0xff, 0x00, 0x00, 0x80])), "#ff000080");
    assert_eq!(Rgba::from(Color32::from_rgba_unmultiplied(0xff, 0x00, 0x00, 0x80)), Rgba([0xff, 0x00, 0x00, 0x80]), "from an egui colour");
}

/// `fg` alpha-composited over the opaque `bg`, as egui blends.
fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f32::from(fg.a()) / 255.0;
    let ch = |i: usize| (f32::from(fg.0[i]) * a + f32::from(bg.0[i]) * (1.0 - a)).round() as u8;
    Rgba([ch(0), ch(1), ch(2), 255])
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
    // The ring is the spinner's own polyline, still at turn 0 with reduced motion, in an icon's
    // square: the ends sit at 0° and 270°, its widest points at 90° and 180°.
    let (shapes, _) = frame(&ctx, 0.0, |ui| {
        ui.add(Button::new("Export").loading(true));
    });
    let points = shapes.into_iter().find_map(|s| if let Shape::Path(p) = s { Some(p.points) } else { None }).unwrap();
    let quarter = (points.len() - 1) / 3;
    let center = Pos2::new((points[0].x + points[2 * quarter].x) / 2.0, (points[quarter].y + points[3 * quarter].y) / 2.0);
    let side = Spinner::style(&tokens_in(&ctx), IconSize::Md, Content::Muted).points;
    let expected = Spinner::points(Rect::from_center_size(center, vec2(side, side)), 0.0);
    assert_eq!(points.len(), expected.len());
    for (p, e) in points.iter().zip(&expected) {
        assert!(p.distance(*e) < 1e-3, "{p:?} is not the spinner's {e:?}");
    }
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

/// The filled, outlined rects of the first frame at which a tooltip shows after hovering the
/// button that `add` puts in the context's theme, and the text colours drawn.
fn hovered_tooltip(id: &str, add: impl Fn(&mut Ui) -> egui::Response) -> (Vec<(Color32, egui::Stroke, egui::CornerRadius)>, Vec<Color32>) {
    let ctx = motion_context_in(id, true);
    ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
    let rect = Cell::new(Rect::NOTHING);
    let run = |time: f64, events: Vec<Event>| frame_at(&ctx, time, events, |ui| rect.set(add(ui).rect));
    run(0.0, vec![]);
    run(0.1, vec![Event::PointerMoved(rect.get().center())]);
    run(0.2, vec![]);
    let shapes = run(0.4, vec![]);
    let frames = shapes
        .iter()
        .filter_map(|s| if let Shape::Rect(r) = s { (!r.stroke.is_empty()).then_some((r.fill, r.stroke, r.corner_radius)) } else { None })
        .collect();
    let texts =
        shapes.iter().filter_map(|s| if let Shape::Text(t) = s { t.galley.job.sections.first().map(|s| s.format.color) } else { None }).collect();
    (frames, texts)
}

#[test]
fn pointer_tooltips_draw_the_themed_card() {
    let tip = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).tooltip("Moves the layer");
    let at_pointer = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).tooltip_at_pointer("Moves the layer");
    let at_pointer_ui = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).tooltip_ui_at_pointer(|ui| drop(ui.label("Moves the layer")));
    let (frames, texts) = hovered_tooltip("craft.classic", tip);
    let (pointer_frames, pointer_texts) = hovered_tooltip("craft.classic", at_pointer);
    assert_eq!(pointer_frames, frames, "the same card as a tooltip on the control");
    assert_eq!(pointer_texts.last(), texts.last());
    assert_eq!(hovered_tooltip("craft.classic", at_pointer_ui).0, frames);
}

#[test]
fn classic_tooltips_draw_a_pale_yellow_card_with_black_text_and_edge() {
    let tip = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).tooltip("Moves the layer");
    let (frames, texts) = hovered_tooltip("craft.classic", tip);
    let (yellow, black) = (Color32::from_rgb(0xff, 0xff, 0xe1), Color32::BLACK);
    let card: Vec<_> = frames.iter().filter(|f| f.0 == yellow).collect();
    assert_eq!(card.len(), 1, "one tooltip card: {frames:?}");
    assert_eq!((card[0].1.color, card[0].1.width, card[0].2), (black, 1.0, egui::CornerRadius::ZERO));
    assert_eq!(texts.last(), Some(&black), "the tooltip text is drawn last");
}

#[test]
fn other_themes_tooltips_keep_the_popup_look() {
    for id in ["craft.artcraft", "craft.soft"] {
        let tokens = tokens_of(&theme(id), Mode::Light);
        let popup = egui::Frame::popup(&tokens.style);
        let tip = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).tooltip("Moves the layer");
        let stock = |ui: &mut Ui| ui.add(Button::new("Target").id_salt("tip")).on_hover_text("Moves the layer");
        let (frames, texts) = hovered_tooltip(id, tip);
        let card = (popup.fill, popup.stroke, popup.corner_radius);
        assert!(frames.contains(&card), "{id}: the popup frame is drawn: {frames:?}");
        assert_eq!(texts.last(), Some(&tokens.color.text.normal.to_egui()), "{id}");
        assert_eq!(hovered_tooltip(id, stock).0, frames, "{id}: as stock egui draws it");
    }
}
