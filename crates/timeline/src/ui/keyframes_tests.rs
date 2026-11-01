//! Keyframe lanes and the navigator (egui_kittest).

use craft_ui::behavior::{Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Modifiers, Pos2, Rect, Sense, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::keyframes::{AnimationToggle, KeyframeId, KeyframeInterpolation, KeyframeLane, KeyframeMark, KeyframeNavigator};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView};

fn rate() -> FrameRate {
    FrameRate::new(24, 1)
}

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
    let mut harness = Harness::<S>::builder()
        .with_size([480.0, 120.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, state);
    harness.run();
    harness
}

struct Lane {
    keys: Vec<KeyframeMark>,
    selection: Selection<KeyframeId>,
    playhead: Tick,
    bounds: Option<TimeRange>,
    moves: Vec<EditEvent<Tick>>,
    changes: Vec<SelectionChange<KeyframeId>>,
    rect: Rect,
}

fn lane() -> Harness<'static, Lane> {
    let keys = [1.0, 2.0, 3.0]
        .into_iter()
        .enumerate()
        .map(|(at, s)| KeyframeMark { id: KeyframeId(at as u64 + 1), time: secs(s), interpolation: KeyframeInterpolation::Linear })
        .collect();
    harness(
        show_lane,
        Lane { keys, selection: Selection::new(), playhead: secs(2.5), bounds: None, moves: Vec::new(), changes: Vec::new(), rect: Rect::NOTHING },
    )
}

fn show_lane(ui: &mut Ui, app: &mut Lane) {
    let rect = Rect::from_min_size(ui.max_rect().min, vec2(400.0, 30.0));
    ui.allocate_rect(rect, Sense::hover());
    app.rect = rect;
    let view = TimeView::new(Tick::ZERO, 100.0);
    let mut lane = KeyframeLane::new("lane", &app.keys, &app.selection, &view, rate()).playhead(app.playhead);
    if let Some(bounds) = app.bounds {
        lane = lane.bounds(bounds);
    }
    let out = lane.show(ui, rect);
    if let Some(change) = out.selection {
        app.selection = change.selection.clone();
        app.changes.push(change);
    }
    app.moves.extend(out.moved);
}

/// The centre of the key at `seconds`.
fn key_at(rect: Rect, seconds: f64) -> Pos2 {
    pos2(rect.left() + seconds as f32 * 100.0, rect.center().y)
}

fn selected(harness: &Harness<'_, Lane>) -> Vec<u64> {
    let mut ids: Vec<u64> = harness.state().selection.items.iter().map(|id| id.0).collect();
    ids.sort_unstable();
    ids
}

// a click selects, Shift adds, a marquee selects the keys inside.
#[test]
fn click_and_marquee_select_keys() {
    let mut harness = lane();
    let rect = harness.state().rect;
    harness.click_at(key_at(rect, 1.0));
    assert_eq!(selected(&harness), [1]);
    harness.click_at_with(key_at(rect, 2.0), egui::PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1, 2], "Shift adds");
    harness.click_at_with(key_at(rect, 2.0), egui::PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1], "Shift on a selected key removes it");
    harness.click_at(pos2(key_at(rect, 1.5).x, rect.top() + 1.0));
    assert!(selected(&harness).is_empty(), "a click on the empty lane clears");

    harness.drag(pos2(key_at(rect, 1.5).x, rect.top() + 1.0), pos2(key_at(rect, 3.5).x, rect.bottom() - 1.0), Modifiers::NONE);
    assert_eq!(selected(&harness), [2, 3], "the marquee takes the keys inside");
    harness.click_at(key_at(rect, 1.0));
    harness.drag(pos2(key_at(rect, 1.5).x, rect.top() + 1.0), pos2(key_at(rect, 2.5).x, rect.bottom() - 1.0), Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1, 2], "Shift keeps the selection and adds");
}

fn commit(harness: &Harness<'_, Lane>) -> Option<Tick> {
    harness.state().moves.iter().rev().find_map(|event| match event {
        EditEvent::Commit { value, .. } => Some(*value),
        _ => None,
    })
}

// moved deltas are frame multiples; Shift snaps within 8 pt.
#[test]
fn dragging_keys_snaps_to_frames_and_with_shift_to_the_playhead() {
    let mut harness = lane();
    let rect = harness.state().rect;
    harness.drag(key_at(rect, 2.0), key_at(rect, 2.0) + vec2(37.0, 0.0), Modifiers::NONE);
    assert_eq!(commit(&harness), Some(rate().tick_of(9)), "0.37 s is 8.9 frames: the nearest frame");
    assert_eq!(selected(&harness), [2], "grabbing an unselected key selects it");

    let mut harness = lane();
    harness.drag(key_at(rect, 2.0), key_at(rect, 2.0) + vec2(48.0, 0.0), Modifiers::SHIFT);
    assert_eq!(commit(&harness), Some(secs(0.5)), "2 pt from the playhead at 2.5 s: snapped");

    let mut harness = lane();
    harness.drag(key_at(rect, 2.0), key_at(rect, 2.0) + vec2(38.0, 0.0), Modifiers::SHIFT);
    assert_eq!(commit(&harness), Some(rate().tick_of(9)), "12 pt away: only the frame grid");
}

// a drag past the clip end stops at it.
#[test]
fn keys_stay_inside_the_bounds() {
    let mut harness = lane();
    harness.state_mut().bounds = Some(TimeRange::new(Tick::ZERO, secs(3.5)));
    harness.run();
    let rect = harness.state().rect;
    harness.drag(key_at(rect, 3.0), key_at(rect, 3.0) + vec2(200.0, 0.0), Modifiers::NONE);
    assert_eq!(commit(&harness), Some(secs(0.5)));
}

#[derive(Default)]
struct Nav {
    previous: usize,
    next: usize,
    toggle: usize,
}

fn show_nav(ui: &mut Ui, app: &mut Nav) {
    let out = KeyframeNavigator::new("nav", true, false, false).show(ui);
    app.previous += usize::from(out.previous);
    app.next += usize::from(out.next);
    app.toggle += usize::from(out.toggle);
}

// previous, toggle and next presses; dimmed arrows without keys do nothing.
#[test]
fn navigator_reports_its_buttons() {
    let mut harness = harness(show_nav, Nav::default());
    harness.get_by_role_and_label(Role::Button, "Go to previous keyframe").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Go to next keyframe").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Add/Remove Keyframe").click();
    harness.run();
    let nav = harness.state();
    assert_eq!((nav.previous, nav.next, nav.toggle), (1, 0, 1));
}

struct Stopwatch {
    on: bool,
    events: Vec<EditEvent<bool>>,
}

fn show_stopwatch(ui: &mut Ui, app: &mut Stopwatch) {
    let out = AnimationToggle::new("watch", &app.on).show(ui);
    if let Some(EditEvent::Set { value, .. }) = &out.event {
        app.on = *value;
    }
    app.events.extend(out.event);
}

// a click on the stopwatch reports Set(!on).
#[test]
fn animation_toggle_reports_a_set() {
    let mut harness = harness(show_stopwatch, Stopwatch { on: false, events: Vec::new() });
    harness.get_by_role_and_label(Role::Button, "Toggle animation").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Toggle animation").click();
    harness.run();
    let values: Vec<bool> = harness.state().events.iter().filter_map(|event| event.value().copied()).collect();
    assert_eq!(values, [true, false]);
}
