//! The selection outlines (20.6): a press away from every canvas control reaches the tool, a marquee maps its shape
//! through the view, the hover mask finds its edges, and the range bands tile.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect};
use craft_ui::canvas::Canvas;
use craft_ui::canvas_controls::{
    CanvasGrid, HoverMask, HoverOutline, HoverShape, Marquee, MarqueeLook, MarqueeShape, SelectionOutline, StraightLinePreview, range_bands,
    rect_around,
};
use craft_ui::contract::{Role, Show as _, ShowContainer as _};
use craft_ui::style::DataColor;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::epaint::ColorMode;
use egui::{Color32, Pos2, Shape, Vec2};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const VIEW_AT: Pos2 = Pos2::new(10.0, 10.0);
const VIEW_SIZE: Vec2 = Vec2::new(200.0, 120.0);
const BAND_WIDTH: f32 = 4.0;

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(VIEW_AT, VIEW_SIZE), Point::new(100.0, 60.0), 2.0)
}

fn selection() -> Rect {
    Rect::new(10.0, 20.0, 50.0, 40.0)
}

#[trial]
fn a_press_on_the_canvas_reaches_the_tool_with_every_selection_outline_shown() {
    let clicked = Rc::new(RefCell::new(None));
    let seen = Rc::clone(&clicked);
    let paths = [craft_behavior::kurbo::BezPath::new()];
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Canvas::new("Document").view(view()).show(ui, |ui, body| {
            Marquee::rect("Selection", selection()).show_at(ui, body.rect);
            Marquee::polygon("Lasso", [Point::new(0.0, 0.0), Point::new(30.0, 10.0)]).look(MarqueeLook::Drawing).show_at(ui, body.rect);
            HoverOutline::new("Rollover", Some(HoverShape::Rect(selection()))).show_at(ui, body.rect);
            SelectionOutline::solid("Selected", &paths).show_at(ui, body.rect);
            StraightLinePreview::new("Line", Point::new(0.0, 0.0), Point::new(40.0, 40.0)).show_at(ui, body.rect);
            CanvasGrid::new("Grid").show_at(ui, body.rect);
        });
        if let Some(at) = canvas.output.click_at() {
            *seen.borrow_mut() = Some(at);
        }
    });
    fixture.settle();
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas has a node");
    fixture.click(&canvas);
    let at = clicked.borrow().expect("the controls take no press, so the tool gets it");
    assert!(canvas.rect.contains(at), "the tool gets the point inside the canvas");
}

#[trial]
fn the_hover_mask_marks_the_edge_of_the_covered_texels_only() {
    let bounds = Rect::new(0.0, 0.0, 9.0, 9.0);
    let mask = HoverMask::sampled(bounds, |x, y| if (3.0..6.0).contains(&x) && (3.0..6.0).contains(&y) { 1.0 } else { 0.0 });
    assert!(mask.is_edge(3, 3), "a corner texel of the covered block");
    assert!(mask.is_edge(4, 3), "a texel beside an uncovered one");
    assert!(!mask.is_edge(4, 4), "the middle of the block");
    assert!(!mask.is_edge(1, 1), "an uncovered texel is not an edge");
    let full = HoverMask::sampled(bounds, |_, _| 1.0);
    assert!(full.is_edge(0, 4) && full.is_edge(8, 4), "texels on the mask's border are edges");
    assert!(!full.is_edge(4, 4));
}

#[trial]
fn range_bands_are_the_four_sides_cut_to_the_clip() {
    let rect = egui::Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(50.0, 40.0));
    let clip = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0));
    assert_eq!(range_bands(rect, BAND_WIDTH, clip).len(), 4);
    let far = egui::Rect::from_min_max(Pos2::new(-2.0e7, 10.0), Pos2::new(2.0e7, 40.0));
    let bands = range_bands(far, BAND_WIDTH, clip);
    assert!(bands.iter().all(|band| band.width() <= clip.width() + 4.0 * BAND_WIDTH), "no side keeps its 40-million-point length");
    assert!(range_bands(egui::Rect::NOTHING, BAND_WIDTH, clip).is_empty());
}

#[trial]
fn rect_around_leaves_the_hole_clear_without_overlap() {
    let area = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(30.0, 30.0));
    let hole = egui::Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(20.0, 20.0));
    let parts = rect_around(area, hole);
    assert_eq!(parts.len(), 4);
    let covered: f32 = parts.iter().map(|part| part.area()).sum();
    assert_eq!(covered, area.area() - hole.area(), "the pieces tile the area without the hole");
    assert!(parts.iter().all(|part| !part.intersects(hole.shrink(1.0))));
    assert_eq!(
        rect_around(area, egui::Rect::from_min_max(Pos2::new(40.0, 40.0), Pos2::new(50.0, 50.0))),
        vec![area],
        "a hole that misses leaves the area whole"
    );
}

/// The seconds between steps of the ants.
const MARCH_TICK: Duration = Duration::from_millis(100);

#[trial]
fn a_shown_selection_asks_for_its_next_repaint_a_march_step_ahead() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        Canvas::new("Document").view(view()).show(ui, |ui, body| {
            Marquee::rect("Selection", selection()).show_at(ui, body.rect);
        });
    };
    frames.idle(3, &mut show);
    // Two march steps of frames. The wait counts down to the step, and a wake under a frame ahead is a repaint now, so a step costs two at most.
    let waits: Vec<Option<Duration>> = (0..12)
        .map(|_| {
            frames.frame(Vec::new(), &mut show);
            frames.repaint_delay()
        })
        .collect();
    let immediate = waits.iter().filter(|wait| **wait == Some(Duration::ZERO)).count();
    assert!(waits.iter().all(|wait| wait.is_some_and(|wait| wait <= MARCH_TICK)), "the ants ask for a repaint within a step: {waits:?}");
    assert!(immediate * 3 <= waits.len(), "a repaint is not asked for every frame: {waits:?}");
}

