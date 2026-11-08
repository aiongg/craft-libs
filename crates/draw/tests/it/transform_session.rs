//! `TransformSession`: results, gestures, undo, presets, the warp phase, the mode menu and the settings.

use craft_behavior::geometry::quad::{BoxAllow, BoxState, ScaleRule, TransformMode};
use craft_behavior::kurbo::{Point, Rect, Vec2};
use craft_behavior::{Anchor9, CancelReason, EditEvent, GestureId, Mixed};
use craft_draw::ui::transform::{
    Interpolation, TransformChoice, TransformEdit, TransformMenuWords, TransformPhase, TransformPreset, TransformRequest, TransformRow,
    TransformSession, TransformSettings, TransformWarp,
};
use craft_ui::contract::Entry;
use craft_ui::runtime::env::{UnitKind, WordSet};
use craft_ui::testing::trial;
use std::sync::Arc;

fn begin<T>(id: GestureId) -> EditEvent<T> {
    EditEvent::begin(id, Mixed::Mixed, Arc::from(""), None)
}

fn update<T>(id: GestureId, value: T) -> EditEvent<T> {
    EditEvent::update(id, value)
}

fn commit<T>(id: GestureId, value: T) -> EditEvent<T> {
    EditEvent::commit(id, Mixed::Mixed, value)
}

fn set<T>(value: T) -> EditEvent<T> {
    EditEvent::set(GestureId::new(1), Mixed::Mixed, value, Arc::from(""), None)
}

fn cancel<T>(id: GestureId) -> EditEvent<T> {
    EditEvent::cancel(id, CancelReason::Escape, None)
}

fn settings(change: impl FnOnce(&mut TransformSettings)) -> TransformSettings {
    let mut settings = TransformSettings::default();
    change(&mut settings);
    settings
}

#[derive(Clone, Debug, PartialEq)]
struct Warp(bool);

impl TransformWarp for Warp {
    fn is_identity(&self) -> bool {
        self.0
    }
}

const FRAME: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);

fn session() -> TransformSession<Warp> {
    TransformSession::new(FRAME)
}

fn moved(by: Vec2) -> BoxState {
    BoxState::from_rect(FRAME).moved(by)
}

/// A whole drag to `state`, settled.
fn drag(s: &mut TransformSession<Warp>, state: BoxState, now: f64) {
    let id = GestureId::new(1);
    s.apply(TransformEdit::Box(begin(id)), now);
    s.apply(TransformEdit::Box(update(id, state)), now);
    s.apply(TransformEdit::Box(commit(id, state)), now);
    s.settle(now, false);
}

fn nudge(s: &mut TransformSession<Warp>, state: BoxState, now: f64) {
    s.apply(TransformEdit::Box(set(state)), now);
    s.settle(now, false);
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-6
}

// A new session has no result and commits as CloseUntouched; a moved box gives Apply with the
// frame, the box and the interpolation; a mode or interpolation change alone stays untouched.
#[trial]
fn an_untouched_session_has_no_result_and_a_moved_one_has() {
    let mut s = session();
    assert!(s.result().is_none());
    s.apply(TransformEdit::Mode(TransformMode::Rotate), 0.0);
    s.apply(TransformEdit::Interpolation(Interpolation::Bilinear), 0.0);
    assert!(s.is_untouched());
    assert_eq!(s.commit(), TransformRequest::CloseUntouched);

    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    let TransformRequest::Apply(result) = s.commit() else { panic!("expected Apply") };
    assert_eq!((result.rect, result.state, result.interpolation, result.warp), (FRAME, moved(Vec2::new(10.0, 0.0)), Interpolation::Bilinear, None));
}

// Box Begin, Update, then Cancel restores the box taken at Begin and requests nothing; discard
// with no drag running requests Discard.
#[trial]
fn esc_during_a_drag_restores_the_box_and_a_second_esc_discards() {
    let mut s = session();
    let id = GestureId::new(1);
    assert_eq!(s.apply(TransformEdit::Box(begin(id)), 0.0), None);
    s.apply(TransformEdit::Box(update(id, moved(Vec2::new(5.0, 5.0)))), 0.0);
    assert!(s.dragging());
    assert_eq!(s.apply(TransformEdit::Box(cancel(id)), 0.0), None);
    assert!(!s.dragging());
    assert_eq!(*s.state(), BoxState::from_rect(FRAME));
    assert_eq!(s.discard(), TransformRequest::Discard);
}

// Discard during a running gesture puts back the box the gesture began on.
#[trial]
fn discard_ends_a_running_gesture_on_the_box_it_began_on() {
    let mut s = session();
    let id = GestureId::new(1);
    s.apply(TransformEdit::Box(begin(id)), 0.0);
    s.apply(TransformEdit::Box(update(id, moved(Vec2::new(5.0, 5.0)))), 0.0);
    assert_eq!(s.discard(), TransformRequest::Discard);
    assert!(!s.dragging());
    assert_eq!(*s.state(), BoxState::from_rect(FRAME));
}

// In the Warp phase the warp holds the box: after a move and a reset to an identity warp, a commit
// closes untouched; a warp that changes something applies with the warp.
#[trial]
fn a_commit_in_the_warp_phase_decides_from_the_warp_alone() {
    let mut s = session();
    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    s.apply(TransformEdit::Warp(Warp(false)), 1.0);
    s.apply(TransformEdit::Warp(Warp(true)), 2.0);
    assert!(s.is_untouched());
    assert_eq!(s.commit(), TransformRequest::CloseUntouched);

    s.apply(TransformEdit::Warp(Warp(false)), 3.0);
    let TransformRequest::Apply(result) = s.commit() else { panic!("expected Apply") };
    assert_eq!(result.warp, Some(Warp(false)));
}

// Mode and interpolation are not undo steps: changing one records nothing, and Undo leaves them.
#[trial]
fn mode_and_interpolation_stay_outside_the_undo_steps() {
    let mut s = session();
    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    s.apply(TransformEdit::Mode(TransformMode::Skew), 1.0);
    s.apply(TransformEdit::Interpolation(Interpolation::Bilinear), 1.0);
    s.settle(1.0, false);
    assert!(s.undo());
    assert!(!s.can_undo());
    assert_eq!((s.current_mode(), s.interpolation()), (TransformMode::Skew, Interpolation::Bilinear));
    assert_eq!(*s.state(), BoxState::from_rect(FRAME));
}

// A Fields typing run (Begin, Updates, Commit) sets the box and the V skew from each report, which
// the fields built from the box at the run's start (gaps 112 to 114), and is one undo step.
#[trial]
fn a_field_typing_run_builds_every_report_from_the_box_at_its_start() {
    let mut s = session();
    let id = GestureId::new(1);
    let row = |dx: f64, skew_v: f64| TransformRow { state: moved(Vec2::new(dx, 0.0)), skew_v };
    s.apply(TransformEdit::Fields(begin(id)), 0.0);
    s.apply(TransformEdit::Fields(update(id, row(3.0, 5.0))), 0.0);
    s.settle(0.0, false);
    assert!(!s.can_undo());
    s.apply(TransformEdit::Fields(commit(id, row(4.0, 6.0))), 0.0);
    s.settle(0.0, false);
    assert_eq!((*s.state(), s.skew_v()), (moved(Vec2::new(4.0, 0.0)), 6.0));
    assert!(s.undo());
    assert_eq!((*s.state(), s.skew_v()), (BoxState::from_rect(FRAME), 0.0));
    assert!(!s.can_undo());
}

// Rotate 90° Clockwise and Flip Horizontal turn and mirror the box about the reference point and
// request nothing; in the Warp phase a preset changes nothing.
#[trial]
fn presets_turn_the_box_about_the_reference_point_without_applying() {
    let mut s = session();
    assert_eq!(s.apply(TransformEdit::Preset(TransformPreset::Rotate90Clockwise), 0.0), None);
    let pivot = s.pivot();
    // The top left corner (0, 0) goes to the top right of the turned box, clockwise on screen.
    assert!(near(s.quad()[0], Point::new(pivot.x + 50.0, pivot.y - 100.0)));
    s.apply(TransformEdit::Preset(TransformPreset::Rotate90CounterClockwise), 0.0);
    s.apply(TransformEdit::Preset(TransformPreset::FlipHorizontal), 0.0);
    assert!(near(s.quad()[0], Point::new(200.0, 0.0)));
    assert!(near(s.pivot(), Point::new(100.0, 50.0)));

    let mut warped = session();
    warped.apply(TransformEdit::Warp(Warp(false)), 0.0);
    warped.apply(TransformEdit::Preset(TransformPreset::Rotate180), 0.0);
    assert_eq!(*warped.state(), BoxState::from_rect(FRAME));
}

// WarpRequested answers NeedWarp with the current frame and box; Warp(w) enters the Warp phase;
// one undo goes back to the Box phase with the box as it was.
#[trial]
fn entering_warp_asks_for_a_warp_and_carries_the_box() {
    let mut s = session();
    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    let request = s.apply(TransformEdit::WarpRequested, 1.0);
    let Some(TransformRequest::NeedWarp { rect, state }) = request else { panic!("expected NeedWarp") };
    assert_eq!((rect, state), (FRAME, moved(Vec2::new(10.0, 0.0))));
    assert_eq!(s.phase(), TransformPhase::Box);
    s.apply(TransformEdit::Warp(Warp(false)), 1.0);
    s.settle(1.0, false);
    assert_eq!(s.phase(), TransformPhase::Warp);
    assert!(s.undo());
    assert_eq!((s.phase(), *s.state()), (TransformPhase::Box, moved(Vec2::new(10.0, 0.0))));
}

// LeaveWarp drops the warp and returns to the Box phase with the box unchanged.
#[trial]
fn leaving_warp_drops_the_warp_and_keeps_the_box() {
    let mut s = session();
    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    s.apply(TransformEdit::Warp(Warp(false)), 1.0);
    assert_eq!(s.apply(TransformEdit::LeaveWarp, 1.0), None);
    assert_eq!((s.phase(), s.warp()), (TransformPhase::Box, None));
    assert_eq!(*s.state(), moved(Vec2::new(10.0, 0.0)));
}

// A move, a pivot move, a warp: undo restores the box, the pivot and the warp of each step
// together; redo goes forward; a new edit after undo drops the redo.
#[trial]
fn undo_and_redo_restore_the_box_pivot_and_warp_together() {
    let mut s = session();
    let first = moved(Vec2::new(10.0, 0.0));
    drag(&mut s, first, 0.0);
    s.apply(TransformEdit::Pivot(Anchor9::TopLeft), 1.0);
    s.settle(1.0, false);
    let pivoted = *s.state();
    assert_eq!(pivoted.pivot, Point::new(10.0, 0.0));
    s.apply(TransformEdit::Warp(Warp(false)), 2.0);
    s.settle(2.0, false);

    assert!(s.undo());
    assert_eq!((*s.state(), s.warp()), (pivoted, None));
    assert!(s.undo());
    assert_eq!(*s.state(), first);
    assert!(s.redo());
    assert_eq!(*s.state(), pivoted);
    assert!(s.redo());
    assert_eq!(s.warp(), Some(&Warp(false)));
    assert!(!s.redo());

    assert!(s.undo());
    drag(&mut s, moved(Vec2::new(0.0, 7.0)), 3.0);
    assert!(!s.can_redo());
}

// The menu lists Free Transform, Scale, Rotate, Skew, Distort, Perspective, Warp, separator, the
// three rotations, separator, the two flips; the current mode is checked; Distort and Perspective
// are disabled when the content refuses distort; every pick applies as its edit.
#[trial]
fn the_mode_menu_lists_photoshops_order_and_the_picks_apply() {
    let labels = <TransformMenuWords as WordSet>::english();
    let s = session().mode(TransformMode::Rotate).allow(BoxAllow::new(true, true, false));
    let menu = s.menu(&labels);
    let shape: Vec<Option<&str>> = menu.iter().map(|e| (!e.is_separator()).then(|| e.label().as_str())).collect();
    assert_eq!(
        shape,
        [
            Some("Free Transform"),
            Some("Scale"),
            Some("Rotate"),
            Some("Skew"),
            Some("Distort"),
            Some("Perspective"),
            Some("Warp"),
            None,
            Some("Rotate 180°"),
            Some("Rotate 90° Clockwise"),
            Some("Rotate 90° Counter Clockwise"),
            None,
            Some("Flip Horizontal"),
            Some("Flip Vertical"),
        ]
    );
    let item = |label: &str| menu.iter().find(|entry| !entry.is_separator() && entry.label().as_str() == label).unwrap();
    assert!(item("Rotate").is_checked());
    assert!(!item("Free Transform").is_checked());
    assert!(!item("Distort").is_enabled() && !item("Perspective").is_enabled());
    assert!(item("Skew").is_enabled());
    assert_eq!(item("Scale").value(), Some(&TransformChoice::Mode(TransformMode::Free)));
    assert_eq!(item("Warp").value(), Some(&TransformChoice::Warp));
    assert_eq!(item("Flip Vertical").value(), Some(&TransformChoice::Preset(TransformPreset::FlipVertical)));

    // Scale counts as Free Transform, and the Warp phase disables the presets.
    let free = session().menu(&labels);
    let checked = |entries: &[Entry<TransformChoice>], label: &str| {
        entries.iter().find(|entry| !entry.is_separator() && entry.label().as_str() == label).unwrap().is_checked()
    };
    assert!(checked(&free, "Free Transform"));
    let mut warped = session();
    warped.apply(TransformEdit::Warp(Warp(false)), 0.0);
    let menu = warped.menu(&labels);
    assert!(menu.iter().filter(|entry| matches!(entry.value(), Some(TransformChoice::Preset(_)))).all(|entry| !entry.is_enabled()));
    assert!(checked(&menu, "Warp"));
}

// ReferencePoint(false) keeps the pivot where it is: a later preset and the box rules still turn
// and scale about it.
#[trial]
fn a_hidden_reference_point_stays_where_it_is_and_still_pivots_scaling() {
    let mut s = session();
    s.apply(TransformEdit::Pivot(Anchor9::TopLeft), 0.0);
    s.apply(TransformEdit::ReferencePoint(true), 0.0);
    s.apply(TransformEdit::ReferencePoint(false), 0.0);
    assert!(!s.reference_point());
    assert_eq!(s.pivot(), Point::new(0.0, 0.0));
    s.apply(TransformEdit::Preset(TransformPreset::Rotate180), 0.0);
    assert!(near(s.pivot(), Point::new(0.0, 0.0)));
    assert!(near(s.quad()[0], Point::new(0.0, 0.0)));
    assert!(near(s.quad()[2], Point::new(-200.0, -100.0)));
    // The view toggle is not an undo step.
    s.settle(0.0, false);
    s.apply(TransformEdit::ReferencePoint(true), 1.0);
    s.settle(1.0, false);
    assert!(s.undo() && !s.can_undo());
    assert!(s.reference_point());
}

// `box_rules` with `legacy` gives ScaleRule::Free, without it Proportional; the aspect link and
// the session's mode and allow carry over; the rotate step is 15.
#[trial]
fn settings_choose_the_legacy_scale_rule_and_the_aspect_link() {
    let allow = BoxAllow::new(true, false, true);
    let s = session().mode(TransformMode::Skew).allow(allow);
    let modern = s.box_rules(&settings(|s| s.aspect_linked = true));
    assert_eq!(
        (modern.scale, modern.aspect_linked, modern.mode, modern.allow, modern.rotate_step),
        (ScaleRule::Proportional, true, TransformMode::Skew, allow, 15.0)
    );
    let legacy = s.box_rules(&settings(|s| s.legacy = true));
    assert_eq!((legacy.scale, legacy.aspect_linked), (ScaleRule::Free, false));
}

// Two nudges within the pause share one step; a drag between them breaks the run; with no pause
// every nudge is its own step.
#[trial]
fn nudges_within_the_pause_fold_into_one_step() {
    let step = |dx: f64| moved(Vec2::new(dx, 0.0));
    let steps = |s: &mut TransformSession<Warp>| {
        let mut n = 0;
        while s.undo() {
            n += 1;
        }
        n
    };

    let mut folded = session().nudge_pause(Some(1.0));
    nudge(&mut folded, step(1.0), 0.0);
    nudge(&mut folded, step(2.0), 0.5);
    folded.settle(5.0, false);
    assert_eq!(steps(&mut folded), 1);

    let mut broken = session().nudge_pause(Some(1.0));
    nudge(&mut broken, step(1.0), 0.0);
    drag(&mut broken, step(30.0), 0.2);
    nudge(&mut broken, step(31.0), 0.4);
    broken.settle(5.0, false);
    assert_eq!(steps(&mut broken), 3);

    let mut plain = session();
    nudge(&mut plain, step(1.0), 0.0);
    nudge(&mut plain, step(2.0), 0.1);
    assert_eq!(steps(&mut plain), 2);
}

// Between a Box Begin and its Commit, undo and redo return false and change nothing.
#[trial]
fn undo_is_refused_while_a_drag_runs() {
    let mut s = session();
    drag(&mut s, moved(Vec2::new(10.0, 0.0)), 0.0);
    assert!(s.undo() && s.redo());
    let id = GestureId::new(1);
    s.apply(TransformEdit::Box(begin(id)), 1.0);
    s.apply(TransformEdit::Box(update(id, moved(Vec2::new(40.0, 0.0)))), 1.0);
    assert!(!s.undo() && !s.redo());
    assert_eq!(*s.state(), moved(Vec2::new(40.0, 0.0)));
    s.apply(TransformEdit::Box(commit(id, moved(Vec2::new(40.0, 0.0)))), 1.0);
    s.settle(1.0, false);
    assert!(s.undo());
    assert_eq!(*s.state(), moved(Vec2::new(10.0, 0.0)));
}

// `homography` maps the frame's corners onto the box's corners and its centre into the box; a box
// with three corners on a line gives None.
#[trial]
fn the_homography_maps_the_frame_onto_the_box() {
    let mut s = TransformSession::<()>::new(Rect::new(10.0, 20.0, 210.0, 120.0));
    let target =
        BoxState::new([Point::new(0.0, 0.0), Point::new(300.0, 10.0), Point::new(280.0, 200.0), Point::new(-20.0, 150.0)], Point::new(100.0, 80.0));
    s.apply(TransformEdit::Box(set(target)), 0.0);
    let h = s.homography().expect("a box");
    let corners = [Point::new(10.0, 20.0), Point::new(210.0, 20.0), Point::new(210.0, 120.0), Point::new(10.0, 120.0)];
    for (corner, expected) in corners.into_iter().zip(target.quad) {
        assert!(near(h.apply(corner), expected), "{corner:?}");
    }
    assert!(target.contains(h.apply(Point::new(110.0, 70.0))));

    let line = BoxState::new([Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(20.0, 0.0), Point::new(30.0, 0.0)], Point::ZERO);
    s.apply(TransformEdit::Box(set(line)), 0.0);
    assert!(s.homography().is_none());
}

// Preferences saved before the unit kinds moved name the W and H unit as `Pixels`, `Percent`, `Inches` and so on.
#[trial]
fn legacy_size_unit_spellings_load() {
    let loaded = |text: &str| serde_json::from_str::<TransformSettings>(&format!(r#"{{"sizeUnit":{text}}}"#)).map(|settings| settings.size_unit);
    for (legacy, kind) in [
        ("Pixels", UnitKind::Pixels),
        ("Percent", UnitKind::Percent),
        ("Inches", UnitKind::Inches),
        ("Centimetres", UnitKind::Centimetres),
        ("Millimetres", UnitKind::Millimetres),
        ("Points", UnitKind::Points),
        ("Picas", UnitKind::Picas),
    ] {
        assert_eq!(loaded(&format!("\"{legacy}\"")).ok(), Some(Some(kind)), "{legacy}");
        let saved = serde_json::to_string(&settings(|settings| settings.size_unit = Some(kind))).expect("settings serialize");
        let again: TransformSettings = serde_json::from_str(&saved).expect("saved settings load");
        assert_eq!(again.size_unit, Some(kind), "{kind:?} round-trips");
    }
    assert_eq!(loaded("null").ok(), Some(None));
    assert!(loaded("\"Furlongs\"").is_err(), "an unknown unit is refused");
}
