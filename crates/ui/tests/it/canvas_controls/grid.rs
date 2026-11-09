//! The canvas grid and the pixel grid (20.6): line counts, the per-axis cap, subdivisions, and the pixel grid's maths and
//! region cache.

use craft_behavior::kurbo::{Point, Vec2};
use craft_behavior::{CanvasView, PixelSize};
use craft_ui::canvas::{Canvas, CanvasSpace, PixelRect, RasterSource};
use craft_ui::canvas_controls::{CanvasGrid, GridLine, GridSegment, PixelGrid};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::primitives::Revision;
use craft_ui::testing::{Fixture, trial};
use egui::{Color32, ColorImage, Pos2};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

const PIXELS_PER_POINT: f32 = 1.0;
const SIDE: f32 = 400.0;

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

/// A grid step of `x` and `y` document units.
fn step(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn grid() -> CanvasGrid {
    CanvasGrid::new("Grid").spacing(step(MAJOR, MAJOR))
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

/// The colour of the pixel at (`x`, `y`) of a document the tests draw: some see-through, the rest in four greys.
fn pixel(x: i32, y: i32, shift: i32) -> [f32; 4] {
    let grey = ((x * 3 + y * 2 + shift).rem_euclid(4)) as f32 / 3.0;
    [grey, grey, grey, if (x + y).rem_euclid(5) == 0 { 0.0 } else { 1.0 }]
}

fn pixels(rect: PixelRect, shift: impl Fn(i32, i32) -> i32) -> Vec<[f32; 4]> {
    (rect.y0..rect.y1).flat_map(|y| (rect.x0..rect.x1).map(move |x| (x, y))).map(|(x, y)| pixel(x, y, shift(x, y))).collect()
}

/// Every unit edge the segments cover, with its colour, so runs split differently still compare equal.
fn edges(segments: &[GridSegment]) -> BTreeMap<(bool, i32, i32), Color32> {
    let mut edges = BTreeMap::new();
    for segment in segments {
        if segment.from[0] == segment.to[0] {
            (segment.from[1]..segment.to[1]).for_each(|y| drop(edges.insert((true, segment.from[0], y), segment.color)));
        } else {
            (segment.from[0]..segment.to[0]).for_each(|x| drop(edges.insert((false, x, segment.from[1]), segment.color)));
        }
    }
    edges
}

#[trial]
fn redrawing_a_damaged_part_of_the_pixel_grid_matches_drawing_the_region_again() {
    let region = PixelRect::new(0, 0, 12, 10);
    let dirty = PixelRect::new(3, 2, 7, 6);
    let old = PixelGrid::segments(&pixels(region, |_, _| 0), region);
    let inside = |x: i32, y: i32| if (dirty.x0..dirty.x1).contains(&x) && (dirty.y0..dirty.y1).contains(&y) { 1 } else { 0 };
    let patched = PixelGrid::patched(&old, &pixels(dirty, |_, _| 1), dirty);
    let again = PixelGrid::segments(&pixels(region, inside), region);
    assert_eq!(edges(&patched), edges(&again));
}

/// A document that records its reads and answers `damage` as the test sets it.
struct Document {
    revision: Cell<u64>,
    damage: Cell<Option<PixelRect>>,
    reads: RefCell<Vec<PixelRect>>,
}

impl RasterSource for Document {
    fn size(&self) -> PixelSize {
        PixelSize::new(256, 256)
    }

    fn revision(&self) -> Revision {
        Revision::new(self.revision.get())
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        self.damage.get()
    }

    fn read(&self, region: PixelRect, _: u32, out: &mut ColorImage) {
        self.reads.borrow_mut().push(region);
        out.pixels.fill(Color32::from_gray(40));
    }
}

#[trial]
fn the_pixel_grid_reads_only_what_the_source_reports_damaged() {
    const ZOOM: f64 = 8.0;
    let document = Rc::new(Document { revision: Cell::new(1), damage: Cell::new(None), reads: RefCell::new(Vec::new()) });
    let shown = Rc::clone(&document);
    let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(200.0));
    let view = CanvasView::new(rect, Point::new(128.0, 128.0), ZOOM);
    let mut fixture = Fixture::new(move |ui| {
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            PixelGrid::new("Pixel grid", &*shown).show_at(ui, body.rect);
        });
    });
    fixture.room(rect.size());
    fixture.settle();
    let region = *document.reads.borrow().first().expect("the first pass reads the region around the view");

    let mut frame = |revision: u64, damage: Option<PixelRect>| {
        document.reads.borrow_mut().clear();
        document.revision.set(revision);
        document.damage.set(damage);
        fixture.frame(Vec::new());
        document.reads.borrow().clone()
    };
    let small = PixelRect::new(region.x0 + 4, region.y0 + 4, region.x0 + 9, region.y0 + 8);
    assert_eq!(frame(2, Some(small)), vec![small], "the damaged part is read alone");
    assert!(frame(3, Some(PixelRect::new(0, 0, 1, 1))).is_empty(), "damage outside the cached region reads nothing");
    assert_eq!(frame(4, None), vec![region], "a change the source cannot place reads the region");
}

#[trial]
fn the_grid_paints_the_same_lines_while_the_view_stays_and_follows_it_when_it_changes() {
    let zoom = Rc::new(Cell::new(1.0));
    let shown = Rc::clone(&zoom);
    let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(400.0));
    let mut fixture = Fixture::new(move |ui| {
        let view = CanvasView::new(rect, Point::new(200.0, 200.0), shown.get());
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            grid().show_at(ui, body.rect);
        });
    });
    fixture.room(rect.size());
    fixture.settle();
    let painted = |fixture: &mut Fixture| {
        fixture.frame(Vec::new());
        fixture.shape_bounds().to_vec()
    };
    let first = painted(&mut fixture);
    assert!(!first.is_empty());
    assert_eq!(painted(&mut fixture), first, "a second pass with the same view paints the same lines");
    zoom.set(2.0);
    let zoomed = painted(&mut fixture);
    assert_ne!(zoomed, first, "a new zoom paints other lines");
    zoom.set(1.0);
    assert_eq!(painted(&mut fixture), first, "going back paints the first lines, not the ones kept for the other zoom");
}
