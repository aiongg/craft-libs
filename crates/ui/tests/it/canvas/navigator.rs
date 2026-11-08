//! The Navigator's geometry, its settings and its zoom row.

use craft_behavior::kurbo::{Point, Rect as DocRect, Size};
use craft_behavior::{CanvasView, ZoomLadder};
use craft_ui::canvas::{
    Navigator, NavigatorOptions, NavigatorSettings, NavigatorZoomRow, ProxyColor, preview_rect, proxy_quad, view_center_for, zoom_to_document_rect,
};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DOC: Size = Size::new(2000.0, 1500.0);

/// The preview box and the inset round the fitted document, in points.
const FRAME: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(216.0, 166.0));
const INSET: f32 = 8.0;

/// The canvas of the views under test, in points.
const CANVAS: Rect = Rect::from_min_max(Pos2::new(50.0, 20.0), Pos2::new(850.0, 620.0));
const ZOOM_CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));

/// How far from the middle a click on the middle of the box may land, as a share of the document.
const CENTRE_SLACK: f64 = 0.05;

/// The zoom the row starts from.
const START: f64 = 1.0;

const CENTRE: Point = Point::new(1000.0, 750.0);

/// The slack of a point comparison after f32 maths, in points.
const TOLERANCE: f32 = 1e-3;

/// The proxy of the upright view in the 216 by 166 box: 800 by 600 points at zoom 1 are 80 by 60 preview points round
/// the preview point of the centre, (108, 83).
const UPRIGHT: [Pos2; 4] = [Pos2::new(68.0, 53.0), Pos2::new(148.0, 53.0), Pos2::new(148.0, 113.0), Pos2::new(68.0, 113.0)];

fn preview() -> Rect {
    preview_rect(FRAME, DOC, INSET)
}

fn view() -> CanvasView {
    CanvasView::new(CANVAS, CENTRE, 1.0)
}

fn near(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < TOLERANCE
}

#[trial]
fn the_proxy_follows_a_rotated_or_flipped_view() {
    assert_eq!(preview(), Rect::from_min_max(Pos2::new(8.0, 8.0), Pos2::new(208.0, 158.0)), "the document fits the box inside the inset");
    let upright = proxy_quad(&view(), DOC, preview());
    assert!(upright.iter().zip(UPRIGHT).all(|(a, b)| near(*a, b)), "{upright:?}");

    let wide = proxy_quad(&view().aspect(2.0), DOC, preview());
    assert!(near(wide[0], Pos2::new(88.0, 53.0)) && near(wide[1], Pos2::new(128.0, 53.0)), "a pixel aspect of 2 shows half the columns: {wide:?}");

    let turned = Rect::from_points(&proxy_quad(&view().rotation(90.0), DOC, preview()));
    assert!((turned.width() - 60.0).abs() < TOLERANCE && (turned.height() - 80.0).abs() < TOLERANCE, "a quarter turn swaps the extents: {turned:?}");

    let [a, b, c, d] = proxy_quad(&view().rotation(30.0), DOC, preview());
    assert!(((b - a).length() - 80.0).abs() < TOLERANCE && ((c - b).length() - 60.0).abs() < TOLERANCE, "the edges keep their length");
    assert!((b - a).dot(c - b).abs() < 1e-2 && near(a + (c - b), d), "the corners stay right angles");
    assert!((a.x - b.x).abs() > 1.0 && (a.y - b.y).abs() > 1.0, "a tilted view is not axis-aligned");

    let flipped = proxy_quad(&view().flip_x(true), DOC, preview());
    assert!(near(flipped[0], UPRIGHT[1]) && near(flipped[1], UPRIGHT[0]), "a mirrored view keeps the canvas's corner order: {flipped:?}");
}

#[trial]
fn a_click_in_the_preview_asks_for_the_document_point_under_it() {
    let middle = view_center_for(preview(), DOC, Pos2::new(108.0, 83.0));
    assert!((middle.x - CENTRE.x).abs() < 1e-2 && (middle.y - CENTRE.y).abs() < 1e-2, "{middle:?}");
    assert_eq!(
        view_center_for(preview(), DOC, Pos2::new(-40.0, 400.0)),
        Point::new(0.0, DOC.height),
        "a click off the picture stays on the document"
    );
}

fn zoom_view(zoom: f64) -> CanvasView {
    CanvasView::new(ZOOM_CANVAS, CENTRE, zoom)
}

#[trial]
fn zooming_to_a_rectangle_fills_the_canvas_with_it() {
    // 800 by 600 points over 50 by 37.5 units: 16 points per unit.
    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(100.0, 100.0, 150.0, 137.5), DOC, 1.0));
    assert!((view.zoom - 16.0).abs() < 1e-9 && view.center == Point::new(125.0, 118.75), "{view:?}");

    let mut view = zoom_view(1.0);
    zoom_to_document_rect(&mut view, DocRect::new(0.0, 0.0, 400.0, 300.0), DOC, 2.0);
    assert!((view.zoom - 2.0).abs() < 1e-9, "the stored zoom is the device zoom over the pixels per point: {view:?}");

    let mut view = zoom_view(1.0).aspect(2.0);
    zoom_to_document_rect(&mut view, DocRect::new(0.0, 0.0, 400.0, 300.0), DOC, 1.0);
    assert!((view.zoom - 1.0).abs() < 1e-9, "a pixel aspect of 2 doubles the columns across the canvas: {view:?}");

    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(-100.0, -100.0, 400.0, 300.0), DOC, 1.0));
    assert!((view.zoom - 2.0).abs() < 1e-9 && view.center == Point::new(200.0, 150.0), "a rectangle past the document is clipped: {view:?}");

    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(100.0, 100.0, 101.0, 150.0), DOC, 1.0));
    assert!((view.zoom - 2.0).abs() < 1e-9 && view.center == Point::new(100.5, 125.0), "a sliver steps one rung in: {view:?}");

    for rect in [DocRect::new(f64::NAN, 0.0, 100.0, 100.0), DocRect::new(3000.0, 0.0, 3100.0, 100.0), DocRect::new(0.0, 0.0, f64::INFINITY, 1.0)] {
        let mut view = zoom_view(1.0);
        assert!(!zoom_to_document_rect(&mut view, rect, DOC, 1.0), "{rect:?}");
        assert_eq!(view, zoom_view(1.0), "{rect:?} changes nothing");
    }
}

#[trial]
fn proxy_colours_resolve_and_the_settings_round_trip() {
    assert_eq!(ProxyColor::Default.rgb(), [255, 84, 84]);
    let named = &ProxyColor::ALL[1..];
    let shown: Vec<[u8; 3]> = named.iter().map(|color| color.rgb()).collect();
    assert!(shown.iter().enumerate().all(|(at, a)| shown[at + 1..].iter().all(|b| a != b)), "the named colours differ");
    assert_eq!(ProxyColor::Custom([1, 2, 3]).rgb(), [1, 2, 3]);

    let red = toml::Value::try_from(NavigatorSettings::new(NavigatorOptions::new(ProxyColor::Red))).expect("settings serialise");
    assert_eq!(red.get("proxyColor").and_then(toml::Value::as_str), Some("Red"), "the options are flattened into the section");
    let custom = NavigatorSettings::new(NavigatorOptions::new(ProxyColor::Custom([1, 2, 3])));
    let written = toml::Value::try_from(custom).expect("settings serialise");
    assert_eq!(written.try_into::<NavigatorSettings>().expect("settings read back"), custom);
    assert_eq!(toml::from_str::<NavigatorSettings>("other = 1").expect("unknown fields are ignored"), NavigatorSettings::default());
}

#[trial]
fn zoom_in_steps_the_ladder_about_the_centre_and_asks_for_the_ease() {
    let view = Rc::new(RefCell::new(zoom_view(START)));
    let eased = Rc::new(Cell::new(false));
    let (held, flag) = (Rc::clone(&view), Rc::clone(&eased));
    let mut fixture = Fixture::new(move |ui| {
        let mut view = held.borrow_mut();
        if NavigatorZoomRow::new(Bind::bound(&mut *view), DOC).show(ui).animate_zoom {
            flag.set(true);
        }
    });
    fixture.settle();
    let zoom_in = fixture.by_label("Zoom In").expect("the row has a Zoom In button");
    fixture.click(&zoom_in);
    fixture.settle();
    let after = *view.borrow();
    assert!(after.zoom > START && after.zoom <= ZoomLadder::MAX, "one rung in: {after:?}");
    assert_eq!(after.center, CENTRE, "the centre stays");
    assert!(eased.get(), "Animated Zoom is on by default, so the button asks for the ease");
}

#[trial]
fn a_press_on_the_preview_centres_the_view_on_the_point_pressed() {
    let view = Rc::new(RefCell::new(CanvasView::new(ZOOM_CANVAS, Point::new(0.0, 0.0), START)));
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let mut view = held.borrow_mut();
        Navigator::new("Navigator", Bind::bound(&mut *view), DOC).show(ui);
    });
    fixture.settle();
    let preview = fixture.by_label("Navigator").expect("the navigator has a node");
    fixture.click(&preview);
    fixture.settle();
    let centre = view.borrow().center;
    assert!(
        (centre.x - DOC.width / 2.0).abs() < DOC.width * CENTRE_SLACK && (centre.y - DOC.height / 2.0).abs() < DOC.height * CENTRE_SLACK,
        "the middle of the box is the middle of the document: {centre:?}"
    );
}
