//! Histogram tests: scales, overlaps, degenerate input, the AccessKit node and the stale mark.

use craft_theme::Mode;
use egui::accesskit::Role;
use egui::{Color32, Context, Mesh, Pos2, Rect, ThemePreference, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::histogram::{self, outline, overlap, plot_shapes, rgb_heights, rgb_mesh, tone_heights, tone_mesh};
use super::*;
use crate::direction::{self, Direction};
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install, tokens_in_mode};

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
fn overlaps_take_the_channel_wise_maximum_whatever_the_order() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 80, 80), Color32::from_rgb(80, 224, 80), Color32::from_rgb(80, 128, 255)];
    let at = |heights: [f32; 3], colors: &[Color32; 3], height: f32| {
        let h = heights.map(|v| flat(4, v));
        color_at(&rgb_mesh(rect, &h, colors, 0.3), pos2(rect.center().x, rect.bottom() - height * rect.height()))
    };
    let red_green = overlap(&colors, 0b011);
    assert_eq!(red_green, Color32::from_rgb(255, 224, 80));
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.2), Some(red_green.gamma_multiply(0.3)));
    // The same data with the channels' roles swapped.
    assert_eq!(at([0.5, 0.8, 0.0], &[colors[1], colors[0], colors[2]], 0.2), Some(red_green.gamma_multiply(0.3)));
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.2), Some(overlap(&colors, 0b111).gamma_multiply(0.3)));
    // One channel alone above the others; nothing above the tallest.
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.7), Some(colors[0].gamma_multiply(0.3)));
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.7), None);
}

#[test]
fn crossing_channels_keep_the_overlap_colour_under_both() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 255, 0), Color32::BLUE];
    let red = vec![0.9, 0.9, 0.1, 0.1];
    let green = vec![0.1, 0.1, 0.9, 0.9];
    let mesh = rgb_mesh(rect, &[red, green, flat(4, 0.0)], &colors, 1.0);
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
fn backdrop_is_the_tone_look_at_the_backdrop_alpha() {
    let style = style();
    let data = HistogramData::tone(&[5u32; 8], ChannelColor::Neutral);
    let color = |look| match &plot_shapes(rect(), &data, look, &style)[0] {
        egui::Shape::Mesh(m) => m.vertices[0].color,
        other => panic!("{other:?}"),
    };
    assert_eq!(color(HistogramLook::Tone), style.tone(ChannelColor::Neutral));
    assert_eq!(color(HistogramLook::Backdrop), style.tone(ChannelColor::Neutral).gamma_multiply(style.backdrop_alpha));
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
        assert_ne!(dark.color.data.red, Tokens::of(ui).color.data.red);
    })
    .drop_without_applying_deltas();
}

/// The RGB look sits on the dark scope in every theme, so a light theme's histogram outlines its
/// ribbons in the dark mode's data colours.
#[test]
fn a_light_theme_draws_the_dark_modes_ribbons() {
    let bins: Vec<u32> = (0..64).map(|k| 10 + k % 7).collect();
    let data = |t: &crate::theme::UiTokens| [t.color.data.red, t.color.data.green, t.color.data.blue].map(|c| c.to_egui());
    let mut harness = harness(Direction::Ltr).build_ui_state(
        move |ui, colors: &mut Option<([Color32; 3], [Color32; 3])>| {
            *colors = Some((data(&tokens_in_mode(ui.ctx(), Mode::Dark)), data(&Tokens::of(ui))));
            let _ = Histogram::new("h", "Histogram").rgb([&bins, &bins, &bins]).show(ui);
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
    assert_eq!(outlines, dark, "the ribbons' outlines, red, green and blue");
}

#[test]
fn marker_is_a_two_point_band_clipped_to_the_plot() {
    let rect = rect();
    let band = |level: f32| match &histogram::marker_shapes(rect, 256, level, 2.0, Color32::WHITE)[0] {
        egui::Shape::Rect(r) => r.rect,
        other => panic!("{other:?}"),
    };
    let middle = band(127.5);
    assert_eq!(middle.width(), 2.0);
    assert!((middle.center().x - rect.center().x).abs() <= 1.0);
    assert_eq!((middle.top(), middle.bottom()), (rect.top(), rect.bottom()));
    assert!(rect.contains_rect(band(-50.0)) && rect.contains_rect(band(5000.0)));
    assert!(histogram::marker_shapes(rect, 256, f32::NAN, 2.0, Color32::WHITE).is_empty());
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
    for (show, name) in [(histogram::show_tone as fn(&mut Ui, &mut Args), "tone"), (histogram::show_rgb, "rgb")] {
        for width in [80.0, 256.0, 400.0] {
            let mut harness = harness(Direction::Ltr).build_ui(move |ui| {
                let mut args = Args::new()
                    .with_choice("channel", &["neutral", "red"], 1)
                    .with_choice("scale", &["peak", "typical"], 1)
                    .with_bool("stale", true)
                    .with_bool("marker", true)
                    .with_number("width", width, 80.0, 400.0);
                show(ui, &mut args);
            });
            harness.run();
            a11y::assert_labelled(&harness);
            assert!(harness.root().children_recursive().any(|n| n.accesskit_node().role() == Role::Image), "{name} at {width}");
        }
    }
}
