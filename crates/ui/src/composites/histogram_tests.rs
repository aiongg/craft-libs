//! Histogram tests: scales, overlaps, degenerate input, the AccessKit node and the stale mark.

use craft_theme::Mode;
use egui::accesskit::{Action, ActionData, ActionRequest, Role, TreeId};
use egui::{Color32, Context, Event, Key, Mesh, Modifiers, PointerButton, Pos2, Rect, ThemePreference, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::histogram::{
    self, Blend, ChannelArg, RgbArgs, ScaleArg, ToneArgs, outline, plot_shapes, product, rgb_heights, rgb_mesh, sum, tone_heights, tone_mesh,
};
use super::*;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install, tokens_in_mode};
use std::ops::RangeInclusive;

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([400.0, 240.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn rect() -> Rect {
    Rect::from_min_size(pos2(10.0, 20.0), vec2(256.0, 100.0))
}

fn flat(n: usize, v: f32) -> Vec<f32> {
    vec![v; n]
}

/// The colour of the mesh's triangle containing `point`.
fn color_at(mesh: &Mesh, point: Pos2) -> Option<Color32> {
    mesh.indices.as_chunks::<3>().0.iter().find_map(|t| {
        let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[t[i] as usize].pos);
        let cross = |u: egui::Vec2, v: egui::Vec2| u.x * v.y - u.y * v.x;
        if cross(b - a, c - a).abs() < 1e-5 {
            return None;
        }
        let sides = [cross(b - a, point - a), cross(c - b, point - b), cross(a - c, point - c)];
        (sides.iter().all(|&s| s >= -1e-5) || sides.iter().all(|&s| s <= 1e-5)).then_some(mesh.vertices[t[0] as usize].color)
    })
}

#[allow(clippy::expect_used)]
fn style() -> HistogramStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(Histogram::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

fn is_empty_mesh(shape: &egui::Shape) -> bool {
    matches!(shape, egui::Shape::Mesh(m) if m.vertices.is_empty())
}

#[test]
fn peak_scale_is_the_tallest_bin_over_one_point_one() {
    let heights = tone_heights(&[0.0, 10.0, 5.0], HistogramScale::Peak);
    assert!((heights[1] - 1.0 / 1.1).abs() < 1e-6, "{heights:?}");
    assert!((heights[2] - 0.5 / 1.1).abs() < 1e-6);
}

#[test]
fn sparse_histograms_keep_their_proportions() {
    // A percentile of the sorted bins would draw the one-pixel bin as tall as the peak (#6574).
    let mut bins = vec![0.0; 256];
    bins[10] = 1.0;
    bins[200] = 5000.0;
    let heights = tone_heights(&bins, HistogramScale::Peak);
    assert!(heights[10] < 0.001 && heights[200] > 0.9, "{} {}", heights[10], heights[200]);
}

#[test]
fn typical_scale_runs_spikes_off_the_top() {
    // Interior columns of 100 with a spike of 100 000 in the middle: the 95th percentile sets the scale.
    let mut bins = flat(256, 100.0);
    bins[128] = 100_000.0;
    let heights = tone_heights(&bins, HistogramScale::Typical);
    assert!((heights[50] - 100.0 / 150.0).abs() < 1e-5, "{}", heights[50]);
    assert_eq!(heights[128], 1.0);
    let peak = tone_heights(&bins, HistogramScale::Peak);
    assert!(peak[50] < 0.01, "Peak keeps the spike on the plot");
}

#[test]
fn rgb_typical_scale_puts_the_typical_column_at_two_thirds() {
    let mut channel = flat(256, 100.0);
    channel[100] = 50_000.0;
    let heights = rgb_heights(&[channel.clone(), channel.clone(), channel], HistogramScale::Typical);
    assert!((heights[0][30] - 2.0 / 3.0).abs() < 0.01, "{}", heights[0][30]);
    assert_eq!(heights[0][100], 1.0);
}

#[test]
fn rgb_edge_spikes_do_not_flatten_the_interior() {
    let mut channel = flat(256, 100.0);
    channel[0] = 1_000_000.0;
    channel[255] = 1_000_000.0;
    let heights = rgb_heights(&[channel.clone(), channel.clone(), channel], HistogramScale::Peak);
    assert!((heights[0][128] - 1.0 / 1.1).abs() < 1e-5, "{}", heights[0][128]);
    assert_eq!((heights[0][0], heights[0][255]), (1.0, 1.0), "edge spikes are capped at the top");
}

#[test]
fn rgb_scale_is_shared_by_the_three_channels() {
    let (a, b, c) = (flat(256, 100.0), flat(256, 50.0), flat(256, 0.0));
    let heights = rgb_heights(&[a, b, c], HistogramScale::Peak);
    assert!((heights[1][128] - heights[0][128] / 2.0).abs() < 1e-5);
}

#[test]
fn a_flat_black_image_uses_its_edge_peak() {
    let mut black = flat(256, 0.0);
    black[0] = 5000.0;
    let heights = rgb_heights(&[black.clone(), black.clone(), black], HistogramScale::Peak);
    assert!((heights[0][0] - 1.0 / 1.1).abs() < 1e-5);
}

#[test]
fn rgb_smoothing_evens_out_a_comb_and_pins_the_edges() {
    let mut comb = vec![0.0; 256];
    for i in (1..255).step_by(2) {
        comb[i] = 160.0;
    }
    comb[0] = 77.0;
    let heights = rgb_heights(&[comb.clone(), comb.clone(), comb], HistogramScale::Peak);
    let (a, b) = (heights[0][100], heights[0][101]);
    assert!((a - b).abs() < 0.25 * a.max(b), "{a} {b}");
    assert!(heights[0][0] > 0.0, "the edge bin keeps its own value");
}

#[test]
fn overlaps_add_the_channels_saturating_whatever_the_order() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 80, 80), Color32::from_rgb(80, 224, 80), Color32::from_rgb(80, 128, 255)];
    let at = |heights: [f32; 3], colors: &[Color32; 3], height: f32| {
        let h = heights.map(|v| flat(4, v));
        color_at(&rgb_mesh(rect, &h, colors, Blend::Add), pos2(rect.center().x, rect.bottom() - height * rect.height()))
    };
    let red_green = sum(&colors, 0b011);
    assert_eq!(red_green, Color32::from_rgb(255, 255, 160));
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.2), Some(red_green));
    // The same data with the channels' roles swapped.
    assert_eq!(at([0.5, 0.8, 0.0], &[colors[1], colors[0], colors[2]], 0.2), Some(red_green));
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.2), Some(Color32::WHITE));
    // One channel alone above the others; nothing above the tallest.
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.7), Some(colors[0]));
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.7), None);
}

#[test]
fn crossing_channels_keep_the_overlap_colour_under_both() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 255, 0), Color32::BLUE];
    let red = vec![0.9, 0.9, 0.1, 0.1];
    let green = vec![0.1, 0.1, 0.9, 0.9];
    let mesh = rgb_mesh(rect, &[red, green, flat(4, 0.0)], &colors, Blend::Add);
    for x in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let point = pos2(rect.left() + x * rect.width(), rect.bottom() - 0.05 * rect.height());
        assert_eq!(color_at(&mesh, point), Some(Color32::from_rgb(255, 255, 0)), "x = {x}");
    }
}

#[test]
fn bins_of_any_count_span_the_plot_width() {
    let rect = rect();
    for n in [1, 2, 3, 7, 256, 1000] {
        let mesh = tone_mesh(rect, &flat(n, 0.5), Color32::WHITE);
        let xs: Vec<f32> = mesh.vertices.iter().map(|v| v.pos.x).collect();
        let (min, max) = (xs.iter().copied().fold(f32::MAX, f32::min), xs.iter().copied().fold(f32::MIN, f32::max));
        assert_eq!((min, max), (rect.left(), rect.right()), "{n} bins");
    }
    assert!(outline(rect, &[]).is_empty());
}

#[test]
fn empty_and_degenerate_input_draws_nothing() {
    let style = style();
    let data = HistogramData::tone(&[1u32, 2, 3], ChannelColor::Red);
    let rgb = HistogramData::rgb([&[1u32, 2, 3], &[1, 2, 3], &[1, 2, 3]]);
    let bad = [
        Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 100.0)),
        Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, -5.0)),
        Rect::from_min_size(pos2(f32::NAN, 0.0), vec2(100.0, 100.0)),
        Rect::from_min_size(pos2(0.0, 0.0), vec2(f32::INFINITY, 100.0)),
    ];
    for rect in bad {
        for (data, look) in [(&data, HistogramLook::Tone), (&data, HistogramLook::Backdrop), (&rgb, HistogramLook::Rgb)] {
            assert!(plot_shapes(rect, data, look, &style).is_empty(), "{rect:?} {look:?}");
        }
    }
    let empty = HistogramData::tone::<u32>(&[], ChannelColor::Red);
    assert!(plot_shapes(rect(), &empty, HistogramLook::Tone, &style).iter().all(is_empty_mesh));
    // Data that doesn't fit the look draws nothing.
    assert!(plot_shapes(rect(), &rgb, HistogramLook::Tone, &style).is_empty());
    assert!(plot_shapes(rect(), &data, HistogramLook::Rgb, &style).is_empty());
}

#[test]
fn all_zero_bins_draw_no_ribbons() {
    let style = style();
    let zero = [0u32; 16];
    let shapes = plot_shapes(rect(), &HistogramData::rgb([&zero, &zero, &zero]), HistogramLook::Rgb, &style);
    assert!(shapes.iter().all(is_empty_mesh), "no fill and no outline");
}

#[test]
fn backdrop_is_one_grey_in_every_channel() {
    let style = style();
    let data = HistogramData::tone(&[5u32; 8], ChannelColor::Neutral);
    let color = |look| match &plot_shapes(rect(), &data, look, &style)[0] {
        egui::Shape::Mesh(m) => m.vertices[0].color,
        other => panic!("{other:?}"),
    };
    assert_eq!(color(HistogramLook::Tone), style.tone(ChannelColor::Neutral));
    assert_eq!(color(HistogramLook::Backdrop), style.backdrop);
    let red = HistogramData::tone(&[5u32; 8], ChannelColor::Red);
    let backdrop = match &plot_shapes(rect(), &red, HistogramLook::Backdrop, &style)[0] {
        egui::Shape::Mesh(m) => m.vertices[0].color,
        other => panic!("{other:?}"),
    };
    assert_eq!(backdrop, style.backdrop);
}

#[test]
fn tone_colours_come_from_the_data_tokens() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let tokens = Tokens::of(ui);
        let style = Histogram::style(&tokens);
        for channel in ChannelColor::ALL {
            let expected = if channel == ChannelColor::Neutral { tokens.color.text.muted.to_egui() } else { channel.color(&tokens) };
            assert_eq!(style.tone(channel), expected, "{channel:?}");
        }
        assert_eq!(ChannelColor::Cyan.color(&tokens), tokens.color.data.cyan.to_egui());
        assert_eq!(style.scope, tokens.color.data.scope.to_egui());
    })
    .drop_without_applying_deltas();
}

#[test]
fn the_dark_modes_tokens_are_available_in_a_light_theme() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.set_theme(ThemePreference::Light);
    ctx.run_ui(egui::RawInput::default(), |ui| {
        assert_eq!(Tokens::of(ui).mode, Mode::Light);
        let dark = tokens_in_mode(ui.ctx(), Mode::Dark);
        assert_eq!(dark.mode, Mode::Dark);
        assert_ne!(dark.color.data.luma, Tokens::of(ui).color.data.luma);
    })
    .drop_without_applying_deltas();
}

/// The RGB look sits on the dark scope in every theme, so a light theme's histogram outlines a
/// neutral ribbon in the dark mode's `data.luma`.
#[test]
fn a_light_theme_draws_the_dark_modes_inks() {
    let bins: Vec<u32> = (0..64).map(|k| 10 + k % 7).collect();
    let data = |t: &crate::theme::UiTokens| [t.color.data.luma.to_egui()];
    let mut harness = harness(Direction::Ltr).build_ui_state(
        move |ui, colors: &mut Option<([Color32; 1], [Color32; 1])>| {
            *colors = Some((data(&tokens_in_mode(ui.ctx(), Mode::Dark)), data(&Tokens::of(ui))));
            let _ = Histogram::new("h", "Histogram").channels(&[(&bins[..], ChannelColor::Neutral)]).show(ui);
        },
        None,
    );
    harness.ctx.set_theme(ThemePreference::Light);
    harness.run();
    let Some((dark, light)) = *harness.state() else { panic!("no pass") };
    let outlines: Vec<Color32> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Path(p) => match p.stroke.color {
                egui::epaint::ColorMode::Solid(color) => Some(color),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_ne!(dark, light);
    assert_eq!(outlines, dark, "the ribbon's outline");
}

#[test]
fn marker_is_a_tick_on_the_bottom_clipped_to_the_plot() {
    let rect = rect();
    let band = |level: f32| match &histogram::marker_shapes(rect, 256, level, (1.5, 9.0), Color32::WHITE)[0] {
        egui::Shape::Rect(r) => r.rect,
        other => panic!("{other:?}"),
    };
    let middle = band(127.5);
    assert_eq!(middle.width(), 1.5);
    assert!((middle.center().x - rect.center().x).abs() <= 1.0);
    assert_eq!((middle.height(), middle.bottom()), (9.0, rect.bottom()));
    assert!(rect.contains_rect(band(-50.0)) && rect.contains_rect(band(5000.0)));
    assert!(histogram::marker_shapes(rect, 256, f32::NAN, (1.5, 9.0), Color32::WHITE).is_empty());
}

#[test]
fn one_image_node_named_by_the_label_with_the_channel_as_value() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        Histogram::new("h", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Red).value("Red channel").show(ui);
    });
    harness.run();
    let images: Vec<_> = harness.root().children_recursive().filter(|n| n.accesskit_node().role() == Role::Image).collect();
    assert_eq!(images.len(), 1);
    let node = images[0].accesskit_node();
    assert_eq!(node.label().as_deref(), Some("Histogram"));
    assert_eq!(node.value().as_deref(), Some("Red channel"));
    assert!(a11y::tab_order(&mut harness).is_empty(), "not a Tab stop");
}

#[test]
fn the_id_salt_is_the_histogram_s_id() {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, ids: &mut Option<(egui::Id, egui::Id)>| {
            let response = Histogram::new("levels-hist", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Neutral).show(ui);
            *ids = Some((response.id, ui.make_persistent_id(egui::IdSalt::new("levels-hist"))));
        },
        None,
    );
    harness.run();
    let Some((id, expected)) = *harness.state() else { panic!("no pass") };
    assert_eq!(id, expected);
}

#[test]
fn default_height_is_the_token_and_height_overrides_it() {
    let height = |h: Option<f32>| {
        let mut harness = harness(Direction::Ltr).build_ui(move |ui| {
            let histogram = Histogram::new("h", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Neutral);
            match h {
                Some(h) => histogram.height(h).show(ui),
                None => histogram.show(ui),
            };
        });
        harness.run();
        harness.get_by_label("Histogram").rect().height()
    };
    assert_eq!(height(None), style().height);
    assert_eq!(height(Some(110.0)), 110.0);
}

#[test]
fn stale_adds_a_named_mark_at_the_trailing_top_corner() {
    let mark = |dir: Direction, stale: bool| {
        let mut harness = harness(dir).build_ui(move |ui| {
            Histogram::new("h", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Neutral).stale(stale).stale_label("Cached data, not current").show(ui);
        });
        harness.run();
        let plot = harness.get_by_label("Histogram").rect();
        (plot, harness.query_by_label("Cached data, not current").map(|n| n.rect()))
    };
    assert_eq!(mark(Direction::Ltr, false).1, None);
    let (plot, rect) = mark(Direction::Ltr, true);
    let rect = rect.expect("stale mark");
    assert!(rect.right() > plot.center().x && rect.top() < plot.top() + 12.0, "{rect:?} in {plot:?}");
    let (plot, rect) = mark(Direction::Rtl, true);
    assert!(rect.expect("stale mark").left() < plot.center().x);
}

#[test]
fn stories_draw_and_are_labelled_at_three_widths() {
    for name in ["tone", "rgb"] {
        for width in [80.0, 256.0, 400.0] {
            let mut harness = harness(Direction::Ltr).build_ui(move |ui| {
                if name == "tone" {
                    histogram::show_tone(ui, &mut ToneArgs { channel: ChannelArg::Red, stale: true, marker: true, width });
                } else {
                    histogram::show_rgb(ui, &mut RgbArgs { scale: ScaleArg::Typical, marker: true, width });
                }
            });
            harness.run();
            a11y::assert_labelled(&harness);
            assert!(harness.root().children_recursive().any(|n| n.accesskit_node().role() == Role::Image), "{name} at {width}");
        }
    }
}

#[allow(clippy::expect_used)]
fn first_mesh(shapes: &[egui::Shape]) -> &Mesh {
    shapes.iter().find_map(|s| if let egui::Shape::Mesh(m) = s { Some(&**m) } else { None }).expect("a mesh")
}

#[test]
fn channels_with_four_inks_draw_four_fills() {
    let ink = |quarter: usize| -> Vec<u32> { (0..64).map(|i| if i / 16 == quarter { 100 } else { 0 }).collect() };
    let bins: Vec<Vec<u32>> = (0..4).map(ink).collect();
    let colors = [ChannelColor::Cyan, ChannelColor::Magenta, ChannelColor::Yellow, ChannelColor::Neutral];
    let inks: Vec<(&[u32], ChannelColor)> = bins.iter().map(Vec::as_slice).zip(colors).collect();
    let (rect, style) = (rect(), style());
    let shapes = plot_shapes(rect, &HistogramData::channels(&inks), HistogramLook::Rgb, &style);
    let mesh = first_mesh(&shapes);
    for (quarter, color) in colors.into_iter().enumerate() {
        let x = rect.left() + (quarter as f32 * 16.0 + 8.0) / 64.0 * rect.width();
        assert_eq!(color_at(mesh, pos2(x, rect.bottom() - 2.0)), Some(style.ink(color)), "ink {quarter}");
    }
    let outlines = shapes.iter().filter(|s| matches!(s, egui::Shape::Path(_))).count();
    assert_eq!(outlines, 4, "an outline per ink");
}

#[test]
fn overlapping_inks_multiply_and_rgb_adds() {
    let colors = [Color32::from_rgb(200, 100, 0), Color32::from_rgb(100, 200, 255)];
    assert_eq!(product(&colors, 0b11), Color32::from_rgb(78, 78, 0));
    assert_eq!(product(&colors, 0b10), colors[1]);
    assert_eq!(product(&colors, 0), Color32::WHITE);
    assert_eq!(sum(&colors, 0b11), Color32::from_rgb(255, 255, 255));
    assert_eq!(sum(&colors, 0), Color32::BLACK);
    let heights = vec![flat(8, 0.8), flat(8, 0.4)];
    let rect = rect();
    let at = |blend| color_at(&rgb_mesh(rect, &heights, &colors, blend), pos2(rect.center().x, rect.bottom() - 5.0));
    assert_eq!(at(Blend::Multiply), Some(Color32::from_rgb(78, 78, 0)));
    assert_eq!(at(Blend::Add), Some(Color32::from_rgb(255, 255, 255)));
}

#[test]
#[allow(clippy::expect_used)]
fn the_outline_is_drawn_over_the_fill() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        Histogram::new("h", "Histogram").tone(&[1u32, 5, 2, 8], ChannelColor::Red).outline(&[9u32, 3, 3, 3], ChannelColor::Neutral).show(ui);
    });
    harness.run();
    let shapes = &harness.output().shapes;
    let fill = shapes.iter().position(|c| matches!(&c.shape, egui::Shape::Mesh(_))).expect("fill");
    let line = shapes.iter().position(|c| matches!(&c.shape, egui::Shape::Path(_))).expect("outline");
    assert!(line > fill);
    let egui::Shape::Path(path) = &shapes[line].shape else { panic!("not a path") };
    assert_eq!(path.stroke.width, style().line);
    assert_eq!(path.stroke.color, egui::epaint::ColorMode::Solid(style().outline_neutral));
}

#[test]
fn the_outline_shares_the_fills_scale_in_the_tone_look() {
    let (rect, style) = (rect(), style());
    let data = HistogramData::tone(&[10u32, 10, 10], ChannelColor::Red);
    let egui::Shape::Path(path) = &histogram::outline_shapes(rect, &data, &[5.0, 5.0, 5.0], ChannelColor::Neutral, &style)[0] else { panic!() };
    let fill = tone_heights(&[10.0, 10.0, 10.0], HistogramScale::Peak)[0];
    assert!((path.points[1].y - (rect.bottom() - fill / 2.0 * rect.height())).abs() < 1e-4, "half the fill's height");
    let egui::Shape::Path(path) = &histogram::outline_shapes(rect, &data, &[500.0; 3], ChannelColor::Neutral, &style)[0] else { panic!() };
    assert_eq!(path.points[1].y, rect.top(), "a taller outline runs off the top");
}

#[test]
fn a_neutral_outline_differs_from_the_neutral_fill() {
    let (rect, style) = (rect(), style());
    let data = HistogramData::tone(&[10u32, 10, 10], ChannelColor::Neutral);
    let egui::Shape::Path(path) = &histogram::outline_shapes(rect, &data, &[5.0; 3], ChannelColor::Neutral, &style)[0] else { panic!() };
    assert_ne!(path.stroke.color, egui::epaint::ColorMode::Solid(style.tone(ChannelColor::Neutral)));
}

/// A harness whose histogram has a selection and keeps the range the user reports.
fn selection_harness(start: Option<RangeInclusive<f32>>) -> Harness<'static, Option<RangeInclusive<f32>>> {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, range: &mut Option<RangeInclusive<f32>>| {
            let shown = Histogram::new("h", "Histogram").tone(&vec![5u32; 100], ChannelColor::Neutral).selection(range.clone()).show(ui);
            if let Some(new) = shown.selection {
                *range = new;
            }
        },
        start,
    );
    harness.run();
    harness
}

#[test]
fn a_dragged_selection_reports_its_range() {
    let mut harness = selection_harness(None);
    let plot = harness.get_by_label("Histogram").rect();
    let at = |bin: f32| pos2(plot.left() + (bin + 0.5) / 100.0 * plot.width(), plot.center().y);
    harness.hover_at(at(20.0));
    harness.event(Event::PointerButton { pos: at(20.0), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    for bin in [40.0, 60.0] {
        harness.event(Event::PointerMoved(at(bin)));
        harness.run();
    }
    assert_eq!(harness.state().clone(), Some(20.0..=60.0));
    harness.event(Event::PointerMoved(at(5.0)));
    harness.run();
    assert_eq!(harness.state().clone(), Some(5.0..=20.0), "dragging back past the start swaps the edges");
    harness.event(Event::PointerMoved(pos2(plot.right() + 500.0, plot.center().y)));
    harness.run();
    assert_eq!(harness.state().clone(), Some(20.0..=99.0), "clamped to the last bin");
}

#[test]
fn the_arrow_keys_move_the_selection_edges() {
    let mut harness = selection_harness(Some(10.0..=20.0));
    harness.get_by_label("Histogram").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().clone(), Some(10.0..=21.0));
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().clone(), Some(9.0..=21.0));
}

#[test]
fn a_histogram_without_selection_senses_nothing() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        Histogram::new("h", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Neutral).show(ui);
    });
    harness.run();
    assert!(a11y::tab_order(&mut harness).is_empty());
}

#[test]
fn markers_draw_a_caption_each_and_name_it() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        let markers = [(10.0, "Black".to_owned()), (250.0, "White".to_owned())];
        Histogram::new("h", "Histogram").tone(&bell_bins(), ChannelColor::Neutral).markers(&markers).show(ui);
    });
    harness.run();
    let plot = harness.get_by_label("Histogram").rect();
    let (black, white) = (harness.get_by_label("Black").rect(), harness.get_by_label("White").rect());
    assert!(plot.contains_rect(black) && plot.contains_rect(white), "{black:?} {white:?} in {plot:?}");
    assert!(black.center().x < white.center().x);
    assert!(white.right() <= plot.right(), "a caption near the end turns to the other side of its band");
}

#[test]
fn a_marker_caption_sits_beside_the_top_of_its_tick() {
    let mut harness = harness(Direction::Ltr).build_ui(|ui| {
        Histogram::new("h", "Histogram").tone(&bell_bins(), ChannelColor::Neutral).markers(&[(10.0, "Black".to_owned())]).show(ui);
    });
    harness.run();
    let plot = harness.get_by_label("Histogram").rect();
    let caption = harness.get_by_label("Black").rect();
    let height = style().marker_height;
    assert!((caption.bottom() - (plot.bottom() - height)).abs() < 1.0, "{caption:?} in {plot:?}");
    assert!(caption.left() > plot.left() + 10.0 / 256.0 * plot.width(), "right of the tick");
}

fn bell_bins() -> Vec<u32> {
    (0..256).map(|i| 100 + i % 7).collect()
}

#[test]
fn the_stale_mark_is_a_button_only_with_on_stale_click() {
    let clicks = |click: bool| {
        let mut harness = harness(Direction::Ltr).build_ui_state(
            move |ui, clicked: &mut u32| {
                let histogram = Histogram::new("h", "Histogram").tone(&[1u32, 5, 2], ChannelColor::Neutral).stale(true).stale_label("Cached data");
                let shown = if click { histogram.on_stale_click("Recalculate").show(ui) } else { histogram.show(ui) };
                *clicked += u32::from(shown.clicked);
            },
            0,
        );
        harness.run();
        let name = if click { "Recalculate" } else { "Cached data" };
        let role = harness.get_by_label(name).accesskit_node().role();
        harness.get_by_label(name).click();
        harness.run();
        (role, *harness.state())
    };
    assert_eq!(clicks(false), (Role::Image, 0));
    assert_eq!(clicks(true), (Role::Button, 1));
}

#[test]
fn ink_fills_are_opaque_and_overlaps_are_the_product_of_the_inks() {
    let (rect, style) = (rect(), style());
    let (tall, short) = (vec![100u32; 16], vec![40u32; 16]);
    let inks: [(&[u32], ChannelColor); 2] = [(&tall, ChannelColor::Cyan), (&short, ChannelColor::Magenta)];
    let shapes = plot_shapes(rect, &HistogramData::channels(&inks), HistogramLook::Rgb, &style);
    let mesh = first_mesh(&shapes);
    let at = |height: f32| color_at(mesh, pos2(rect.center().x, rect.bottom() - height * rect.height()));
    let colors = [style.ink(ChannelColor::Cyan), style.ink(ChannelColor::Magenta)];
    assert_eq!(at(0.1), Some(product(&colors, 0b11)), "under both inks");
    assert_eq!(at(0.6), Some(colors[0]), "under the tall ink only");
    assert_eq!(colors[0].a(), 255);
    // The RGB look is opaque too: the tall channel alone is its own colour, the overlap their saturating sum.
    let rgb = plot_shapes(rect, &HistogramData::rgb([&tall, &short, &short]), HistogramLook::Rgb, &style);
    let rgb_at = |height: f32| color_at(first_mesh(&rgb), pos2(rect.center().x, rect.bottom() - height * rect.height()));
    assert_eq!(rgb_at(0.6), Some(style.ink(ChannelColor::Red)));
    assert_eq!(rgb_at(0.1), Some(sum(&[style.ink(ChannelColor::Red), style.ink(ChannelColor::Green), style.ink(ChannelColor::Blue)], 0b111)));
}

#[test]
fn the_selection_band_covers_whole_bin_slots() {
    let (rect, style) = (rect(), style());
    let slot = rect.width() / 64.0;
    let band = |range: RangeInclusive<f32>| match &histogram::selection_shapes(rect, 64, &range, style.marker_width, Color32::WHITE)[0] {
        egui::Shape::Rect(r) => r.rect,
        other => panic!("{other:?}"),
    };
    let one = band(10.0..=10.0);
    assert!((one.width() - slot).abs() < 1e-4 && (one.left() - (rect.left() + 10.0 * slot)).abs() < 1e-4, "{one:?}");
    let many = band(10.0..=20.0);
    assert!((many.left() - one.left()).abs() < 1e-4 && (many.width() - 11.0 * slot).abs() < 1e-4, "{many:?}");
    let all = band(0.0..=63.0);
    assert_eq!((all.left(), all.right()), (rect.left(), rect.right()));
    let edges: Vec<_> = histogram::selection_shapes(rect, 64, &(10.0..=20.0), style.marker_width, Color32::WHITE)[1..].to_vec();
    assert_eq!(edges.len(), 2, "an edge band at each end");
}

/// A selection histogram whose host stores the range, counts clears and keeps the level under the pointer.
#[derive(Default)]
struct Probe {
    range: Option<RangeInclusive<f32>>,
    clears: u32,
    hovered: Option<f32>,
    id: Option<egui::Id>,
}

fn probe(range: Option<RangeInclusive<f32>>) -> Harness<'static, Probe> {
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, state: &mut Probe| {
            let shown = Histogram::new("h", "Histogram").tone(&vec![5u32; 100], ChannelColor::Neutral).selection(state.range.clone()).show(ui);
            state.id = Some(shown.id);
            if let Some(new) = shown.selection {
                state.clears += u32::from(new.is_none());
                state.range = new;
            }
            state.hovered = shown.hovered_level;
        },
        Probe { range, ..Probe::default() },
    );
    harness.run();
    harness
}

#[test]
fn a_click_or_esc_clears_the_selection_and_is_reported() {
    let mut harness = probe(Some(10.0..=20.0));
    harness.get_by_label("Histogram").click();
    harness.run();
    assert_eq!((harness.state().range.clone(), harness.state().clears), (None, 1), "a click clears");
    harness.get_by_label("Histogram").click();
    harness.run();
    assert_eq!(harness.state().clears, 1, "a click without a selection reports nothing");

    let mut harness = probe(Some(10.0..=20.0));
    harness.get_by_label("Histogram").focus();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!((harness.state().range.clone(), harness.state().clears), (None, 1), "Esc clears");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().clears, 1);
}

#[test]
fn a_drag_does_not_clear_and_enter_does_not_either() {
    let mut harness = probe(Some(10.0..=20.0));
    let plot = harness.get_by_label("Histogram").rect();
    let at = |bin: f32| pos2(plot.left() + (bin + 0.5) / 100.0 * plot.width(), plot.center().y);
    harness.hover_at(at(50.0));
    harness.event(Event::PointerButton { pos: at(50.0), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    harness.event(Event::PointerMoved(at(60.0)));
    harness.run();
    harness.event(Event::PointerButton { pos: at(60.0), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    assert_eq!((harness.state().range.clone(), harness.state().clears), (Some(50.0..=60.0), 0));
    harness.get_by_label("Histogram").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().clears, 0, "Enter activates nothing");
}

#[test]
fn the_response_reports_the_level_under_the_pointer() {
    let mut harness = probe(None);
    let plot = harness.get_by_label("Histogram").rect();
    harness.hover_at(pos2(plot.left() + 37.5 / 100.0 * plot.width(), plot.center().y));
    harness.run();
    assert_eq!(harness.state().hovered, Some(37.0));
    harness.hover_at(pos2(plot.right() - 0.1, plot.center().y));
    harness.run();
    assert_eq!(harness.state().hovered, Some(99.0));
    harness.hover_at(pos2(plot.right() + 50.0, plot.center().y));
    harness.run();
    assert_eq!(harness.state().hovered, None);
}

#[test]
fn a_selection_is_two_sliders_beneath_the_plot_with_values_and_steps() {
    let mut harness = probe(Some(10.0..=20.0));
    let parent = harness.get_by_label("Histogram").accesskit_node().id();
    for (name, value, min, max) in [("Selection start", 10.0, 0.0, 20.0), ("Selection end", 20.0, 10.0, 99.0)] {
        let node = harness.get_by_label(name).accesskit_node();
        assert_eq!(node.role(), Role::Slider, "{name}");
        assert!(std::iter::successors(node.parent(), |n| n.parent()).any(|n| n.id() == parent), "{name} is beneath the plot");
        assert_eq!((node.numeric_value(), node.min_numeric_value(), node.max_numeric_value()), (Some(value), Some(min), Some(max)), "{name}");
        assert!(node.data().supports_action(Action::Increment) && node.data().supports_action(Action::Decrement), "{name}");
    }
    let request = |harness: &mut Harness<'_, Probe>, name: &str, action, data| {
        let edge = usize::from(name == "Selection end");
        let target_node = histogram::selection_node(harness.state().id.expect("shown"), edge).accesskit_id();
        harness.event(Event::AccessKitActionRequest(ActionRequest { action, target_tree: TreeId::ROOT, target_node, data }));
        harness.run();
    };
    request(&mut harness, "Selection end", Action::Increment, None);
    assert_eq!(harness.state().range, Some(10.0..=21.0));
    request(&mut harness, "Selection start", Action::Decrement, None);
    assert_eq!(harness.state().range, Some(9.0..=21.0));
    request(&mut harness, "Selection start", Action::SetValue, Some(ActionData::NumericValue(30.0)));
    assert_eq!(harness.state().range, Some(21.0..=21.0), "a start is held at the end");
    request(&mut harness, "Selection end", Action::Decrement, None);
    assert_eq!(harness.state().range, Some(21.0..=21.0), "an end is held at the start");
    assert_eq!(harness.get_by_label("Selection start").accesskit_node().numeric_value(), Some(21.0));
}

#[test]
fn no_selection_has_no_slider_nodes_and_the_names_are_the_hosts() {
    let harness = probe(None);
    assert!(harness.query_by_label("Selection start").is_none());
    let mut named = harness_named();
    named.run();
    assert_eq!(named.get_by_label("Anfang").accesskit_node().role(), Role::Slider);
    assert_eq!(named.get_by_label("Ende").accesskit_node().role(), Role::Slider);
}

fn harness_named() -> Harness<'static> {
    harness(Direction::Ltr).build_ui(|ui| {
        Histogram::new("h", "Histogram")
            .tone(&[1u32; 8], ChannelColor::Neutral)
            .selection(Some(2.0..=3.0))
            .selection_labels("Anfang", "Ende")
            .show(ui);
    })
}

/// PhotoCraft's Histogram panel has no outline; its levels dialog draws one, and a selected range is
/// the accent at 0.35.
#[test]
fn the_plot_has_an_edge_only_when_bordered_and_a_selection_is_the_accent() {
    let edges = |bordered: bool| {
        let mut harness = harness(Direction::Ltr).build_ui(move |ui| {
            let _ = Histogram::new("h", "Histogram").tone(&[1u32, 4, 2], ChannelColor::Neutral).bordered(bordered).show(ui);
        });
        harness.run();
        harness.output().shapes.iter().filter(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.stroke.width > 0.0)).count()
    };
    assert_eq!(edges(false), 0);
    assert_eq!(edges(true), 1);
    let style = style();
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let tokens = Tokens::of(ui);
        assert_eq!(style.selection, tokens.color.accent.solid.to_egui().gamma_multiply(tokens.component.histogram.selection_alpha));
    })
    .drop_without_applying_deltas();
}
