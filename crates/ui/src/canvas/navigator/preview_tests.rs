//! The Navigator on screen (egui_kittest): pointer, wheel, keys, the zoom row and AccessKit.
#![allow(clippy::expect_used, reason = "tests")]

use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, Rect, Shape, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::{Point, Size};

use super::{Navigator, ProxyColor};
use crate::canvas::{CanvasView, NavigationOptions, ZoomLadder};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, UiTokens, install};

const DOC: Size = Size::new(2000.0, 1500.0);

/// Space above the navigator, so a pointer can be outside it.
const ABOVE: f32 = 30.0;

struct Host {
    view: CanvasView,
    shown: Option<CanvasView>,
    nav: NavigationOptions,
    settings: super::NavigatorSettings,
    changed: bool,
    animate: bool,
    frame: Rect,
    fitted: Rect,
    row: Rect,
    tokens: Option<std::sync::Arc<UiTokens>>,
}

impl Host {
    fn new(zoom: f64, center: Point) -> Self {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        Self {
            view: CanvasView::new(canvas, center, zoom),
            shown: None,
            nav: NavigationOptions { animated_zoom: false, ..NavigationOptions::default() },
            settings: super::NavigatorSettings::default(),
            changed: false,
            animate: false,
            frame: Rect::NOTHING,
            fitted: Rect::NOTHING,
            row: Rect::NOTHING,
            tokens: None,
        }
    }

    /// A point in the picture at `(x, y)` of its width and height.
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.fitted.min + vec2(self.fitted.width() * x, self.fitted.height() * y)
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(300.0, 280.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                ui.add_space(ABOVE);
                let numbers = LocaleNumbers::of(ui);
                let mut view = host.shown.unwrap_or(host.view);
                let out = Navigator::new("navigator", DOC, &numbers).navigation(&host.nav).settings(&mut host.settings).show(ui, &mut view);
                if host.shown.is_none() {
                    host.view = view;
                }
                host.changed |= out.view_changed;
                host.animate |= out.animate_zoom;
                host.frame = out.response.rect;
                host.fitted = out.preview_rect;
                host.row = out.row_rect;
                host.tokens = Some(Tokens::of(ui));
            },
            host,
        );
    harness.run_steps(2);
    harness
}

fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    press(h, at);
    release(h, at);
}

fn move_to(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    for k in 1..=8 {
        h.event(Event::PointerMoved(from + (to - from) * (k as f32 / 8.0)));
        h.run();
    }
}

fn near(a: Point, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 2.0 && (a.y - y).abs() < 2.0
}

fn focus_preview(h: &mut Harness<'_, Host>) {
    h.get_by_role_and_label(Role::Group, "Navigator").focus();
    h.run();
}

// A click on the preview sets the view centre to the point, clamped to the document.
#[test]
fn navigator_click_centres_the_view_on_the_point() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    let at = h.state().at(0.25, 0.5);
    click(&mut h, at);
    assert!(near(h.state().view.center, 500.0, 750.0), "{:?}", h.state().view.center);
    assert!(h.state().changed);
    assert!(!h.state().animate);
    // The margin round the picture is outside the document: the centre stops at its edge.
    let corner = h.state().frame.min + vec2(1.0, 1.0);
    click(&mut h, corner);
    assert!(near(h.state().view.center, 0.0, 0.0), "{:?}", h.state().view.center);
}

// A drag moves the centre each pass and reports `view_changed`; Esc puts the press view back.
#[test]
fn navigator_drag_follows_and_esc_restores() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    let (start, mid, end) = (h.state().at(0.5, 0.5), h.state().at(0.25, 0.5), h.state().at(0.75, 0.5));
    press(&mut h, start);
    h.state_mut().changed = false;
    move_to(&mut h, start, mid);
    assert!(h.state().changed, "a drag pass reports the change");
    assert!(near(h.state().view.center, 500.0, 750.0), "{:?}", h.state().view.center);
    h.key_press(Key::Escape);
    h.run();
    assert!(near(h.state().view.center, 1000.0, 750.0), "Esc restores the press view: {:?}", h.state().view.center);
    move_to(&mut h, mid, end);
    assert!(near(h.state().view.center, 1000.0, 750.0), "the cancelled drag no longer moves it");
    release(&mut h, end);
    assert!(near(h.state().view.center, 1000.0, 750.0));
}

/// The rectangles drawn with a stroke of `color`.
fn stroked(h: &Harness<'_, Host>, color: egui::Color32) -> Vec<Rect> {
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut shapes = Vec::new();
    h.output().shapes.iter().for_each(|clipped| flatten(&clipped.shape, &mut shapes));
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => Some(r.rect),
            _ => None,
        })
        .collect()
}

// The proxy is drawn from the view passed in (the animated one), size and centre as PhotoCraft's
// test.
#[test]
fn navigator_proxy_follows_the_animated_view() {
    let mut host = Host::new(1.0, Point::new(1000.0, 750.0));
    // The app's own view is elsewhere; the navigator is handed the animated one.
    let mut shown = host.view;
    shown.zoom = 2.0;
    shown.center = Point::new(500.0, 375.0);
    host.shown = Some(shown);
    let mut h = harness(host);
    let tokens = h.state().tokens.clone().expect("drawn");
    let red = ProxyColor::Default.color(&tokens);
    let fitted = h.state().fitted;
    let scale = fitted.width() / DOC.width as f32;
    // 800 × 600 points at zoom 2 are 400 × 300 units.
    let expected = Rect::from_center_size(fitted.min + vec2(500.0, 375.0) * scale, vec2(400.0, 300.0) * scale);
    let drawn = stroked(&h, red);
    assert!(drawn.iter().any(|r| (r.min - expected.min).length() < 0.5 && (r.max - expected.max).length() < 0.5), "{drawn:?} vs {expected:?}");
    // The settings' colour replaces the token.
    h.state_mut().settings.options.proxy_color = ProxyColor::Blue;
    h.run_steps(2);
    let blue = ProxyColor::Blue.color(&tokens);
    assert!(!stroked(&h, blue).is_empty() && stroked(&h, red).is_empty());
    assert_eq!(blue, tokens.component.navigator.proxy_colors[4].to_egui());
}

fn wheel(h: &mut Harness<'_, Host>, at: Pos2, lines: f32) {
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: MouseWheelSource::Unknown,
    });
    h.run_steps(2);
}

// With Animated Zoom off one notch over the preview is ×1.1 and leaves the centre; outside the
// preview nothing changes (PhotoCraft's `navigator_wheel_changes_zoom_only_when_over_preview`,
// panels.rs:336).
#[test]
fn navigator_wheel_zooms_only_over_the_preview_and_never_pans() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    wheel(&mut h, pos2(5.0, ABOVE / 2.0), 1.0);
    assert_eq!(h.state().view.zoom, 1.0, "outside the preview");
    let inside = h.state().at(0.1, 0.1);
    wheel(&mut h, inside, 1.0);
    assert!((h.state().view.zoom - 1.1).abs() < 1e-6, "{}", h.state().view.zoom);
    assert!(near(h.state().view.center, 1000.0, 750.0), "the zoom is about the view centre, not the pointer");
    assert!(h.state().changed);
    // Zoom with Scroll Wheel is off here; the navigator zooms anyway and does not pan.
    assert!(!h.state().nav.zoom_with_wheel);
}

// A notch past 12800 % changes nothing and settles the wheel.
#[test]
fn navigator_wheel_at_the_limit_settles() {
    let mut h = harness(Host::new(ZoomLadder::MAX, Point::new(1000.0, 750.0)));
    let inside = h.state().at(0.5, 0.5);
    wheel(&mut h, inside, 1.0);
    assert_eq!(h.state().view.zoom, ZoomLadder::MAX);
    assert!(!h.state().changed);
    // An eased notch (Animated Zoom on) also leaves the view alone and finishes within a pass.
    h.state_mut().nav.animated_zoom = true;
    wheel(&mut h, inside, 1.0);
    h.run_steps(5);
    assert_eq!(h.state().view.zoom, ZoomLadder::MAX);
    assert!(!h.state().changed);
}

// Arrows, Shift + arrows, Page Up and Down, `+`, `-` and Home on the focused preview.
#[test]
fn navigator_keys_move_the_view_and_zoom() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    focus_preview(&mut h);
    h.key_press(Key::ArrowRight);
    h.run();
    assert!(near(h.state().view.center, 1080.0, 750.0), "a tenth of the 800 point span: {:?}", h.state().view.center);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    h.run();
    assert!(near(h.state().view.center, 680.0, 750.0), "half a span: {:?}", h.state().view.center);
    h.key_press(Key::PageDown);
    h.run();
    assert!(near(h.state().view.center, 680.0, 1350.0), "a span down: {:?}", h.state().view.center);
    h.key_press(Key::ArrowDown);
    h.run();
    assert!(near(h.state().view.center, 680.0, 1410.0));
    h.key_press(Key::PageUp);
    h.run();
    assert!(near(h.state().view.center, 680.0, 810.0), "a span up: {:?}", h.state().view.center);
    h.key_press(Key::Home);
    h.run();
    assert!(near(h.state().view.center, 1000.0, 750.0), "Home centres the document");
    h.key_press(Key::Plus);
    h.run();
    assert!((h.state().view.zoom - 2.0).abs() < 1e-9, "+ steps the ladder in");
    h.key_press(Key::Minus);
    h.run();
    assert!((h.state().view.zoom - 1.0).abs() < 1e-9, "- steps it out");
    assert!(h.state().changed);
    // The arrows stop at the document's edge.
    for _ in 0..40 {
        h.key_press(Key::PageDown);
        h.run();
    }
    assert!(h.state().view.center.y <= 1500.0);
}

// Zoom Out, Zoom In and the field's Up and Down step the ladder; the buttons set `animate_zoom`.
#[test]
fn navigator_zoom_row_steps_the_ladder() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    assert!(h.state().row.min.y >= h.state().frame.max.y, "the row sits under the preview");
    h.get_by_label("Zoom In").click();
    h.run();
    assert!((h.state().view.zoom - 2.0).abs() < 1e-9);
    assert!(h.state().changed && !h.state().animate, "Animated Zoom is off");
    h.state_mut().nav.animated_zoom = true;
    h.get_by_label("Zoom Out").click();
    h.run();
    assert!((h.state().view.zoom - 1.0).abs() < 1e-9);
    assert!(h.state().animate, "a button with Animated Zoom on asks for the animation");
    h.state_mut().animate = false;
    h.get_by_role_and_label(Role::SpinButton, "Zoom").focus();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    assert!((h.state().view.zoom - 2.0).abs() < 1e-9, "Up steps the ladder: {}", h.state().view.zoom);
    assert!(!h.state().animate, "the field never animates");
    assert!(near(h.state().view.center, 1000.0, 750.0), "the zoom is about the centre");
}

// A zoom change from outside during a field run ends the run without putting the start back;
// only Esc does.
#[test]
fn navigator_zoom_row_restores_the_start_only_on_escape() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    h.get_by_role_and_label(Role::SpinButton, "Zoom").focus();
    h.run();
    h.event(Event::Text("2".into()));
    h.run();
    assert!((h.state().view.zoom - 0.02).abs() < 1e-9, "the typed value applies: {}", h.state().view.zoom);
    h.state_mut().view.zoom = 0.5;
    h.run_steps(3);
    assert!((h.state().view.zoom - 0.5).abs() < 1e-9, "the outside change stays: {}", h.state().view.zoom);

    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    h.get_by_role_and_label(Role::SpinButton, "Zoom").focus();
    h.run();
    h.event(Event::Text("2".into()));
    h.run();
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!((h.state().view.zoom - 1.0).abs() < 1e-9, "Esc restores the start: {}", h.state().view.zoom);
}

// The two AccessKit scroll bar nodes carry the percent value, and Increment moves the view.
#[test]
fn navigator_preview_scroll_bars_expose_the_view() {
    let mut h = harness(Host::new(1.0, Point::new(500.0, 750.0)));
    let value = |h: &Harness<'_, Host>, name: &str| h.get_by_role_and_label(Role::ScrollBar, name).accesskit_node().numeric_value();
    assert_eq!(value(&h, "Visible area horizontal position"), Some(25.0));
    assert_eq!(value(&h, "Visible area vertical position"), Some(50.0));
    let id = h.get_by_role_and_label(Role::ScrollBar, "Visible area horizontal position").accesskit_node().id();
    h.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_tree: TreeId::ROOT,
        target_node: NodeId((u128::from(id) >> 64) as u64),
        data: None,
    }));
    h.run();
    assert!(near(h.state().view.center, 580.0, 750.0), "the arrow-key step: {:?}", h.state().view.center);
    assert!(h.state().changed);
    assert!((value(&h, "Visible area horizontal position").expect("a value") - 29.0).abs() < 1e-3);
    assert!(h.query_by_role_and_label(Role::Image, "Document preview").is_some());
}

// With Cmd/Ctrl held at the press, the drag draws a zoom rectangle and the release zooms to it
// without moving the view while it is dragged.
#[test]
fn navigator_command_drag_zooms_to_the_dragged_rectangle() {
    let mut h = harness(Host::new(1.0, Point::new(1000.0, 750.0)));
    let (from, to) = (h.state().at(0.25, 0.25), h.state().at(0.5, 0.5));
    h.event(Event::ModifiersChanged(Modifiers::COMMAND));
    press(&mut h, from);
    move_to(&mut h, from, to);
    assert!(near(h.state().view.center, 1000.0, 750.0), "the view stays put during the drag");
    release(&mut h, to);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    // 500 × 375 units in an 800 × 600 canvas: 1.6 points per unit, centred on the rectangle.
    assert!((h.state().view.zoom - 1.6).abs() < 1e-2, "{}", h.state().view.zoom);
    assert!(near(h.state().view.center, 750.0, 562.5), "{:?}", h.state().view.center);
    assert!(h.state().changed && !h.state().animate);
}
