//! The canvas view, its zoom rules, the view history and the ruler ticks.

use craft_behavior::canvas::zoom;
use craft_behavior::{CanvasView, RulerUnits, ViewHistory, ViewPlace, ZoomLadder};
use emath::{Rect as ScreenRect, pos2};
use kurbo::{Point, Rect, Size};

fn view() -> CanvasView {
    CanvasView::new(ScreenRect::from_min_max(pos2(40.0, 30.0), pos2(840.0, 630.0)), Point::new(500.0, 400.0), 0.75)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_document_point_survives_the_trip_to_the_screen_and_back() {
    let views = [
        view(),
        view().rotation(33.0),
        view().rotation(-120.0).flip_x(true),
        view().aspect(2.0).flip_x(true),
        view().rotation(90.0).aspect(0.5),
        view().rotation(217.5).flip_x(true).aspect(1.5),
    ];
    for view in views {
        for p in [Point::new(0.0, 0.0), Point::new(512.0, 300.0), Point::new(-40.0, 1200.0)] {
            let back = view.to_doc(view.to_screen(p));
            assert!(near(back.x, p.x) && near(back.y, p.y), "{p:?} came back as {back:?} through {view:?}");
        }
    }
}

#[test]
fn quarter_turns_map_a_rect_to_an_axis_aligned_quad() {
    let doc = Rect::new(10.0, 20.0, 310.0, 220.0);
    for turns in 0..8 {
        for flip in [false, true] {
            let quad = view().rotation(90.0 * f64::from(turns)).flip_x(flip).aspect(1.25).quad(doc);
            for i in 0..4 {
                let (a, b) = (quad[i], quad[(i + 1) % 4]);
                assert!(a.x == b.x || a.y == b.y, "edge {i} of turn {turns} (flip {flip}) is slanted: {a:?} to {b:?}");
            }
        }
    }
    let turned = view().rotation(30.0).quad(doc);
    assert!(turned[0].x != turned[1].x && turned[0].y != turned[1].y, "a 30 degree turn slants the edges");
}

#[test]
fn only_an_upright_square_view_is_a_layer_transform() {
    let upright = view().layer_transform().expect("an upright view is a layer transform");
    let p = Point::new(123.0, 456.0);
    let on_screen = view().to_screen(p);
    let in_layer = upright.to_layer(pos2(p.x as f32, p.y as f32));
    assert!((on_screen - in_layer).length() < 0.05, "the transform maps like the view: {on_screen:?} against {in_layer:?}");
    assert!(view().rotation(90.0).layer_transform().is_none());
    assert!(view().flip_x(true).layer_transform().is_none());
    assert!(view().aspect(2.0).layer_transform().is_none());
}

#[test]
fn zooming_about_a_point_keeps_it_where_it_is() {
    for base in [view(), view().rotation(40.0), view().flip_x(true).aspect(1.5)] {
        let anchor = pos2(300.0, 200.0);
        let before = base.to_doc(anchor);
        let mut zoomed = base;
        zoom::about(&mut zoomed, anchor, 3.0, false, 2.0);
        assert!(near(zoom::device(&zoomed, 2.0), 3.0), "the zoom is the one asked for");
        let after = zoomed.to_screen(before);
        assert!((after - anchor).length() < 1e-3, "the anchored point stays at {anchor:?}, not {after:?}");
    }
}

#[test]
fn the_ladder_steps_rung_by_rung_and_stops_at_its_ends() {
    let size = Size::new(1000.0, 800.0);
    let lowest = ZoomLadder::min(size);
    let rungs: Vec<f64> = ZoomLadder::STEPS.iter().copied().filter(|rung| *rung >= lowest).collect();
    let mut zoom = lowest;
    let mut seen = Vec::new();
    for _ in 0..ZoomLadder::STEPS.len() {
        let next = ZoomLadder::step(zoom, 1, size);
        if next == zoom {
            break;
        }
        assert!(next > zoom, "zooming in rises: {zoom} to {next}");
        seen.push(next);
        zoom = next;
    }
    assert_eq!(zoom, ZoomLadder::MAX, "zooming in reaches the top rung");
    assert_eq!(ZoomLadder::step(ZoomLadder::MAX, 1, size), ZoomLadder::MAX, "the top rung holds");
    assert_eq!(seen, rungs.iter().copied().filter(|rung| *rung > lowest).collect::<Vec<_>>(), "no rung is skipped going up");
    for pair in rungs.windows(2).rev() {
        assert_eq!(ZoomLadder::step(pair[1], -1, size), pair[0], "zooming out from {} lands on {}", pair[1], pair[0]);
    }
    assert_eq!(ZoomLadder::step(lowest, -1, size), lowest, "the lowest zoom holds");
    assert_eq!(ZoomLadder::step(8.8, 1, size), 12.0, "880 % goes in to 1200 %");
    assert_eq!(ZoomLadder::step(8.8, -1, size), 8.0, "880 % goes out to 800 %");
}

#[test]
fn going_back_skips_places_that_no_longer_exist() {
    let at = |key: u32| ViewPlace::new(key, Point::new(f64::from(key), 0.0), 1.0);
    let mut history = ViewHistory::default();
    for key in 1..=4 {
        history.visit(at(key));
    }
    let exists = |key: &u32| *key != 3 && *key != 2;
    assert!(history.can_go(-1, exists), "page 1 still exists");
    assert!(!history.can_go(1, exists), "nothing lies ahead");
    let place = history.go(-1, exists).expect("Go Back finds page 1");
    assert_eq!(place.key, 1, "pages 3 and 2 are skipped");
    assert!(history.can_go(1, |key| *key == 4), "Go Forward still reaches page 4");
    assert!(!history.can_go(1, |_| false), "no place exists");
}

#[test]
fn shift_snap_lands_on_a_tick_of_the_ruler() {
    let units = RulerUnits::pixels();
    let (zoom, step_min) = (2.0, 60.0);
    let (step, subdivisions) = units.step(zoom, step_min);
    let minor = step / f64::from(subdivisions);
    for position in [103.4, 17.77, -250.3, 0.2] {
        let snapped = units.snap_to_tick(position, 10.0, zoom, step_min);
        let ticks = (snapped - 10.0) / minor;
        assert!(near(ticks, ticks.round()), "{position} snaps to {snapped}, which is not a tick of {minor}");
        assert!((snapped - position).abs() <= minor / 2.0 + 1e-9, "{position} moves to the nearest tick, not {snapped}");
    }
}

#[test]
fn fit_up_to_magnifies_a_small_document_where_fit_stops_at_one_hundred_percent() {
    let size = Size::new(100.0, 100.0);
    let mut view = CanvasView::new(ScreenRect::from_min_size(pos2(0.0, 0.0), emath::vec2(1000.0, 1000.0)), Point::ZERO, 1.0);
    zoom::fit(&mut view, size, 1.0);
    assert!(near(zoom::device(&view, 1.0), 1.0), "fit stops at 100 %");
    zoom::fit_up_to(&mut view, size, 1.0, 4.0);
    assert!(near(zoom::device(&view, 1.0), 4.0), "a 100 px image in a 1000 pt canvas fits at the 400 % bound");
    assert_eq!(view.center, Point::new(50.0, 50.0));
}
