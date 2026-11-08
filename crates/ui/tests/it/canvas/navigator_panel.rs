//! `NavigatorPanel`: the preview box and the zoom row bound to one view.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Size};
use craft_ui::canvas::NavigationOptions;
use craft_ui::contract::Bind;
use craft_ui::patterns::canvas::NavigatorPanel;
use craft_ui::testing::access::focused_key;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const DOC: Size = Size::new(2000.0, 1500.0);
const CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));

/// How far the trials drag in the preview box, in points.
const DRAG: Vec2 = Vec2::new(20.0, 12.0);

/// A Navigator panel over `DOC` bound to the returned view, with Animated Zoom off.
fn panel(enabled: bool) -> (Fixture, Rc<RefCell<CanvasView>>) {
    let view = Rc::new(RefCell::new(CanvasView::new(CANVAS, Point::new(1000.0, 750.0), 0.25)));
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let mut navigation = NavigationOptions::default();
        navigation.animated_zoom = false;
        let mut view = held.borrow_mut();
        NavigatorPanel::new("Navigator", Bind::bound(&mut *view), DOC).navigation(navigation).enabled(enabled).show(ui);
    });
    fixture.settle();
    (fixture, view)
}

#[trial]
fn a_zoom_row_click_and_a_proxy_drag_both_reach_the_one_bound_view() {
    let (mut fixture, view) = panel(true);
    let before = *view.borrow();
    let zoom_in = fixture.by_label("Zoom In").expect("the zoom row has Zoom In");
    fixture.click(&zoom_in).settle();
    assert!(view.borrow().zoom > before.zoom, "Zoom In zooms the bound view: {:?}", view.borrow());

    let zoomed = *view.borrow();
    let preview = fixture.by_label("Navigator").expect("the preview box has a node");
    fixture.drag(&preview, DRAG).settle();
    assert_ne!(view.borrow().center, zoomed.center, "a drag in the preview moves the bound view");
    assert_eq!(view.borrow().zoom, zoomed.zoom, "and keeps its zoom");
}

#[trial]
fn tab_goes_from_the_preview_to_the_zoom_field() {
    let (mut fixture, view) = panel(true);
    let preview = fixture.by_label("Navigator").expect("the preview box has a node");
    fixture.key(Key::Tab, Modifiers::NONE).settle();
    assert_eq!(focused_key(fixture.tree()), preview.node, "the first Tab focuses the preview box");
    let before = *view.borrow();
    fixture.key(Key::Tab, Modifiers::NONE).settle();
    // Up in the preview would pan; in the zoom field it steps the zoom.
    fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    assert!(view.borrow().zoom > before.zoom, "the next Tab focuses the zoom field, whose Up zooms in: {:?}", view.borrow());
    assert_eq!(view.borrow().center, before.center, "and nothing pans");
}

#[trial]
fn a_disabled_panel_takes_no_click_and_no_drag() {
    let (mut fixture, view) = panel(false);
    let before = *view.borrow();
    let zoom_in = fixture.by_label("Zoom In").expect("the zoom row has Zoom In");
    fixture.click(&zoom_in).settle();
    let preview = fixture.by_label("Navigator").expect("the preview box has a node");
    fixture.drag(&preview, DRAG).settle();
    assert_eq!(*view.borrow(), before, "neither part edits the view");
}
