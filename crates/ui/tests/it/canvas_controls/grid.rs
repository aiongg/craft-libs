//! The canvas grid and the pixel grid (20.6): line counts, the per-axis cap, subdivisions, and the pixel grid's maths and
//! region cache.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Vec2};
use craft_ui::canvas::{CanvasSpace, PixelRect};
use craft_ui::canvas_controls::{CanvasGrid, GridLine, MAX_LINES, PixelGrid};
use craft_ui::testing::trial;
use egui::{Color32, Pos2};

const PIXELS_PER_POINT: f32 = 1.0;
const SIDE: f32 = 400.0;

/// A canvas wide enough to show more lines than the cap allows.
const HUGE_SIDE: f32 = 1.0e6;
const MAJOR: f64 = 100.0;

/// A view of a `SIDE` square canvas showing the document from (0, 0) at `zoom` points per unit.
fn space(zoom: f64) -> CanvasSpace {
    square_space(SIDE, zoom)
}

/// The same for a canvas `side` points wide.
fn square_space(side: f32, zoom: f64) -> CanvasSpace {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(side));
    let half = f64::from(side) / zoom / 2.0;
    CanvasSpace::new(CanvasView::new(rect, Point::new(half, half), zoom), PIXELS_PER_POINT)
}

fn grid() -> CanvasGrid {
    CanvasGrid::new("Grid").spacing(Vec2::new(MAJOR, MAJOR))
}

#[trial]
fn major_lines_fall_every_step_from_the_origin_across_the_view() {
    let lines = grid().lines(&space(1.0));
    assert_eq!(lines.x.major, vec![0.0, 100.0, 200.0, 300.0, 400.0]);
    assert_eq!(lines.y.major, vec![0.0, 100.0, 200.0, 300.0, 400.0]);
    assert!(lines.x.minor.is_empty(), "one subdivision is no minor lines");
    let moved = grid().origin(Point::new(30.0, 0.0)).lines(&space(1.0));
    assert_eq!(moved.x.major, vec![30.0, 130.0, 230.0, 330.0], "the lines count from the origin");
}

#[trial]
fn subdivisions_add_minor_lines_between_majors_while_they_stay_apart() {
    let lines = grid().subdivisions(4).lines(&space(1.0));
    assert_eq!(lines.x.major.len(), 5);
    assert_eq!(lines.x.minor.len(), 12, "three minors between each pair of majors, four gaps in the view");
    let close = grid().subdivisions(40).lines(&space(1.0));
    assert!(close.x.minor.is_empty(), "minor lines under six points apart are left out");
    assert_eq!(close.x.major.len(), 5, "the majors stay");
}

#[trial]
fn an_axis_with_a_step_under_six_points_or_no_step_draws_nothing() {
    assert!(grid().lines(&space(0.05)).x.major.is_empty(), "a 100-unit step at 0.05 is 5 points");
    let no_x = grid().spacing(Vec2::new(0.0, MAJOR)).lines(&space(1.0));
    assert!(no_x.x.major.is_empty() && !no_x.y.major.is_empty(), "a step that is not positive draws nothing on its axis");
}

#[trial]
fn lines_per_axis_are_capped() {
    let many = CanvasGrid::new("Grid").spacing(Vec2::new(1.0, 1.0)).lines(&square_space(HUGE_SIDE, 1.0));
    assert_eq!(many.x.major.len(), MAX_LINES);
    assert_eq!(many.y.major.len(), MAX_LINES);
}

#[trial]
fn bounds_cut_the_lines_and_a_zoom_floor_hides_the_grid() {
    let bounded = grid().bounds(craft_behavior::kurbo::Rect::new(0.0, 0.0, 150.0, 150.0)).lines(&space(1.0));
    assert_eq!(bounded.x.major, vec![0.0, 100.0]);
    let outside = grid().bounds(craft_behavior::kurbo::Rect::new(900.0, 900.0, 950.0, 950.0)).lines(&space(1.0));
    assert!(outside.extent.is_none(), "bounds the view does not show draw nothing");
    assert!(grid().min_zoom(2.0).lines(&space(1.0)).extent.is_none(), "below the floor");
    assert!(grid().min_zoom(2.0).lines(&space(3.0)).extent.is_some());
    assert_eq!(GridLine::default(), GridLine::Solid);
}

#[trial]
fn crossings_need_a_major_step_of_eight_points() {
    assert!(grid().line(GridLine::Crossings).lines(&space(1.0)).crossings);
    assert!(!grid().spacing(Vec2::new(7.0, MAJOR)).lines(&space(1.0)).crossings);
}

#[trial]
fn the_pixel_grid_shows_only_above_five_hundred_percent() {
    assert!(!PixelGrid::shows_at(5.0));
    assert!(!PixelGrid::shows_at(1.0));
    assert!(PixelGrid::shows_at(5.01));
}

#[trial]
fn the_pixel_grid_has_no_line_over_see_through_pixels_and_fades_by_luma() {
    assert_eq!(PixelGrid::cell_color([0.0; 4]), None);
    assert_eq!(PixelGrid::cell_color([1.0, 1.0, 1.0, 0.0]), None);
    assert_eq!(PixelGrid::cell_color([0.5, 0.5, 0.5, f32::NAN]), None);
    assert_eq!(PixelGrid::cell_color([0.1, 0.1, 0.1, 1.0]), Some(Color32::from_white_alpha(64)), "a dark pixel gets a light line");
    assert_eq!(PixelGrid::cell_color([0.9, 0.9, 0.9, 1.0]), Some(Color32::from_black_alpha(41)), "a light pixel gets a dark line");
    assert_eq!(PixelGrid::light_share(0.2), 0.0);
    assert_eq!(PixelGrid::light_share(0.9), 1.0);
    assert_eq!(PixelGrid::light_share(f32::NAN), 0.0);
}

#[trial]
fn runs_of_one_colour_merge_into_one_segment() {
    let dark = [[0.1, 0.1, 0.1, 1.0]; 16];
    let segments = PixelGrid::segments(&dark, PixelRect::new(10, 20, 14, 24));
    assert_eq!(segments.len(), 8, "four vertical lines and four horizontal ones");
    assert!(segments.iter().any(|s| s.from == [10, 20] && s.to == [10, 24]));
    assert!(segments.iter().any(|s| s.from == [10, 23] && s.to == [14, 23]));
    let mut holey = dark;
    holey[5] = [0.0; 4];
    assert!(PixelGrid::segments(&holey, PixelRect::new(0, 0, 4, 4)).len() > 8, "a see-through cell splits the runs");
}

#[trial]
fn the_cached_region_snaps_outward_inside_the_document() {
    let bounds = PixelRect::new(0, 0, 1000, 1000);
    let region = PixelGrid::cache_region(PixelRect::new(70, 130, 90, 140), bounds);
    assert_eq!(region, PixelRect::new(64, 128, 128, 192));
    assert_eq!(PixelGrid::cache_region(PixelRect::new(990, 990, 1100, 1100), bounds), PixelRect::new(960, 960, 1000, 1000), "cut to the document");
    assert!(PixelGrid::cache_region(PixelRect::new(2000, 0, 2010, 10), bounds).is_empty(), "a view off the document needs nothing");
}
