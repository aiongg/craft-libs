use egui::accesskit::{self, Role};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect as ScreenRect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{CanvasScrollbars, ScrollbarNames, ScrollbarsResponse, Span, bar_rects, clamp_axis};
use crate::canvas::CanvasView;
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, UiTokens, install};

const MIN: f32 = 24.0;
const WIDTH: f32 = 12.0;

#[test]
fn a_fitting_document_scrolls_edge_to_edge_with_overscroll() {
    // 1000 unit document, 500 points at 0.4 zoom show 1250 units: it fits, centred.
    let s = Span::of(1000.0, 500.0, 500.0, 0.4).unwrap();
    let e = s.extent(true);
    assert_eq!(e, (-250.0, 1250.0));
    let (at, len) = s.thumb(e, 300.0, MIN);
    assert!((len - 250.0).abs() < 1e-3 && (at - 25.0).abs() < 1e-3, "a centred thumb with room: {at} {len}");
    assert!((s.doc_per_point(e, 300.0, MIN) - 5.0).abs() < 1e-3, "50 points of travel cover 250 units");
    // Overscroll off: nothing to scroll, the thumb fills the track.
    let e = s.extent(false);
    assert_eq!(e, (-125.0, 1125.0));
    assert_eq!(s.thumb(e, 300.0, MIN), (0.0, 300.0));
    assert_eq!(s.doc_per_point(e, 300.0, MIN), 0.0);
    // At 100 % only half is visible: the extent is the document either way, the thumb half the track.
    let s = Span::of(1000.0, 500.0, 500.0, 1.0).unwrap();
    for overscroll in [true, false] {
        let e = s.extent(overscroll);
        assert_eq!(e, (0.0, 1000.0));
        let (at, len) = s.thumb(e, 400.0, MIN);
        assert!((len - 200.0).abs() < 1e-3 && (at - 100.0).abs() < 1e-3, "{at} {len}");
    }
}

#[test]
fn the_thumb_reaches_both_ends_and_has_a_minimum() {
    let left = Span::of(10_000.0, 250.0, 500.0, 1.0).unwrap();
    let e = left.extent(true);
    assert_eq!(left.thumb(e, 300.0, MIN), (0.0, MIN));
    let right = Span::of(10_000.0, 9750.0, 500.0, 1.0).unwrap();
    let (at, len) = right.thumb(right.extent(true), 300.0, MIN);
    assert!((at + len - 300.0).abs() < 1e-3);
    // Dragging the thumb the whole travel scrolls the whole room.
    let k = left.doc_per_point(e, 300.0, MIN);
    assert!(((300.0 - MIN) * k - 9500.0).abs() < 1e-2, "{k}");
}

#[test]
fn an_overscrolled_view_extends_the_extent() {
    // Scrolled 300 units past the start.
    let s = Span::of(1000.0, -50.0, 500.0, 1.0).unwrap();
    let e = s.extent(true);
    assert_eq!(e, (-300.0, 1000.0));
    assert_eq!(s.thumb(e, 400.0, MIN).0, 0.0);
}

#[test]
fn overscroll_off_keeps_the_document_in_view() {
    assert_eq!(clamp_axis(-400.0, 1000.0, 500.0, 1.0), 250.0);
    assert_eq!(clamp_axis(4000.0, 1000.0, 500.0, 1.0), 750.0);
    assert_eq!(clamp_axis(600.0, 1000.0, 500.0, 1.0), 600.0);
    // Smaller than the view (2500 units of view for 1000 of document): it stays where it is while
    // it is inside the view, and is pushed back in at the edges.
    assert_eq!(clamp_axis(-90.0, 1000.0, 500.0, 0.2), -90.0);
    assert_eq!(clamp_axis(-400.0, 1000.0, 500.0, 0.2), -250.0, "the document's end on the view's");
    assert_eq!(clamp_axis(1400.0, 1000.0, 500.0, 0.2), 1250.0, "its start on the view's");
    assert_eq!(clamp_axis(500.0, 1000.0, 1000.0, 1.0), 500.0, "exactly the view's size: centred");
    let doc = Rect::new(0.0, 0.0, 1000.0, 800.0);
    let mut view = CanvasView::new(ScreenRect::from_min_size(pos2(30.0, 40.0), vec2(500.0, 400.0)), Point::new(-500.0, 20.0), 1.0);
    assert!(CanvasScrollbars::clamp(&mut view, doc));
    assert_eq!(view.center, Point::new(250.0, 200.0));
    assert!(!CanvasScrollbars::clamp(&mut view, doc), "stable: no repaint loop");
}

#[test]
fn the_clamp_follows_a_document_not_at_the_origin_and_a_turned_view() {
    let doc = Rect::new(100.0, 50.0, 1100.0, 850.0);
    let mut view = CanvasView::new(ScreenRect::from_min_size(Pos2::ZERO, vec2(500.0, 400.0)), Point::new(-500.0, 20.0), 1.0);
    assert!(CanvasScrollbars::clamp(&mut view, doc));
    assert_eq!(view.center, Point::new(350.0, 250.0), "the offset of the document's corner is kept");
    for rotation in [90.0, 180.0, 270.0, 30.0] {
        let mut view =
            CanvasView { rotation, ..CanvasView::new(ScreenRect::from_min_size(Pos2::ZERO, vec2(500.0, 400.0)), Point::new(-900.0, 3000.0), 1.0) };
        assert!(CanvasScrollbars::clamp(&mut view, doc), "{rotation}");
        let visible = view.visible();
        if rotation % 90.0 == 0.0 {
            let tolerance = 1e-6;
            assert!(
                visible.x0 >= doc.x0 - tolerance
                    && visible.x1 <= doc.x1 + tolerance
                    && visible.y0 >= doc.y0 - tolerance
                    && visible.y1 <= doc.y1 + tolerance,
                "{rotation}: {visible:?} inside {doc:?}"
            );
        }
        assert!(!CanvasScrollbars::clamp(&mut view, doc), "{rotation}: stable");
    }
}

#[test]
fn hostile_numbers_are_harmless() {
    for z in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(Span::of(1000.0, 10.0, 500.0, z).is_none());
        assert_eq!(clamp_axis(10.0, 1000.0, 500.0, z), 10.0);
    }
    assert!(Span::of(f32::NAN, 10.0, 500.0, 1.0).is_none());
    assert!(Span::of(1000.0, f32::INFINITY, 500.0, 1.0).is_none());
    assert_eq!(clamp_axis(f32::NAN, 1000.0, 500.0, 1.0), 500.0);
    let s = Span::of(1000.0, 500.0, 500.0, 1.0).unwrap();
    let (at, len) = s.thumb((0.0, 1000.0), 0.0, MIN);
    assert!(at.is_finite() && len.is_finite());
    assert_eq!(s.doc_per_point((0.0, 1000.0), 0.0, MIN), 0.0);
    let mut view = CanvasView::new(ScreenRect::from_min_size(Pos2::ZERO, vec2(500.0, 400.0)), Point::new(0.0, 0.0), 1.0);
    assert!(!CanvasScrollbars::clamp(&mut view, Rect::new(0.0, 0.0, 0.0, 0.0)));
    assert!(!CanvasScrollbars::clamp(&mut CanvasView { zoom: f64::NAN, ..view }, Rect::new(0.0, 0.0, 10.0, 10.0)));
}

#[test]
fn the_bars_share_the_corner() {
    let rect = ScreenRect::from_min_size(pos2(20.0, 20.0), vec2(400.0, 300.0));
    let (h, v) = bar_rects(rect, WIDTH, true, true);
    assert_eq!(h, Some(ScreenRect::from_min_max(pos2(20.0, 308.0), pos2(408.0, 320.0))));
    assert_eq!(v, Some(ScreenRect::from_min_max(pos2(408.0, 20.0), pos2(420.0, 308.0))));
    assert_eq!(bar_rects(rect, WIDTH, true, false).0.unwrap().right(), rect.right());
}

struct Host {
    view: CanvasView,
    doc: Rect,
    overscroll: bool,
    over: bool,
    changed: bool,
    tokens: Option<std::sync::Arc<UiTokens>>,
}

impl Host {
    fn new(zoom: f64, center: Point) -> Self {
        let rect = ScreenRect::from_min_size(pos2(20.0, 20.0), vec2(400.0, 300.0));
        Self {
            view: CanvasView::new(rect, center, zoom),
            doc: Rect::new(0.0, 0.0, 2000.0, 1500.0),
            overscroll: true,
            over: false,
            changed: false,
            tokens: None,
        }
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    Harness::<Host>::builder()
        .with_size(vec2(440.0, 340.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let names = ScrollbarNames::new("Horizontal scroll bar", "Vertical scroll bar");
                let ScrollbarsResponse { over, changed } =
                    CanvasScrollbars::new("bars", names).overscroll(host.overscroll).show(ui, &mut host.view, host.doc);
                host.tokens = Some(Tokens::of(ui));
                host.over = over;
                host.changed |= changed;
            },
            host,
        )
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

fn drag(h: &mut Harness<'_, Host>, from: Pos2, by: egui::Vec2) {
    press(h, from);
    for k in 1..=10 {
        h.event(Event::PointerMoved(from + by * (k as f32 / 10.0)));
        h.run();
    }
    release(h, from + by);
}

/// Centre of the horizontal thumb and its length, in screen points.
fn h_thumb(host: &Host) -> (Pos2, f32) {
    let view = host.view;
    let s = Span::of(host.doc.width() as f32, view.center.x as f32, view.rect.width(), view.zoom as f32).unwrap();
    let bar = bar_rects(view.rect, WIDTH, true, true).0.unwrap();
    let (at, len) = s.thumb(s.extent(host.overscroll), bar.width(), MIN);
    (pos2(bar.left() + at + len / 2.0, bar.center().y), len)
}

#[test]
fn dragging_the_thumb_scrolls_that_axis() {
    let mut h = harness(Host::new(1.0, Point::new(700.0, 750.0)));
    h.run_steps(2);
    let (p, _) = h_thumb(h.state());
    let c0 = h.state().view.center;
    drag(&mut h, p, vec2(40.0, 0.0));
    let c1 = h.state().view.center;
    // 2000 units of room over ~ 400 points of travel: about 5 units per point.
    assert!(c1.x - c0.x > 100.0, "{c0:?} -> {c1:?}");
    assert_eq!(c1.y, c0.y);
    assert!(h.state().changed);
    let (t, _) = h_thumb(h.state());
    assert!((t.x - (p.x + 40.0)).abs() < 3.0, "the thumb followed the pointer: {t:?} vs {p:?}");
}

#[test]
fn clicking_the_track_pages_by_a_screenful() {
    let mut h = harness(Host::new(1.0, Point::new(500.0, 750.0)));
    h.run_steps(2);
    let (p, _) = h_thumb(h.state());
    let rect = h.state().view.rect;
    let x0 = h.state().view.center.x;
    click(&mut h, pos2(rect.right() - 30.0, p.y));
    assert!((h.state().view.center.x - x0 - f64::from(rect.width())).abs() < 1.0, "{x0} -> {}", h.state().view.center.x);
    // Paging stops at the end of the document.
    for _ in 0..4 {
        click(&mut h, pos2(rect.right() - 30.0, p.y));
    }
    let end = 2000.0 - f64::from(rect.width()) / 2.0;
    assert!((h.state().view.center.x - end).abs() < 1.0, "{}", h.state().view.center.x);
    click(&mut h, pos2(rect.left() + 5.0, p.y));
    assert!((h.state().view.center.x - (end - f64::from(rect.width()))).abs() < 1.0);
}

#[test]
fn a_flipped_view_scrolls_the_other_way() {
    let mut host = Host::new(1.0, Point::new(1500.0, 750.0));
    host.view.flip_x = true;
    let mut h = harness(host);
    h.run_steps(2);
    let rect = h.state().view.rect;
    // The view is flipped, so the document's end is at the left: the thumb sits near the start of its bar.
    let bar = bar_rects(rect, WIDTH, true, true).0.unwrap();
    let x0 = h.state().view.center.x;
    click(&mut h, pos2(bar.right() - 20.0, bar.center().y));
    assert!(h.state().view.center.x < x0, "paging toward the bar's end moves the centre back: {x0} -> {}", h.state().view.center.x);
}

#[test]
fn the_pointer_over_a_bar_is_reported() {
    let mut h = harness(Host::new(1.0, Point::new(700.0, 750.0)));
    h.run_steps(2);
    assert!(!h.state().over);
    let rect = h.state().view.rect;
    h.event(Event::PointerMoved(pos2(rect.center().x, rect.bottom() - 6.0)));
    h.run_steps(2);
    assert!(h.state().over);
    h.event(Event::PointerMoved(rect.center()));
    h.run_steps(2);
    assert!(!h.state().over);
}

#[test]
fn a_document_that_fits_has_no_bar_on_an_empty_axis() {
    let mut host = Host::new(1.0, Point::new(50.0, 50.0));
    host.doc = Rect::new(0.0, 0.0, 0.0, 0.0);
    let mut h = harness(host);
    h.run_steps(2);
    assert!(!h.state().over && !h.state().changed);
}

#[test]
fn the_bars_are_scroll_bars_in_the_accesskit_tree_and_no_tab_stops() {
    let mut h = harness(Host::new(1.0, Point::new(700.0, 750.0)));
    h.run_steps(2);
    for (name, orientation) in
        [("Horizontal scroll bar", accesskit::Orientation::Horizontal), ("Vertical scroll bar", accesskit::Orientation::Vertical)]
    {
        let data = h.get_by_label(name).accesskit_node().data().clone();
        assert_eq!(data.role(), Role::ScrollBar);
        assert_eq!(data.orientation(), Some(orientation));
        assert!(data.numeric_value().is_some_and(|v| v > 0.0), "{name} reports its position");
        assert!(!data.supports_action(accesskit::Action::Focus), "{name} is not a Tab stop");
    }
}

#[test]
fn the_look_is_the_chrome_track_and_the_muted_hot_thumb() {
    let h = harness(Host::new(1.0, Point::new(700.0, 750.0)));
    let mut h = h;
    h.run();
    let tokens = h.state().tokens.clone().unwrap();
    let style = CanvasScrollbars::style(&tokens);
    assert_eq!((style.width, style.inset, style.thumb_min), (12.0, 3.0, 24.0));
    assert_eq!(style.track, tokens.color.surface.chrome.to_egui());
    assert_eq!(style.thumb, tokens.color.text.faint.to_egui());
    assert_eq!(style.thumb_hot, tokens.color.text.muted.to_egui());
}
