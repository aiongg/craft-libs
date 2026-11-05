//! Marker strip and dialog (egui_kittest).

use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Key, KeyboardShortcut, Modifiers, PointerButton, Pos2, Rect, Ui, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::markers::{MarkerDialog, MarkerDialogAction, MarkerDialogResult, MarkerEdit, MarkerStrip, MarkerStripLabels};
use crate::clips::SnapTargets;
use crate::markers::{Marker, MarkerColor, MarkerId};
use crate::rates::FrameRate;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

fn marker(id: u64, at: f64) -> Marker {
    Marker { id: MarkerId(id), start: secs(at), duration: Tick::ZERO, name: format!("Cue {id}"), comment: String::new(), color: MarkerColor(9) }
}

fn harness<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S) -> Harness<'static, S> {
    sized(show, state, [480.0, 160.0])
}

fn sized<S>(show: impl FnMut(&mut Ui, &mut S) + 'static, state: S, size: [f32; 2]) -> Harness<'static, S> {
    let mut harness = Harness::<S>::builder()
        .with_size(size)
        .with_step_dt(0.02)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, state);
    harness.run();
    harness
}

struct Strip {
    markers: Vec<Marker>,
    snap: SnapTargets,
    edits: Vec<MarkerEdit>,
}

const STRIP: Rect = Rect { min: pos2(10.0, 10.0), max: pos2(470.0, 40.0) };

fn show_strip(ui: &mut Ui, app: &mut Strip) {
    let view = TimeView::new(Tick::ZERO, 100.0);
    let out = MarkerStrip::new("strip", &app.markers, &view).snap(Some(&app.snap)).show(ui, STRIP);
    app.edits.extend(out.edits);
}

fn strip(markers: Vec<Marker>) -> Harness<'static, Strip> {
    let mut snap = SnapTargets::new();
    snap.push(secs(4.0));
    harness(show_strip, Strip { markers, snap, edits: Vec::new() })
}

fn at(seconds: f64) -> Pos2 {
    pos2(STRIP.left() + seconds as f32 * 100.0, STRIP.top() + 5.0)
}

// a drag reports Move on release with the snapped time.
#[test]
fn dragging_a_marker_moves_it() {
    let mut harness = strip(vec![marker(1, 2.0)]);
    harness.drag(at(2.0), at(4.05), Modifiers::NONE);
    let edits = &harness.state().edits;
    assert!(matches!(edits.first(), Some(MarkerEdit::Move { to: EditEvent::Begin { .. }, .. })), "{edits:?}");
    let Some(MarkerEdit::Move { id, to: EditEvent::Commit { value, changed: true, .. } }) = edits.last() else { panic!("{edits:?}") };
    assert_eq!((*id, *value), (MarkerId(1), secs(4.0)), "the time snaps to the target within reach");
}

// Alt-drag reports SetDuration.
#[test]
fn alt_drag_sets_the_duration() {
    let mut harness = strip(vec![marker(1, 1.0)]);
    harness.drag(at(1.0), at(3.0), Modifiers::ALT);
    let edits = &harness.state().edits;
    assert!(edits.iter().all(|edit| matches!(edit, MarkerEdit::SetDuration { .. })), "{edits:?}");
    let Some(MarkerEdit::SetDuration { duration: EditEvent::Commit { value, .. }, .. }) = edits.last() else { panic!("{edits:?}") };
    assert_eq!(*value, secs(2.0));
}

// Cmd-click reports Delete, a double-click Open.
#[test]
fn command_click_deletes_and_double_click_opens() {
    let mut harness = strip(vec![marker(1, 2.0)]);
    harness.click_at_with(at(2.0), PointerButton::Primary, Modifiers::COMMAND);
    assert_eq!(harness.state().edits, [MarkerEdit::Delete(MarkerId(1))]);

    let mut harness = strip(vec![marker(1, 2.0)]);
    for _ in 0..2 {
        harness.event(egui::Event::PointerMoved(at(2.0)));
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton { pos: at(2.0), button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
        harness.run();
    }
    assert!(harness.state().edits.contains(&MarkerEdit::Open(MarkerId(1))), "{:?}", harness.state().edits);
}

// Left and Right pick markers in time order, Enter opens the picked one, Delete and Backspace
// delete it; Shift+Delete is another chord.
#[test]
fn keys_pick_open_and_delete_markers() {
    let mut harness = strip(vec![marker(2, 3.0), marker(1, 2.0)]);
    let title = MarkerStripLabels::default().title;
    harness.get_by_role_and_label(Role::ListBox, &title).focus();
    harness.run();
    let press = |harness: &mut Harness<'static, Strip>, modifiers, key| harness.press(KeyboardShortcut::new(modifiers, key));

    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    press(&mut harness, Modifiers::SHIFT, Key::Delete);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(
        harness.state().edits,
        [MarkerEdit::Open(MarkerId(1)), MarkerEdit::Delete(MarkerId(2)), MarkerEdit::Delete(MarkerId(1))],
        "the first pick is the earliest marker"
    );
}

struct Dialog {
    dialog: MarkerDialog,
    stack: DialogStack,
    result: Option<DialogResult<MarkerDialogResult>>,
}

fn show_dialog(ui: &mut Ui, app: &mut Dialog) {
    app.stack.push(app.dialog.id(), None);
    if let Some(result) = app.dialog.show(ui.ctx(), &app.stack) {
        app.result = Some(result);
    }
}

fn dialog() -> Harness<'static, Dialog> {
    let format = TimeFormat::timecode(FrameRate::new(24, 1));
    let dialog = MarkerDialog::new(&marker(7, 2.0), format);
    sized(show_dialog, Dialog { dialog, stack: DialogStack::default(), result: None }, [640.0, 720.0])
}

// OK returns the edited draft, Delete answers Delete, Next keeps the edits.
#[test]
fn dialog_returns_the_draft_or_delete() {
    let mut harness = dialog();
    harness.get_by_label("OK").click();
    harness.run();
    let Some(DialogResult::Ok(answer)) = &harness.state().result else { panic!("OK answers") };
    assert_eq!(answer.marker, MarkerId(7));
    assert!(
        matches!(&answer.action, MarkerDialogAction::Save(draft) if draft.name == "Cue 7" && draft.start == secs(2.0) && draft.color == MarkerColor(9))
    );

    let mut harness = dialog();
    harness.get_by_label("Delete").click();
    harness.run();
    assert!(matches!(&harness.state().result, Some(DialogResult::Ok(answer)) if answer.action == MarkerDialogAction::Delete));

    let mut harness = dialog();
    harness.get_by_role_and_label(Role::RadioButton, "3").click();
    harness.run();
    harness.get_by_label("Next").click();
    harness.run();
    let Some(DialogResult::Ok(answer)) = &harness.state().result else { panic!("Next answers") };
    assert!(matches!(&answer.action, MarkerDialogAction::SaveAndNext(draft) if draft.color == MarkerColor(2)), "{answer:?}");
}
