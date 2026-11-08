//! The view tools: what a gesture asks the app's view to become, and the checks every tool owes the canvas.

use craft_behavior::kurbo::{Point, Size};
use craft_behavior::{CancelReason, CanvasView, PointerButton};
use craft_ui::canvas::{
    CanvasSpace, CanvasTool, HandTool, NavigationOptions, RotateViewTool, ToolContext, ToolKey, ToolPointer, ViewCommands, ZoomTool,
};
use craft_ui::testing::{ToolFixture, trial};
use egui::{Pos2, Rect, Vec2};

const CANVAS: Vec2 = Vec2::new(800.0, 600.0);
const DOCUMENT: Size = Size::new(2000.0, 1500.0);
const DRAG: Vec2 = Vec2::new(60.0, 40.0);

fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Point::new(1000.0, 750.0), 1.0)
}

const SLOP: f32 = 6.0;

fn context<'a>(view: CanvasView, time: f64) -> ToolContext<'a> {
    ToolContext::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Some(CanvasSpace::new(view, 1.0).with_document(Some(DOCUMENT)))).time(time).slop(SLOP)
}

fn at(pos: Pos2, time: f64) -> ToolPointer {
    ToolPointer::new(pos, PointerButton::Primary, time)
}

#[trial]
fn the_hand_drag_moves_the_document_with_the_pointer_and_escape_puts_the_view_back() {
    let start = view();
    let mut hand = HandTool::new(NavigationOptions::default());
    let from = Pos2::new(300.0, 300.0);
    hand.press(&at(from, 0.0), &mut context(start, 0.0));
    let mut dragged = context(start, 0.5);
    hand.drag(&at(from + DRAG, 0.5), &mut dragged);
    let moved = dragged.requested_view().expect("a drag asks for a new view");
    let point = Point::new(1000.0, 750.0);
    let before = start.to_screen(point);
    let after = moved.to_screen(point);
    assert!((after - before - DRAG).length() < 1e-3, "the document follows the pointer: {before:?} -> {after:?}");

    let mut cancelled = context(moved, 0.6);
    hand.cancel(CancelReason::Escape, &mut cancelled);
    assert_eq!(cancelled.requested_view(), Some(start), "a cancelled drag restores the view");
}

#[trial]
fn a_zoom_click_steps_the_ladder_about_the_point_and_alt_steps_out() {
    let start = view();
    let click = Pos2::new(200.0, 100.0);
    let mut zoom = ZoomTool::new(NavigationOptions::default());
    zoom.press(&at(click, 0.0), &mut context(start, 0.0));
    let mut released = context(start, 0.05);
    zoom.release(&at(click, 0.05), &mut released);
    let zoomed = released.requested_view().expect("a click zooms");
    assert!(zoomed.zoom > start.zoom && released.requested_ease().is_some(), "a click zooms in, eased under Animated Zoom");
    let doc = start.to_doc(click);
    let kept = zoomed.to_doc(click);
    assert!((kept.x - doc.x).abs() < 1e-6 && (kept.y - doc.y).abs() < 1e-6, "the clicked point stays put");

    zoom.out = true;
    zoom.press(&at(click, 1.0), &mut context(start, 1.0));
    let mut released = context(start, 1.05);
    zoom.release(&at(click, 1.05), &mut released);
    assert!(released.requested_view().expect("an out click zooms").zoom < start.zoom);
}

#[trial]
fn the_zoom_click_stays_within_the_ladder_of_the_document_the_canvas_was_given() {
    let start = view();
    let click = Pos2::new(200.0, 100.0);
    let out = |document: Option<Size>| {
        let mut zoom = ZoomTool::new(NavigationOptions::default());
        zoom.out = true;
        let space = |time| {
            let space = CanvasSpace::new(start, 1.0).with_document(document);
            ToolContext::new(Rect::from_min_size(Pos2::ZERO, CANVAS), Some(space)).time(time).slop(SLOP)
        };
        zoom.press(&at(click, 0.0), &mut space(0.0));
        let mut released = space(0.05);
        zoom.release(&at(click, 0.05), &mut released);
        released.requested_view()
    };
    let large = out(Some(DOCUMENT)).expect("a click zooms");
    assert!(large.zoom < start.zoom, "a large document zooms out from 100 %");
    let tiny = out(Some(Size::new(2.0, 2.0))).expect("a click at the limit puts the view back");
    assert_eq!(tiny, start, "a document of 2 units has 100 % as its lowest rung");
    assert_eq!(out(None), None, "a canvas given no document gives the Zoom tool no ladder");
}

#[trial]
fn a_zoom_press_held_still_zooms_instead_of_stepping() {
    let start = view();
    let click = Pos2::new(200.0, 100.0);
    let mut zoom = ZoomTool::new(NavigationOptions::default());
    zoom.press(&at(click, 0.0), &mut context(start, 0.0));
    let mut pass = context(start, 0.7);
    zoom.each_pass(&mut pass);
    let held = pass.requested_view().expect("a hold zooms");
    assert!(held.zoom > start.zoom);
    let mut released = context(held, 0.8);
    zoom.release(&at(click, 0.8), &mut released);
    assert_eq!(released.requested_view(), None, "the release of a hold is no click step");
}

#[trial]
fn a_zoom_press_that_dragged_never_steps_whatever_the_rectangle() {
    let start = view();
    let from = Pos2::new(200.0, 100.0);
    let mut options = NavigationOptions::default();
    options.scrubby_zoom = false;
    options.animated_zoom = false;
    let mut zoom = ZoomTool::new(options);
    // Past the arena's slop in one direction only: a sliver, too small to zoom to.
    let sliver = from + Vec2::new(SLOP + 1.0, 1.0);
    zoom.press(&at(from, 0.0), &mut context(start, 0.0));
    zoom.drag(&at(sliver, 0.1), &mut context(start, 0.1));
    let mut released = context(start, 0.2);
    zoom.release(&at(sliver, 0.2), &mut released);
    let view = released.requested_view().unwrap_or(start);
    assert_eq!(view.zoom, start.zoom, "a dragged press that draws a sliver does not step");

    let far = from + Vec2::new(200.0, 150.0);
    zoom.press(&at(from, 1.0), &mut context(start, 1.0));
    zoom.drag(&at(far, 1.1), &mut context(start, 1.1));
    let mut released = context(start, 1.2);
    zoom.release(&at(far, 1.2), &mut released);
    assert!(released.requested_view().expect("a rectangle zooms").zoom > start.zoom, "a dragged rectangle zooms to it");
}

#[trial]
fn the_rotate_drag_turns_the_view_and_shift_snaps_and_a_double_click_resets() {
    let mut turned = view();
    turned.rotation = 40.0;
    let mut tool = RotateViewTool::default();
    let pivot = turned.rect.center();
    tool.press(&at(pivot + Vec2::new(100.0, 0.0), 0.0), &mut context(turned, 0.0));
    let mut dragged = context(turned, 0.1);
    tool.drag(&at(pivot + Vec2::new(0.0, 100.0), 0.1), &mut dragged);
    let quarter = dragged.requested_view().expect("a drag turns the view").rotation;
    assert!((quarter - 130.0).abs() < 1e-6, "a quarter turn clockwise adds 90 degrees: {quarter}");

    let mut released = context(turned, 0.2);
    tool.release(&at(pivot + Vec2::new(0.0, 100.0), 0.2).clicks(2), &mut released);
    assert_eq!(released.requested_view().map(|view| view.rotation), Some(0.0), "a double click resets");

    let mut escaped = context(turned, 0.3);
    assert!(tool.key(&ToolKey::Cancel, &mut escaped), "Escape resets a turned view");
    assert_eq!(escaped.requested_view().map(|view| view.rotation), Some(0.0));
}

#[trial]
fn the_view_commands_zoom_fit_and_turn_the_view() {
    let mut view = view();
    assert!(ViewCommands::ZoomIn.apply(&mut view, DOCUMENT, 1.0) && view.zoom > 1.0);
    assert!(ViewCommands::FitOnScreen.apply(&mut view, DOCUMENT, 1.0));
    assert!(ViewCommands::RotateClockwise.apply(&mut view, DOCUMENT, 1.0) && view.rotation == 15.0);
    assert!(ViewCommands::ResetRotation.apply(&mut view, DOCUMENT, 1.0) && view.rotation == 0.0);
}

#[trial]
fn each_view_tool_owes_the_canvas_its_checks() {
    ToolFixture::new("Hand", HandTool::new(NavigationOptions::default())).check();
    ToolFixture::new("Zoom", ZoomTool::new(NavigationOptions::default())).check();
    ToolFixture::new("Rotate View", RotateViewTool::default()).check();
}
