//! The histogram: how tall the bins are drawn, how overlapping channels combine, the selection band and what the
//! plot reports for the pointer, the keys and the stale mark.

use craft_imaging::ui::histogram::paint::{
    Blend, outline, outline_shapes, plot_shapes, product, rgb_heights, rgb_mesh, selection_shapes, sum, tone_heights, tone_mesh,
};
use craft_imaging::ui::histogram::{
    ChannelColor, Histogram, HistogramData, HistogramInks, HistogramLook, HistogramScale, LevelRange, StaleMark, level_at,
};
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Fixture, Frames, Story, StoryData, StoryOutput, pointer, trial};
use egui::{Color32, Key, Mesh, Modifiers, Pos2, Rect, Shape, Ui, Vec2, pos2, vec2};

const BINS: usize = 100;
const TOLERANCE: f32 = 1e-5;

fn rect() -> Rect {
    Rect::from_min_size(pos2(10.0, 20.0), vec2(256.0, 100.0))
}

fn flat(n: usize, v: f32) -> Vec<f32> {
    vec![v; n]
}

fn inks() -> HistogramInks {
    let frames = Frames::new();
    with_cx(&frames.ctx, &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, HistogramInks::new)
}

/// The colour of the mesh's triangle containing `point`.
fn color_at(mesh: &Mesh, point: Pos2) -> Option<Color32> {
    mesh.indices.as_chunks::<3>().0.iter().find_map(|t| {
        let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[t[i] as usize].pos);
        let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
        if cross(b - a, c - a).abs() < 1e-5 {
            return None;
        }
        let sides = [cross(b - a, point - a), cross(c - b, point - b), cross(a - c, point - c)];
        (sides.iter().all(|&s| s >= -1e-5) || sides.iter().all(|&s| s <= 1e-5)).then_some(mesh.vertices[t[0] as usize].color)
    })
}

fn is_empty_mesh(shape: &Shape) -> bool {
    matches!(shape, Shape::Mesh(m) if m.vertices.is_empty())
}

fn first_mesh(shapes: &[Shape]) -> &Mesh {
    shapes.iter().find_map(|s| if let Shape::Mesh(m) = s { Some(&**m) } else { None }).expect("a mesh")
}

#[trial]
fn the_tallest_bin_is_drawn_at_one_over_one_point_one_and_a_sparse_bin_keeps_its_proportion() {
    let heights = tone_heights(&[0.0, 10.0, 5.0], HistogramScale::Peak);
    assert!((heights[1] - 1.0 / 1.1).abs() < TOLERANCE && (heights[2] - 0.5 / 1.1).abs() < TOLERANCE, "{heights:?}");
    let mut bins = vec![0.0; 256];
    bins[10] = 1.0;
    bins[200] = 5000.0;
    let heights = tone_heights(&bins, HistogramScale::Peak);
    assert!(heights[10] < 0.001 && heights[200] > 0.9, "a percentile would draw the one-pixel bin as tall as the peak");
}

#[trial]
fn the_typical_scale_runs_spikes_off_the_top() {
    let mut bins = flat(256, 100.0);
    bins[128] = 100_000.0;
    let heights = tone_heights(&bins, HistogramScale::Typical);
    assert!((heights[50] - 100.0 / 150.0).abs() < TOLERANCE, "{}", heights[50]);
    assert_eq!(heights[128], 1.0);
    assert!(tone_heights(&bins, HistogramScale::Peak)[50] < 0.01, "Peak keeps the spike on the plot");
    let mut channel = flat(256, 100.0);
    channel[100] = 50_000.0;
    let rgb = rgb_heights(&[channel.clone(), channel.clone(), channel], HistogramScale::Typical);
    assert!((rgb[0][30] - 2.0 / 3.0).abs() < 0.01 && rgb[0][100] == 1.0, "{}", rgb[0][30]);
}

#[trial]
fn ribbons_share_one_scale_that_edge_spikes_do_not_set() {
    let mut channel = flat(256, 100.0);
    channel[0] = 1_000_000.0;
    channel[255] = 1_000_000.0;
    let heights = rgb_heights(&[channel.clone(), channel.clone(), channel], HistogramScale::Peak);
    assert!((heights[0][128] - 1.0 / 1.1).abs() < TOLERANCE, "{}", heights[0][128]);
    assert_eq!((heights[0][0], heights[0][255]), (1.0, 1.0), "edge spikes are capped at the top");
    let shared = rgb_heights(&[flat(256, 100.0), flat(256, 50.0), flat(256, 0.0)], HistogramScale::Peak);
    assert!((shared[1][128] - shared[0][128] / 2.0).abs() < TOLERANCE);
    let mut black = flat(256, 0.0);
    black[0] = 5000.0;
    let heights = rgb_heights(&[black.clone(), black.clone(), black], HistogramScale::Peak);
    assert!((heights[0][0] - 1.0 / 1.1).abs() < TOLERANCE, "a flat black image uses its edge peak");
}

#[trial]
fn smoothing_evens_out_a_comb_and_pins_the_edges() {
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

#[trial]
fn overlaps_combine_the_channels_whatever_their_order() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 80, 80), Color32::from_rgb(80, 224, 80), Color32::from_rgb(80, 128, 255)];
    let at = |heights: [f32; 3], colors: &[Color32; 3], height: f32| {
        let h = heights.map(|v| flat(4, v));
        color_at(&rgb_mesh(rect, &h, colors, Blend::Add), pos2(rect.center().x, rect.bottom() - height * rect.height()))
    };
    let red_green = sum(&colors, 0b011);
    assert_eq!(red_green, Color32::from_rgb(255, 255, 160));
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.2), Some(red_green));
    assert_eq!(at([0.5, 0.8, 0.0], &[colors[1], colors[0], colors[2]], 0.2), Some(red_green), "the same data with the roles swapped");
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.2), Some(Color32::WHITE));
    assert_eq!(at([0.8, 0.5, 0.0], &colors, 0.7), Some(colors[0]));
    assert_eq!(at([0.5, 0.5, 0.5], &colors, 0.7), None, "nothing above the tallest");

    let inks = [Color32::from_rgb(200, 100, 0), Color32::from_rgb(100, 200, 255)];
    assert_eq!(product(&inks, 0b11), Color32::from_rgb(78, 78, 0));
    assert_eq!(product(&inks, 0), Color32::WHITE);
    let heights = vec![flat(8, 0.8), flat(8, 0.4)];
    let at = |blend| color_at(&rgb_mesh(rect, &heights, &inks, blend), pos2(rect.center().x, rect.bottom() - 5.0));
    assert_eq!(at(Blend::Multiply), Some(Color32::from_rgb(78, 78, 0)));
    assert_eq!(at(Blend::Add), Some(Color32::WHITE));
}

#[trial]
fn crossing_channels_keep_the_overlap_colour_under_both() {
    let rect = rect();
    let colors = [Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 255, 0), Color32::BLUE];
    let mesh = rgb_mesh(rect, &[vec![0.9, 0.9, 0.1, 0.1], vec![0.1, 0.1, 0.9, 0.9], flat(4, 0.0)], &colors, Blend::Add);
    for x in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let point = pos2(rect.left() + x * rect.width(), rect.bottom() - 0.05 * rect.height());
        assert_eq!(color_at(&mesh, point), Some(Color32::from_rgb(255, 255, 0)), "x = {x}");
    }
}

#[trial]
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

#[trial]
fn empty_and_degenerate_input_draws_nothing() {
    let inks = inks();
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
            assert!(plot_shapes(rect, data, look, &inks).is_empty(), "{rect:?} {look:?}");
        }
    }
    let empty = HistogramData::tone::<u32>(&[], ChannelColor::Red);
    assert!(plot_shapes(rect(), &empty, HistogramLook::Tone, &inks).iter().all(is_empty_mesh));
    assert!(plot_shapes(rect(), &rgb, HistogramLook::Tone, &inks).is_empty(), "data that doesn't fit the look draws nothing");
    assert!(plot_shapes(rect(), &data, HistogramLook::Rgb, &inks).is_empty());
    let zero = [0u32; 16];
    let shapes = plot_shapes(rect(), &HistogramData::rgb([&zero, &zero, &zero]), HistogramLook::Rgb, &inks);
    assert!(shapes.iter().all(is_empty_mesh), "all-zero bins draw no ribbons and no outline");
}

#[trial]
fn ink_fills_are_opaque_and_overlaps_are_the_product_of_the_inks() {
    let (rect, inks) = (rect(), inks());
    let (tall, short) = (vec![100u32; 16], vec![40u32; 16]);
    let channels: [(&[u32], ChannelColor); 2] = [(&tall, ChannelColor::Cyan), (&short, ChannelColor::Magenta)];
    let shapes = plot_shapes(rect, &HistogramData::channels(&channels), HistogramLook::Rgb, &inks);
    let mesh = first_mesh(&shapes);
    let at = |height: f32| color_at(mesh, pos2(rect.center().x, rect.bottom() - height * rect.height()));
    let colors = [inks.ink(ChannelColor::Cyan), inks.ink(ChannelColor::Magenta)];
    assert_eq!(at(0.1), Some(product(&colors, 0b11)), "under both inks");
    assert_eq!(at(0.6), Some(colors[0]), "under the tall ink only");
    assert_eq!(colors[0].a(), 255);
    let rgb = plot_shapes(rect, &HistogramData::rgb([&tall, &short, &short]), HistogramLook::Rgb, &inks);
    let rgb_at = |height: f32| color_at(first_mesh(&rgb), pos2(rect.center().x, rect.bottom() - height * rect.height()));
    assert_eq!(rgb_at(0.6), Some(inks.ink(ChannelColor::Red)));
    assert_eq!(rgb_at(0.1), Some(sum(&inks.ribbons(), 0b111)));
    let four: Vec<Vec<u32>> = (0..4).map(|quarter| (0..64).map(|i| if i / 16 == quarter { 100 } else { 0 }).collect()).collect();
    let colors = [ChannelColor::Cyan, ChannelColor::Magenta, ChannelColor::Yellow, ChannelColor::Neutral];
    let channels: Vec<(&[u32], ChannelColor)> = four.iter().map(Vec::as_slice).zip(colors).collect();
    let shapes = plot_shapes(rect, &HistogramData::channels(&channels), HistogramLook::Rgb, &inks);
    for (quarter, color) in colors.into_iter().enumerate() {
        let x = rect.left() + (quarter as f32 * 16.0 + 8.0) / 64.0 * rect.width();
        assert_eq!(color_at(first_mesh(&shapes), pos2(x, rect.bottom() - 2.0)), Some(inks.ink(color)), "ink {quarter}");
    }
    assert_eq!(shapes.iter().filter(|s| matches!(s, Shape::Path(_))).count(), 4, "an outline per ink");
}

#[trial]
fn the_outline_shares_the_fills_scale_in_the_tone_look() {
    let (rect, inks) = (rect(), inks());
    let data = HistogramData::tone(&[10u32, 10, 10], ChannelColor::Red);
    let Shape::Path(path) = &outline_shapes(rect, &data, &[5.0, 5.0, 5.0], ChannelColor::Neutral, &inks)[0] else { panic!("not a path") };
    let fill = tone_heights(&[10.0, 10.0, 10.0], HistogramScale::Peak)[0];
    assert!((path.points[1].y - (rect.bottom() - fill / 2.0 * rect.height())).abs() < 1e-4, "half the fill's height");
    let Shape::Path(path) = &outline_shapes(rect, &data, &[500.0; 3], ChannelColor::Neutral, &inks)[0] else { panic!("not a path") };
    assert_eq!(path.points[1].y, rect.top(), "a taller outline runs off the top");
}

#[trial]
fn the_selection_band_covers_whole_bin_slots() {
    let rect = rect();
    let slot = rect.width() / 64.0;
    let band = |range: LevelRange| match &selection_shapes(rect, 64, range, 1.5, Color32::WHITE)[0] {
        Shape::Rect(r) => r.rect,
        other => panic!("{other:?}"),
    };
    let one = band(LevelRange::new(10.0, 10.0));
    assert!((one.width() - slot).abs() < 1e-4 && (one.left() - (rect.left() + 10.0 * slot)).abs() < 1e-4, "{one:?}");
    let many = band(LevelRange::new(10.0, 20.0));
    assert!((many.left() - one.left()).abs() < 1e-4 && (many.width() - 11.0 * slot).abs() < 1e-4, "{many:?}");
    let all = band(LevelRange::new(0.0, 63.0));
    assert_eq!((all.left(), all.right()), (rect.left(), rect.right()));
    assert_eq!(selection_shapes(rect, 64, LevelRange::new(10.0, 20.0), 1.5, Color32::WHITE).len(), 3, "the band and an edge at each end");
}

fn bins() -> HistogramData {
    HistogramData::tone(&[5u32; BINS], ChannelColor::Neutral)
}

fn start_range() -> Option<LevelRange> {
    Some(LevelRange::new(10.0, 20.0))
}

/// A selection histogram whose range the host keeps, with the clears and the level under the pointer it was told of.
fn show_selection(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let data_ptr = bins();
    let range = data.value("range", start_range());
    let out = Histogram::new("Histogram", &data_ptr).selection(Bind::bound(range)).show(ui);
    let cleared = matches!(out.selection.event.as_ref().and_then(|event| event.value()), Some(None));
    *data.value("clears", 0u32) += u32::from(cleared);
    *data.value("hovered", None::<f32>) = out.hovered_level;
    out.selection.into()
}

fn selection_fixture() -> Fixture {
    let mut fixture = Fixture::story(&Story::new("histogram-selection", show_selection));
    fixture.settle();
    fixture
}

fn range_of(fixture: &Fixture) -> Option<LevelRange> {
    *fixture.data().value("range", start_range())
}

fn clears(fixture: &Fixture) -> u32 {
    *fixture.data().value("clears", 0u32)
}

#[trial]
fn a_dragged_selection_reports_its_range() {
    let mut fixture = selection_fixture();
    let plot = fixture.by_label("Histogram").expect("the plot is named by its label");
    let by = Vec2::new(plot.rect.width() * 0.3, 0.0);
    fixture.drag(&plot, by);
    let from = level_at(plot.rect, BINS, plot.center().x);
    let to = level_at(plot.rect, BINS, plot.center().x + by.x);
    assert_eq!(range_of(&fixture), Some(LevelRange::new(from, to)));
    assert_eq!(clears(&fixture), 0, "a drag does not clear");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(clears(&fixture), 0, "Enter activates nothing");
}

#[trial]
fn the_arrow_keys_move_the_selection_edges() {
    let mut fixture = selection_fixture();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(range_of(&fixture), Some(LevelRange::new(10.0, 21.0)), "the arrow moves the end edge");
    fixture.key(Key::ArrowLeft, Modifiers::SHIFT);
    fixture.idle(2);
    assert_eq!(range_of(&fixture), Some(LevelRange::new(9.0, 21.0)), "Shift moves the start edge");
}

#[trial]
fn a_click_or_escape_clears_the_selection_once_and_is_reported() {
    let mut fixture = selection_fixture();
    let plot = fixture.by_label("Histogram").expect("the plot is named by its label");
    fixture.click(&plot);
    assert_eq!((range_of(&fixture), clears(&fixture)), (None, 1), "a click clears");
    fixture.click(&plot);
    assert_eq!(clears(&fixture), 1, "a click without a selection reports nothing");

    let mut fixture = selection_fixture();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!((range_of(&fixture), clears(&fixture)), (None, 1), "Escape clears");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(clears(&fixture), 1);
}

#[trial]
fn the_output_reports_the_level_under_the_pointer() {
    let mut fixture = selection_fixture();
    let plot = fixture.by_label("Histogram").expect("the plot is named by its label");
    let level = |fixture: &Fixture| *fixture.data().value("hovered", None::<f32>);
    let inside = pos2(plot.rect.left() + 0.375 * plot.rect.width(), plot.rect.center().y);
    fixture.frame(vec![pointer::moved(inside)]);
    fixture.idle(2);
    assert_eq!(level(&fixture), Some(level_at(plot.rect, BINS, inside.x)));
    fixture.frame(vec![pointer::moved(pos2(plot.rect.right() + 50.0, plot.rect.center().y))]);
    fixture.idle(2);
    assert_eq!(level(&fixture), None);
}

fn show_stale(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let button = *data.value("button", false);
    let data_ptr = bins();
    let mark = if button { StaleMark::Button("Recalculate".into()) } else { StaleMark::Mark("Cached data".into()) };
    let out = Histogram::new("Histogram", &data_ptr).stale(mark).show(ui);
    *data.value("refreshes", 0u32) += u32::from(out.refresh);
    out.selection.into()
}

fn stale_refreshes(button: bool) -> (bool, u32) {
    let mut fixture = Fixture::story(&Story::new("histogram-stale", show_stale));
    *fixture.data().value("button", false) = button;
    fixture.settle();
    let name = if button { "Recalculate" } else { "Cached data" };
    let mark = fixture.by_label(name).expect("the stale mark is named");
    let is_button = fixture.by_role(Role::BUTTON).iter().any(|target| target.node == mark.node);
    fixture.click(&mark);
    (is_button, *fixture.data().value("refreshes", 0u32))
}

#[trial]
fn the_stale_mark_is_a_button_only_as_a_stale_button() {
    assert_eq!(stale_refreshes(false), (false, 0));
    assert_eq!(stale_refreshes(true), (true, 1));
}

fn show_markers(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let data = HistogramData::tone(&(0..256).map(|i| 100 + i % 7).collect::<Vec<u32>>(), ChannelColor::Neutral);
    let markers = [(10.0, "Black".into()), (250.0, "White".into())];
    Histogram::new("Histogram", &data).markers(&markers).show(ui).selection.into()
}

#[trial]
fn markers_draw_a_caption_each_and_name_it() {
    let mut fixture = Fixture::story(&Story::new("histogram-markers", show_markers));
    fixture.settle();
    let plot = fixture.by_label("Histogram").expect("the plot is named by its label").rect;
    let (black, white) = (fixture.by_label("Black").expect("a caption").rect, fixture.by_label("White").expect("a caption").rect);
    assert!(plot.contains_rect(black) && plot.contains_rect(white), "{black:?} {white:?} in {plot:?}");
    assert!(black.center().x < white.center().x);
    assert!(white.right() <= plot.right(), "a caption near the end turns to the other side of its tick");
}
