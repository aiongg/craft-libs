//! Painting through `Draw`: shapes, colours from the two colour types, and what `Draw` records.

use craft_i18n::Direction;
use craft_theme::{Contrast, ControlSize, Mode, Rgba, Variant};
use craft_ui::fonts::FontRef;
use craft_ui::primitives::{Dashes, DocRun, DocText, HAlign, IconSpec, PaintShape, RuleKind, StrokeWidth, TextRun, TextureCache, ToolIcon, has_icon};
use craft_ui::style::{Corners, DataColor, Ink, SurfaceKind};
use craft_ui::testing::style::{paint_frame, tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Pos2, Rect, Shape, pos2};

const HALF: f32 = 0.5;
const EPS: f32 = 1e-3;
const RECT: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 40.0));

fn color(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0[0], c.0[1], c.0[2], c.0[3])
}

fn stroke_is(shape: &Shape, color: Color32) -> bool {
    match shape {
        Shape::LineSegment { stroke, .. } => stroke.color == color,
        Shape::Path(path) => path.stroke.color == egui::epaint::ColorMode::Solid(color),
        _ => false,
    }
}

#[trial]
fn a_rule_sits_on_the_pixel_grid_whatever_the_coordinate() {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
        draw.rule(RuleKind::Divider, Pos2::new(5.0, 20.3), Pos2::new(95.0, 20.3));
    });
    let ppp = frames.ctx.pixels_per_point();
    let line = shapes.iter().find_map(|s| match s {
        Shape::LineSegment { points, stroke } => Some((*points, stroke.width)),
        _ => None,
    });
    assert!(line.is_some(), "the rule painted a line segment");
    let (points, width) = line.unwrap_or_default();
    let physical = (width * ppp).round();
    let centre = points[0].y * ppp;
    if physical % 2.0 == 1.0 {
        assert!((centre.fract() - HALF).abs() < EPS, "an odd-width line is centred on a pixel");
    } else {
        assert!(centre.fract().abs() < EPS, "an even-width line sits on a pixel edge");
    }
    assert_eq!(points[0].y, points[1].y, "the line stays level");
}

#[trial]
fn every_tool_names_a_catalog_icon_and_an_id_that_round_trips() {
    let mut ids = std::collections::HashSet::new();
    let all = ToolIcon::ALL;
    for tool in &all {
        assert!(has_icon(&tool.icon()), "{tool:?} names an icon the catalog lacks");
        assert!(ids.insert(tool.id()), "{tool:?} repeats the id {}", tool.id());
        assert_eq!(ToolIcon::from_id(tool.id()).as_ref(), Some(tool));
    }
    assert_eq!(ToolIcon::from_id("nonsense"), None);
}

#[trial]
fn an_icon_loads_its_texture_once_and_paints_it_every_frame() {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let spec = IconSpec::svg(ToolIcon::Brush.icon());
    for _ in 0..3 {
        let (shapes, _) = paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| draw.icon(RECT, &spec));
        assert!(!shapes.is_empty(), "the icon paints a textured shape");
    }
    assert_eq!(TextureCache::len(&frames.ctx), 1, "one texture serves every frame");
}

/// Hashes like the `u64` it holds, so only the cache's own key namespacing tells it apart.
#[derive(Hash)]
struct Stamp(u64);

#[trial]
fn texture_keys_of_different_types_name_different_textures() {
    let frames = Frames::new();
    let image = || egui::ColorImage::filled([2, 2], Default::default());
    let plain = TextureCache::texture(&frames.ctx, 7u64, image);
    let wrapped = TextureCache::texture(&frames.ctx, Stamp(7), image);
    assert_ne!(plain, wrapped, "equal hashes of different key types share one texture");
    assert_eq!(TextureCache::len(&frames.ctx), 2);
}

#[trial]
fn textures_nobody_asks_for_are_freed_once_the_cache_is_over_capacity() {
    let mut frames = Frames::new();
    let image = || egui::ColorImage::filled([1, 1], Default::default());
    let ctx = frames.ctx.clone();
    frames.frame(Vec::new(), |_| {
        for key in 0..TextureCache::CAPACITY as u64 + 10 {
            TextureCache::texture(&ctx, key, image);
        }
    });
    assert_eq!(TextureCache::len(&ctx), TextureCache::CAPACITY + 10, "textures asked for in the running frame stay");
    frames.frame(Vec::new(), |_| {
        TextureCache::texture(&ctx, u64::MAX, image);
    });
    assert_eq!(TextureCache::len(&ctx), TextureCache::CAPACITY, "the oldest unused textures are freed down to the capacity");
}

/// How many textures painting the icon `name` in each direction loads.
fn textures_for_both_directions(name: &'static str) -> usize {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let spec = IconSpec::svg(craft_behavior::IconId::new(name));
    for direction in [Direction::Ltr, Direction::Rtl] {
        let (shapes, _) = paint_frame(&mut frames, &tokens, direction, Ink::OnPanel, |draw| draw.icon(RECT, &spec));
        assert!(!shapes.is_empty(), "{name} paints in {direction:?}");
    }
    TextureCache::len(&frames.ctx)
}

#[trial]
fn a_directional_svg_icon_mirrors_under_rtl_and_another_stays() {
    assert_eq!(textures_for_both_directions("chevron_right"), 2, "the mirrored raster is its own texture");
    assert_eq!(textures_for_both_directions("close"), 1, "an icon that does not point along the reading direction keeps one texture");
}

/// The horizontal extent of the text `doc` paints in `RECT`.
fn doc_extent(direction: Direction, align: HAlign) -> (f32, f32) {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let runs = [DocRun::new("Title", FontRef::proportional(), 14.0, DataColor::new(Color32::from_gray(0)))];
    let mut text = DocText::new(&runs);
    text.align = align;
    let (shapes, _) = paint_frame(&mut frames, &tokens, direction, Ink::OnPanel, |draw| {
        draw.doc(RECT, &text);
    });
    let shown = shapes.iter().find_map(|s| if let Shape::Text(text) = s { Some(text) } else { None }).expect("the text painted");
    let rect = shown.galley.rect.translate(shown.pos.to_vec2());
    (rect.min.x, rect.max.x)
}

#[trial]
fn doc_text_sits_in_its_rect_by_its_alignment_and_start_and_end_swap_under_rtl() {
    let (left, right) = (RECT.left(), RECT.right());
    let (start, end) = doc_extent(Direction::Ltr, HAlign::Start);
    assert!((start - left).abs() < EPS && end < right, "start is the left edge in LTR");
    let (start, end) = doc_extent(Direction::Ltr, HAlign::End);
    assert!((end - right).abs() < EPS && start > left, "end is the right edge in LTR");
    let (start, end) = doc_extent(Direction::Ltr, HAlign::Center);
    assert!(((start + end) / 2.0 - RECT.center().x).abs() < 1.0, "centre is the middle of the rect");
    let (start, end) = doc_extent(Direction::Rtl, HAlign::Start);
    assert!((end - right).abs() < EPS && start > left, "start is the right edge in RTL");
    let (start, _) = doc_extent(Direction::Rtl, HAlign::End);
    assert!((start - left).abs() < EPS, "end is the left edge in RTL");
}
