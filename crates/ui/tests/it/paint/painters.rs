//! The pure painters: focus rings, scroll bars, handles and the face id.

use craft_behavior::PartKey;
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::primitives::{BarAxis, GridInk, GridLines, ScrollBar, Tick, TickEdge, TickRank, grid_lines, ticks};
use craft_ui::style::{DataColor, StateSet};
use craft_ui::testing::style::{face_id, tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Rect, Shape, pos2, vec2};

const EPS: f32 = 1e-3;
const END: f32 = 800.0;
const MIDDLE: f32 = 400.0;

#[trial]
fn the_scroll_thumb_tracks_the_offset_and_never_leaves_the_track() {
    let frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let look = with_cx(&frames.ctx, &tokens, Variant::SECONDARY, ControlSize::Md, |cx| cx.scroll_bar());
    let track = Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 200.0));
    let at = |offset| ScrollBar::thumb(&look, BarAxis::Vertical, track, 1000.0, 200.0, offset);
    assert_eq!(at(0.0).min.y, track.min.y);
    assert!((at(END).max.y - track.max.y).abs() < EPS, "the end of the content puts the thumb at the track's end");
    assert!(at(MIDDLE).min.y > at(0.0).min.y && at(MIDDLE).max.y < at(END).max.y + EPS);
    let tiny = ScrollBar::thumb(&look, BarAxis::Vertical, track, 1_000_000.0, 200.0, 0.0);
    assert!(tiny.height() >= look.min_thumb.min(track.height()), "the thumb keeps a grabbable length");
    let shapes = ScrollBar::shapes(&look, track, at(0.0), &StateSet::default());
    assert_eq!(shapes.len(), 2, "track, then thumb");
}

#[trial]
fn a_face_has_its_own_id_per_part_owned_by_the_composite() {
    const CHECK: PartKey = PartKey::new("check");
    const CLOSE: PartKey = PartKey::new("close");
    let chip = "chip";
    assert_eq!(face_id(chip, CLOSE.into()), face_id(chip, CLOSE.into()));
    assert_ne!(face_id(chip, CLOSE.into()), face_id(chip, CHECK.into()), "parts hover and press apart");
    assert_ne!(face_id(chip, CLOSE.at(1)), face_id(chip, CLOSE.at(2)), "repeated parts too");
    assert_ne!(face_id(chip, CLOSE.into()), face_id("other", CLOSE.into()), "two composites do not share faces");
}

#[trial]
fn ellipse_points_lie_on_the_ellipse_whatever_the_conjugate_axes() {
    let center = pos2(50.0, 40.0);
    // A circle of radius 30 turned by 30 degrees and squeezed to a skew: the axes are conjugate, not orthogonal.
    let axes = [vec2(30.0, 0.0), vec2(15.0, 20.0)];
    let points = craft_ui::primitives::ellipse(center, axes);
    assert!(points.len() >= 16);
    // Solve p - center = a * axes[0] + b * axes[1]; the points are where a^2 + b^2 = 1.
    let det = axes[0].x * axes[1].y - axes[0].y * axes[1].x;
    for p in points {
        let d = p - center;
        let a = (d.x * axes[1].y - d.y * axes[1].x) / det;
        let b = (axes[0].x * d.y - axes[0].y * d.x) / det;
        assert!((a * a + b * b - 1.0).abs() < EPS, "{p:?} is off the ellipse");
    }
}

#[trial]
fn the_checker_puts_dark_cells_on_alternate_squares_from_the_corner() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let side = tokens.size.checker;
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(side * 3.0, side * 2.0));
    let shapes = craft_ui::primitives::checker(&tokens, rect, rect);
    // The light base, then the dark squares (0,0), (2,0) and (1,1).
    assert_eq!(shapes.len(), 1 + 3);
    let Shape::Rect(first) = &shapes[1] else { panic!("a dark cell is a rect") };
    assert_eq!(first.rect.min, rect.min, "the first dark cell sits in the corner");
}

#[trial]
fn checker_and_grid_shapes_stay_within_the_clip_of_a_huge_rect() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let huge = Rect::from_min_size(pos2(-1.0e6, -1.0e6), vec2(2.0e6, 2.0e6));
    let clip = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
    let checker = craft_ui::primitives::checker(&tokens, huge, clip);
    assert!(checker.len() < 10_000, "{} checker shapes for a 100 point clip", checker.len());
    let grid = grid_lines(&tokens, &GridLines::screen(huge, vec2(0.001, 10.0), clip, 2.0), &GridInk::new(), 2.0);
    assert!(grid.len() < 10_000, "{} grid shapes for a 100 point clip", grid.len());
    let reach = clip.expand(2.0);
    for shape in checker.iter().chain(&grid) {
        assert!(reach.contains_rect(shape.visual_bounding_rect()), "{:?} reaches outside the clip", shape.visual_bounding_rect());
    }
}

#[trial]
fn a_document_grid_draws_minor_lines_in_their_own_colour_through_the_view() {
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let view = craft_behavior::CanvasView::new(
        Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 400.0)),
        craft_behavior::kurbo::Point::new(200.0, 200.0),
        1.0,
    );
    let extent = craft_behavior::kurbo::Rect::new(0.0, 0.0, 400.0, 400.0);
    let lines = GridLines::document(&view, extent, craft_behavior::kurbo::Vec2::new(100.0, 100.0), craft_behavior::kurbo::Point::ZERO, 4);
    let (major, minor) = (Color32::from_rgb(200, 0, 0), Color32::from_rgb(0, 0, 200));
    let shapes = grid_lines(&tokens, &lines, &GridInk::new().major(DataColor::new(major)).minor(DataColor::new(minor)), 1.0);
    let colors: Vec<Color32> =
        shapes.iter().map(|shape| if let Shape::LineSegment { stroke, .. } = shape { stroke.color } else { panic!("a line") }).collect();
    let count = |color| colors.iter().filter(|c| **c == color).count();
    assert_eq!((count(major), count(minor)), (lines.x.major.len() + lines.y.major.len(), lines.x.minor.len() + lines.y.minor.len()));
    assert!(
        count(minor) > 0 && colors.iter().position(|c| *c == minor) < colors.iter().position(|c| *c == major),
        "minor lines sit under the majors"
    );
}

/// A labelled tick runs the whole depth of its strip, beside its label; the halfway tick is shorter and the others
/// shorter still, on whichever edge the strip sits.
#[trial]
fn a_major_tick_spans_the_strip_and_outranks_mid_and_minor_ticks() {
    let tokens = tokens(Mode::Dark, Contrast::Standard);
    let strip = Rect::from_min_size(pos2(10.0, 20.0), vec2(300.0, 16.0));
    for (edge, depth) in [(TickEdge::Bottom, strip.height()), (TickEdge::Right, strip.width())] {
        let length = |rank| {
            let shapes = ticks(&tokens, strip, edge, &[Tick::new(5.0, rank)], Rect::EVERYTHING, 1.0);
            let bounds = shapes.iter().map(Shape::visual_bounding_rect).fold(Rect::NOTHING, |a, b| a.union(b));
            if matches!(edge, TickEdge::Bottom) { bounds.height() } else { bounds.width() }
        };
        let (major, mid, minor) = (length(TickRank::Major), length(TickRank::Mid), length(TickRank::Minor));
        assert!(major >= depth - EPS, "{edge:?}: the major tick spans the strip: {major} of {depth}");
        assert!(major > mid && mid > minor && minor > 0.0, "{edge:?}: major {major}, mid {mid}, minor {minor}");
    }
}
