//! The corner widgets: placement, the hide rules, dragging, clicks, the keyboard and the access tree.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_draw::ui::corner::{CornerKind, CornerPoint, CornerRef, CornerTarget, CornerWidgets};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

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
    let corners =
        (0..n).map(|i| CornerPoint { vertex: p[i], prev: p[(i + n - 1) % n], next: p[(i + 1) % n], radius, kind: CornerKind::Round }).collect();
    CornerTarget { key, corners, individual }
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
    let (fixture, _) = show(host(vec![rect(1, (50.0, 50.0, 100.0, 60.0), 20.0, true)]));
    let (first, third) = (widget(&fixture, 1, 4).expect("corner 1"), widget(&fixture, 3, 4).expect("corner 3"));
    let apart = third.center() - first.center();
    assert!((apart - vec2(60.0, 20.0)).length() < 1.0, "{apart:?}");

    let (fixture, _) = show(host(vec![rect(1, (50.0, 50.0, 100.0, 60.0), 0.0, true)]));
    let (first, third) = (widget(&fixture, 1, 4).expect("corner 1"), widget(&fixture, 3, 4).expect("corner 3"));
    let (inward_first, inward_third) = (first.center(), third.center());
    assert!(inward_third.x - inward_first.x < 100.0 && inward_third.y - inward_first.y < 60.0, "both widgets sit inside their corners");
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
    let open = vec![CornerTarget { corners: vec![flat.corners[1]], ..flat }];
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
    let (mut fixture, state) = show(host(vec![rect(1, (50.0, 50.0, 100.0, 60.0), 0.0, true), triangle]));
    let grabbed = widget(&fixture, 1, 7).expect("the rectangle's first widget");
    fixture.drag(&grabbed, vec2(400.0, 400.0));
    assert!(radii(&state, 0).iter().all(|radius| (radius - 30.0).abs() < 1e-6), "{:?}", radii(&state, 0));
    let tight = radii(&state, 1);
    assert!(tight[0] > 0.0 && tight[0] < 30.0 && tight.iter().all(|radius| *radius == tight[0]), "{tight:?}");
}

/// A drag sets the radius of every target; Alt sets one corner of a rectangle and every corner of a polygon.
#[trial]
fn a_drag_sets_every_target_and_alt_sets_one_corner() {
    let (mut fixture, state) = show(host(pair()));
    let grabbed = widget(&fixture, 1, 8).expect("the rectangle's first widget");
    fixture.drag(&grabbed, vec2(10.0, 10.0));
    assert_eq!(radii(&state, 0), vec![10.0; 4]);
    assert_eq!(radii(&state, 1), vec![10.0; 4]);

    let (mut fixture, state) = show(host(pair()));
    let grabbed = widget(&fixture, 1, 8).expect("the rectangle's first widget");
    fixture.hold(Modifiers::ALT);
    fixture.drag(&grabbed, vec2(10.0, 10.0));
    assert_eq!(radii(&state, 0), vec![10.0, 0.0, 0.0, 0.0], "one corner of the rectangle");
    assert_eq!(radii(&state, 1), vec![10.0; 4], "a polygon shares one radius");
}

/// A drag back toward the corner never goes below 0.
#[trial]
fn the_dragged_radius_never_goes_below_zero() {
    let (mut fixture, state) = show(host(vec![rect(1, (50.0, 50.0, 200.0, 100.0), 10.0, true)]));
    let grabbed = widget(&fixture, 1, 4).expect("corner 1");
    fixture.drag(&grabbed, vec2(-200.0, -200.0));
    assert_eq!(radii(&state, 0), vec![0.0; 4]);
}

/// A click without movement changes nothing.
#[trial]
fn a_click_without_movement_changes_nothing() {
    let (mut fixture, state) = show(host(pair()));
    let before = state.borrow().targets.clone();
    let clicked = widget(&fixture, 1, 8).expect("corner 1");
    fixture.click(&clicked);
    assert_eq!(state.borrow().targets, before);
    assert!(state.borrow().cycled.is_empty() && state.borrow().activated.is_empty());
}

/// Alt-click with kinds on asks to cycle the corner's kind; a double click asks to open the corner's dialog.
#[trial]
fn alt_click_cycles_the_kind_and_a_double_click_activates() {
    let (mut fixture, state) = show(Host { kinds: true, ..host(pair()) });
    let clicked = widget(&fixture, 2, 8).expect("corner 2");
    fixture.hold(Modifiers::ALT);
    fixture.click(&clicked);
    assert_eq!(state.borrow().cycled, [CornerRef { target: 1, corner: 1 }]);
    fixture.hold(Modifiers::NONE);
    fixture.click(&clicked);
    fixture.click(&clicked);
    assert_eq!(state.borrow().activated.last(), Some(&CornerRef { target: 1, corner: 1 }));
}

/// A press between two widgets takes the nearer one; of equal distances the first target's.
#[trial]
fn a_press_takes_the_nearest_widget_topmost_first() {
    // Two squares whose adjoining corners put their widgets close together.
    let targets = vec![rect(1, (50.0, 50.0, 60.0, 60.0), 0.0, true), rect(2, (112.0, 50.0, 60.0, 60.0), 0.0, true)];
    let (mut fixture, state) = show(host(targets));
    let first = widget(&fixture, 2, 8).expect("the first square's top right corner");
    fixture.click(&first);
    fixture.click(&first);
    assert_eq!(state.borrow().activated.last().map(|corner| corner.target), Some(1));
}

/// The arrows step the picked widget's radius by 1 (Shift 10; up and right grow it), Alt that corner only; Page Down
/// picks the next widget.
#[trial]
fn the_keyboard_steps_the_radius() {
    let (mut fixture, state) = show(host(pair()));
    let first = widget(&fixture, 1, 8).expect("corner 1");
    fixture.click(&first);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(radii(&state, 0), vec![1.0; 4]);
    fixture.key(Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(radii(&state, 0), vec![11.0; 4]);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(radii(&state, 0), vec![10.0; 4]);
    fixture.key(Key::ArrowRight, Modifiers::ALT);
    assert_eq!(radii(&state, 0), vec![11.0, 10.0, 10.0, 10.0], "one corner");
    fixture.key(Key::PageDown, Modifiers::NONE);
    fixture.key(Key::ArrowRight, Modifiers::ALT);
    assert_eq!(radii(&state, 0), vec![11.0, 11.0, 10.0, 10.0], "the next widget");
    fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(state.borrow().activated.last(), Some(&CornerRef { target: 1, corner: 1 }));
}

/// One object per widget, named "Corner i of n" and valued with its radius.
#[trial]
fn the_access_tree_names_each_widget_and_gives_its_radius() {
    let (fixture, _) = show(host(vec![rect(1, (50.0, 50.0, 100.0, 60.0), 12.0, true)]));
    for n in 1..=4 {
        let node = widget(&fixture, n, 4).unwrap_or_else(|| panic!("Corner {n} of 4"));
        assert!(fixture.tree().value_of(node.node).is_some_and(|value| value.contains("radius 12")), "corner {n}");
    }
}
