//! The corner widgets: placement, the hide rules, dragging, clicks, the keyboard and the access tree.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_draw::ui::corner::{CornerKind, CornerPoint, CornerRef, CornerTarget, CornerWidgets};
use craft_ui::canvas::{Canvas, NUDGE, NUDGE_FAR};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The rectangle most trials draw: its origin, size and the radius that puts a widget at the arc's centre.
const BOX: (f64, f64, f64, f64) = (50.0, 50.0, 100.0, 60.0);
const BOX_WIDTH: f64 = BOX.2;
const BOX_HEIGHT: f64 = BOX.3;
const ARC: f64 = 20.0;
/// The tightest radius of the rectangle: half its short side.
const FILLET_CAP: f64 = BOX_HEIGHT / 2.0;
/// The pull of a drag and the radius it sets.
const PULL: f64 = 10.0;
/// Slack for rounding to screen points.
const SLACK: f32 = 1.0;

/// What the host of the widgets keeps and what they reported.
struct Host {
    targets: Vec<CornerTarget>,
    kinds: bool,
    hide_above: f64,
    zoom: f64,
    cycled: Vec<CornerRef>,
    activated: Vec<CornerRef>,
}

fn polygon(key: u64, points: &[(f64, f64)], radius: f64, individual: bool) -> CornerTarget {
    let p: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let n = p.len();
    let corners = (0..n).map(|i| CornerPoint::new(p[i], p[(i + n - 1) % n], p[(i + 1) % n], radius, CornerKind::Round)).collect();
    CornerTarget::new(key, corners, individual)
}

/// A rectangle from (`x`, `y`) of `w` by `h`.
fn rect(key: u64, (x, y, w, h): (f64, f64, f64, f64), radius: f64, individual: bool) -> CornerTarget {
    polygon(key, &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], radius, individual)
}

fn host(targets: Vec<CornerTarget>) -> Host {
    Host { targets, kinds: false, hide_above: CornerWidgets::HIDE_ABOVE, zoom: 1.0, cycled: Vec::new(), activated: Vec::new() }
}

/// The widgets over a canvas showing the document's (0, 0) to (400, 300) at the host's zoom.
fn show(host: Host) -> (Fixture, Rc<RefCell<Host>>) {
    let state = Rc::new(RefCell::new(host));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let Host { targets, kinds, hide_above, zoom, cycled, activated } = &mut *host;
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(200.0, 150.0), *zoom);
        let mut control = Some(CornerWidgets::new("Corner radius", Bind::bound(targets)).kinds(*kinds).hide_above(*hide_above));
        let canvas = Canvas::new("Document").view(view).show(ui, |ui, body| control.take().map(|control| control.show_at(ui, body.rect)));
        if let Some(Some(output)) = canvas.inner {
            cycled.extend(output.cycle_kind);
            activated.extend(output.activate);
        }
    });
    fixture.settle();
    (fixture, state)
}

/// The widget of the `n`th shown corner of `of`, counted from 1.
fn widget(fixture: &Fixture, n: usize, of: usize) -> Option<Target> {
    fixture.by_label(&format!("Corner {n} of {of}"))
}

fn radii(host: &Rc<RefCell<Host>>, target: usize) -> Vec<f64> {
    host.borrow().targets[target].corners.iter().map(|corner| corner.radius).collect()
}

fn pair() -> Vec<CornerTarget> {
    vec![rect(1, (50.0, 50.0, 200.0, 100.0), 0.0, true), rect(2, (260.0, 200.0, 80.0, 80.0), 0.0, false)]
}

/// At a radius over the inset a widget sits at the arc's centre: the first and third corner of a 100 by 60 rectangle
/// at radius 20 are 60 and 20 apart. A radius of 0 puts the widget inside the corner along its diagonal.
#[trial]
fn a_widget_sits_at_the_arc_centre_and_inside_a_sharp_corner() {
    let (fixture, _) = show(host(vec![rect(1, BOX, ARC, true)]));
    let (first, third) = (widget(&fixture, 1, 4).expect("corner 1"), widget(&fixture, 3, 4).expect("corner 3"));
    let apart = third.center() - first.center();
    let between_centres = vec2((BOX_WIDTH - ARC - ARC) as f32, (BOX_HEIGHT - ARC - ARC) as f32);
    assert!((apart - between_centres).length() < SLACK, "{apart:?}");

    let (fixture, _) = show(host(vec![rect(1, BOX, 0.0, true)]));
    let (first, third) = (widget(&fixture, 1, 4).expect("corner 1"), widget(&fixture, 3, 4).expect("corner 3"));
    let (inward_first, inward_third) = (first.center(), third.center());
    assert!(
        inward_third.x - inward_first.x < BOX_WIDTH as f32 && inward_third.y - inward_first.y < BOX_HEIGHT as f32,
        "both widgets sit inside their corners"
    );
}

/// A side under the least length hides the corner's widget and zooming in shows it; an inner angle over the limit
/// hides it.
#[trial]
fn widgets_hide_on_short_sides_and_wide_angles() {
    let short = vec![rect(1, (50.0, 50.0, 100.0, 20.0), 0.0, true)];
    let (fixture, _) = show(host(short.clone()));
    assert!(widget(&fixture, 1, 4).is_none() && fixture.by_label("Corner 1 of 1").is_none());
    let (fixture, _) = show(Host { zoom: 2.0, ..host(short) });
    assert!(widget(&fixture, 1, 4).is_some());

    // A vertex of 170 degrees shows under a limit of 177 and hides under 100.
    let flat = polygon(3, &[(-100.0, 0.0), (0.0, 0.0), (98.48, 17.36)], 0.0, false);
    let open = vec![CornerTarget::new(flat.key, vec![flat.corners[1]], flat.individual)];
    let (fixture, _) = show(Host { hide_above: 177.0, ..host(open.clone()) });
    assert!(fixture.by_label("Corner 1 of 1").is_some());
    let (fixture, _) = show(Host { hide_above: 100.0, ..host(open) });
    assert!(fixture.by_label("Corner 1 of 1").is_none());
}

/// A drag far past the limit gives each target its own maximum: half the short side of a rectangle, the tightest
/// fillet of a triangle.
#[trial]
fn a_drag_caps_the_radius_at_the_fillet_limit() {
    let triangle = polygon(2, &[(260.0, 200.0), (360.0, 200.0), (310.0, 210.0)], 0.0, false);
    let (mut fixture, state) = show(host(vec![rect(1, BOX, 0.0, true), triangle]));
    let grabbed = widget(&fixture, 1, 7).expect("the rectangle's first widget");
    fixture.drag(&grabbed, vec2(400.0, 400.0));
    assert!(radii(&state, 0).iter().all(|radius| (radius - FILLET_CAP).abs() < 1e-6), "{:?}", radii(&state, 0));
    let tight = radii(&state, 1);
    assert!(tight[0] > 0.0 && tight[0] < FILLET_CAP && tight.iter().all(|radius| *radius == tight[0]), "{tight:?}");
}

/// A drag sets the radius of every target; Alt sets one corner of a rectangle and every corner of a polygon.
#[trial]
fn a_drag_sets_every_target_and_alt_sets_one_corner() {
    let (mut fixture, state) = show(host(pair()));
    let grabbed = widget(&fixture, 1, 8).expect("the rectangle's first widget");
    fixture.drag(&grabbed, vec2(PULL as f32, PULL as f32));
    assert_eq!(radii(&state, 0), vec![PULL; 4]);
    assert_eq!(radii(&state, 1), vec![PULL; 4]);

    let (mut fixture, state) = show(host(pair()));
    let grabbed = widget(&fixture, 1, 8).expect("the rectangle's first widget");
    fixture.hold(Modifiers::ALT);
    fixture.drag(&grabbed, vec2(PULL as f32, PULL as f32));
    assert_eq!(radii(&state, 0), vec![PULL, 0.0, 0.0, 0.0], "one corner of the rectangle");
    assert_eq!(radii(&state, 1), vec![PULL; 4], "a polygon shares one radius");
}

