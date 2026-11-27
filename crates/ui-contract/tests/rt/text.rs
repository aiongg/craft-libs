//! Count trials of intents, commands, text input and the IME engine on craft-ui-rt: a typed
//! character, an IME step, a dialog's Submit, the cancel rule and a command shortcut run only what
//! they must. Relations only, no times.

use crate::skeleton::IconButton;
use crate::text_field::TextField;
use craft_behavior::{CancelRule, IconId};
use craft_keymap::{Binding, Chord, CommandDef, Keymap, Scope, Scopes};
use craft_ui_contract::contract::Bind;
use craft_ui_contract::contract::overlay::Overlay;
use craft_ui_contract::contract::rt::{CommandBoard, MemoView, Ui, WindowEnv, show_dialog, window_root_with};
use craft_ui_contract::runtime::commands::CommandState;
use craft_ui_contract::runtime::focus::FocusPolicy;
use craft_ui_platform_headless::Input;
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{ImeRequest, Key, Mods, Rect, State, StepReport, Vec2};
use craft_ui_test_kit::rt::Rig;
use std::cell::Cell;
use std::rc::Rc;

/// Past the theme's hover fade, which paint reads from the clock.
const FADE_END: f64 = 0.25;

/// What the window shows besides button A and the field in a view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layer {
    None,
    /// A dialog whose field takes the focus; Enter submits.
    Dialog,
    /// A palette whose first Escape clears its field and whose second closes it.
    Palette,
}

struct App {
    text: String,
    query: String,
    layer: Layer,
    a: Rect,
    field: Rect,
    view_runs: u32,
    submits: u32,
    /// Runs of the view that reads the command's state, and of the one that does not.
    reader_runs: u32,
    other_runs: u32,
    checked: Option<bool>,
    board: Option<CommandBoard>,
}

impl App {
    fn new(layer: Layer) -> App {
        App {
            text: String::new(),
            query: String::new(),
            layer,
            a: Rect::NOTHING,
            field: Rect::NOTHING,
            view_runs: 0,
            submits: 0,
            reader_runs: 0,
            other_runs: 0,
            checked: None,
            board: None,
        }
    }
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    app.a = IconButton::new(IconId::new("icon"), "A").show(ui).rect;
    let (text, view_runs) = (&mut app.text, &mut app.view_runs);
    app.field = MemoView::new(()).show(ui, |ui, _| {
        *view_runs += 1;
        TextField::new(Bind::bound(text)).show(ui).response.rect
    });
    if app.board.is_some() {
        let (reader_runs, other_runs) = (&mut app.reader_runs, &mut app.other_runs);
        app.checked = MemoView::new(1u8).show(ui, |ui, _| {
            *reader_runs += 1;
            ui.command_state("view.grid").is_checked()
        });
        MemoView::new(2u8).show(ui, |_, _| *other_runs += 1);
    }
    match app.layer {
        Layer::None => {}
        Layer::Dialog => {
            let (query, submits) = (&mut app.query, &mut app.submits);
            let shown = show_dialog(ui, 1, &Overlay::modal(), |ui| {
                TextField::new(Bind::bound(query)).show(ui);
                if ui.take_submit() {
                    *submits += 1;
                }
            });
            if shown.is_err() {
                app.layer = Layer::None;
            }
        }
        Layer::Palette => {
            let mut overlay = Overlay::modal();
            overlay.focus = FocusPolicy::DIALOG.cancel(CancelRule::ClearThenClose);
            let query = &mut app.query;
            let shown = show_dialog(ui, 2, &overlay, |ui| {
                if ui.take_clear() {
                    query.clear();
                }
                TextField::new(Bind::bound(query)).show(ui);
                ui.clearable(!query.is_empty());
            });
            if shown.is_err() {
                app.layer = Layer::None;
            }
        }
    }
}

fn step(rig: &mut Rig<RecordingRenderer>, app: &mut App) -> StepReport {
    let host = WindowEnv { commands: app.board.clone(), ..WindowEnv::default() };
    rig.step_with(&mut |_, cx| window_root_with(cx, host.clone(), |ui| app_ui(ui, app)))
}

fn settle(rig: &mut Rig<RecordingRenderer>, app: &mut App) {
    for _ in 0..6 {
        rig.driver.platform.advance(FADE_END);
        if !step(rig, app).ran {
            return;
        }
    }
    panic!("the rig did not settle");
}

fn rig(app: &mut App) -> Rig<RecordingRenderer> {
    let mut rig = Rig::inline(Vec2::new(400.0, 600.0), 1.0, RecordingRenderer::default());
    step(&mut rig, app);
    settle(&mut rig, app);
    rig
}

/// Injects `inputs` and runs one frame.
fn send(rig: &mut Rig<RecordingRenderer>, app: &mut App, inputs: Vec<Input>) -> StepReport {
    let main = rig.main;
    for input in inputs {
        rig.inject(main, input);
    }
    step(rig, app)
}

/// Presses and releases `key`; the report of the press's frame.
fn press(rig: &mut Rig<RecordingRenderer>, app: &mut App, key: Key, mods: Mods) -> StepReport {
    let [down, up] = Input::key_press(key, mods);
    let report = send(rig, app, vec![down]);
    send(rig, app, vec![up]);
    report
}

/// Types `c` as a platform does: its key press with its text; the report of the press's frame.
fn type_char(rig: &mut Rig<RecordingRenderer>, app: &mut App, key: Key, c: &str) -> StepReport {
    let [down, up] = Input::key_press(key, Mods::NONE);
    let report = send(rig, app, vec![down, Input::Text(c.to_string())]);
    send(rig, app, vec![up]);
    report
}

fn click(rig: &mut Rig<RecordingRenderer>, app: &mut App, at: Rect) -> (StepReport, StepReport) {
    send(rig, app, vec![Input::mouse_move(at.center())]);
    let down = send(rig, app, vec![Input::mouse_down(at.center())]);
    let up = send(rig, app, vec![Input::mouse_up(at.center())]);
    (down, up)
}

fn focused(rig: &Rig<RecordingRenderer>) -> Option<Rect> {
    let rt = &rig.driver.runtime;
    rt.focused(rig.main).and_then(|n| rt.window_rect(n))
}

fn preedit(text: &str) -> Input {
    Input::ImePreedit { text: text.to_string(), cursor: Some(text.len()..text.len()) }
}

#[test]
fn typing_into_a_focused_field_runs_only_the_field_s_path_per_character() {
    let mut app = App::new(Layer::None);
    let mut rig = rig(&mut app);
    let field = app.field;
    click(&mut rig, &mut app, field);
    settle(&mut rig, &mut app);
    assert_eq!(focused(&rig), Some(app.field), "a press focuses the field");
    let main = rig.main;
    assert!(rig.driver.platform.ime_for(main).any(|r| matches!(r, ImeRequest::Start { .. })), "the field's focus starts the window's IME session");
    let views = app.view_runs;
    for (key, c) in [(Key::H, "h"), (Key::I, "i")] {
        let typed = type_char(&mut rig, &mut app, key, c);
        assert_eq!((typed.frame.runs, typed.frame.scopes), (1, 2), "the field runs, under the root and its view: {typed:?}");
    }
    assert_eq!(app.text, "hi", "each character is inserted once");
    assert_eq!(app.view_runs, views + 2, "the view ran only to reach the field");
    let back = press(&mut rig, &mut app, Key::Backspace, Mods::NONE);
    assert_eq!(app.text, "h", "Backspace deletes through the intents");
    assert_eq!(back.frame.runs, 1);
    let cursor = rig.driver.platform.ime_for(main).filter_map(|r| if let ImeRequest::Cursor(c) = r { Some(*c) } else { None }).last();
    assert!(cursor.is_some_and(|c| app.field.contains(c.center())), "the caret rect follows the typing: {cursor:?}");
}

#[test]
fn ime_composition_runs_the_field_only_and_its_commit_inserts_once() {
    let mut app = App::new(Layer::None);
    let mut rig = rig(&mut app);
    let field = app.field;
    click(&mut rig, &mut app, field);
    settle(&mut rig, &mut app);
    for text in ["k", "か"] {
        let composing = send(&mut rig, &mut app, vec![preedit(text)]);
        assert_eq!((composing.frame.runs, composing.frame.scopes), (1, 2), "a preedit runs the field's path: {composing:?}");
        assert_eq!(app.text, "", "a preedit is not text");
    }
    let held = press(&mut rig, &mut app, Key::Enter, Mods::NONE);
    assert_eq!((held.frame.runs, held.frame.scopes), (0, 0), "keys during the composition are the input method's: {held:?}");
    let commit = send(&mut rig, &mut app, vec![Input::ImeCommit("か".to_string())]);
    assert_eq!(commit.frame.runs, 1, "{commit:?}");
    assert_eq!(app.text, "か", "the commit inserts once");
    let after = send(&mut rig, &mut app, vec![preedit("")]);
    assert_eq!(app.text, "か", "an empty preedit after the commit inserts nothing: {after:?}");
}

#[test]
fn a_click_elsewhere_during_composition_commits_then_moves_the_focus() {
    let mut app = App::new(Layer::None);
    let mut rig = rig(&mut app);
    let field = app.field;
    click(&mut rig, &mut app, field);
    settle(&mut rig, &mut app);
    send(&mut rig, &mut app, vec![preedit("ab")]);
    assert_eq!(app.text, "");
    let a = app.a;
    let (down, _) = click(&mut rig, &mut app, a);
    assert_eq!(app.text, "ab", "the composition is committed when the press moves the focus");
    assert_eq!(down.frame.runs, 1, "the press frame runs the field for the commit: {down:?}");
    assert_eq!(focused(&rig), Some(app.a), "the focus moved to the pressed button");
    let main = rig.main;
    assert!(rig.driver.platform.ime_for(main).any(|r| *r == ImeRequest::Stop), "the IME session stopped with the focus");
    send(&mut rig, &mut app, vec![Input::ImeCommit("ab".to_string())]);
    assert_eq!(app.text, "ab", "a late commit from the platform reaches no field");
}

#[test]
fn enter_in_a_dialog_submits_once_and_runs_only_the_dialog_s_scope() {
    let mut app = App::new(Layer::Dialog);
    let mut rig = rig(&mut app);
    type_char(&mut rig, &mut app, Key::X, "x");
    assert_eq!(app.query, "x", "the dialog's field took the focus when the dialog appeared (FirstField)");
    let enter = press(&mut rig, &mut app, Key::Enter, Mods::NONE);
    assert_eq!(app.submits, 1, "the Enter the field left is the dialog's Submit");
    assert_eq!((enter.frame.runs, enter.frame.scopes), (0, 2), "the dialog's scope runs under the root, its field keeps: {enter:?}");
    press(&mut rig, &mut app, Key::Enter, Mods::NONE);
    assert_eq!(app.submits, 2, "one Submit per Enter");
    let idle = step(&mut rig, &mut app);
    assert!(!idle.ran && app.submits == 2, "nothing replays the Submit");
}

#[test]
fn escape_under_clear_then_close_clears_first_and_closes_on_the_second() {
    let mut app = App::new(Layer::Palette);
    let mut rig = rig(&mut app);
    type_char(&mut rig, &mut app, Key::A, "a");
    type_char(&mut rig, &mut app, Key::B, "b");
    assert_eq!(app.query, "ab");
    let first = press(&mut rig, &mut app, Key::Escape, Mods::NONE);
    assert_eq!(app.query, "", "the first Escape clears the palette's content");
    assert_eq!(app.layer, Layer::Palette, "and leaves it open");
    assert_eq!((first.frame.runs, first.frame.scopes), (1, 2), "the palette's scope and its cleared field run: {first:?}");
    settle(&mut rig, &mut app);
    press(&mut rig, &mut app, Key::Escape, Mods::NONE);
    settle(&mut rig, &mut app);
    assert_eq!(app.layer, Layer::None, "the second Escape closes it");
}

/// An app with the command `view.grid` on Cmd+G, whose handler flips a grid cell and the command's
/// checked state. Returns the app, the rig and the handler's run count.
fn command_app() -> (App, Rig<RecordingRenderer>, Rc<Cell<u32>>) {
    let rig = Rig::inline(Vec2::new(400.0, 600.0), 1.0, RecordingRenderer::default());
    let os = rig.driver.runtime.os();
    let def = CommandDef::new("view.grid", "Show Grid", Scope::app()).with_default(Binding::new(Chord::new(Mods::CMD, Key::G)));
    let board = CommandBoard::new(Keymap::new(vec![def], Scopes::new(), os));
    let mut rig = rig;
    let grid: State<bool> = rig.driver.runtime.store_mut().create(false);
    let runs = Rc::new(Cell::new(0));
    let (handler_board, handler_runs) = (board.clone(), runs.clone());
    board.on("view.grid", move |_, store| {
        handler_runs.set(handler_runs.get() + 1);
        let on = !*store.peek(grid);
        grid.set(store, on);
        handler_board.set_state(store, "view.grid", CommandState::new().checked(on));
    });
    let mut app = App::new(Layer::None);
    app.board = Some(board);
    step(&mut rig, &mut app);
    settle(&mut rig, &mut app);
    (app, rig, runs)
}

#[test]
fn a_command_shortcut_runs_the_command_and_reruns_only_the_views_that_read_its_state() {
    let (mut app, mut rig, runs) = command_app();
    let (readers, others) = (app.reader_runs, app.other_runs);
    let fired = press(&mut rig, &mut app, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 1, "the shortcut ran the command once");
    assert_eq!(app.checked, Some(true), "the reader shows the new state in the frame of the key");
    assert_eq!(app.reader_runs, readers + 1, "the view that reads the state ran");
    assert_eq!(app.other_runs, others, "the view that does not read it kept");
    assert_eq!((fired.frame.runs, fired.frame.scopes), (0, 2), "the reader's scope under the root, no component: {fired:?}");
    let field = app.field;
    click(&mut rig, &mut app, field);
    settle(&mut rig, &mut app);
    type_char(&mut rig, &mut app, Key::G, "g");
    assert_eq!((runs.get(), app.text.as_str()), (1, "g"), "a plain key types in the focused field, it runs no command");
    press(&mut rig, &mut app, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 2, "a command chord still runs while the field types");
}

#[test]
fn a_disabled_command_s_shortcut_runs_nothing() {
    let (mut app, mut rig, runs) = command_app();
    let board = app.board.clone().expect("the app has a board");
    board.set_state(rig.driver.runtime.store_mut(), "view.grid", CommandState::new().enabled(false));
    settle(&mut rig, &mut app);
    let readers = app.reader_runs;
    let pressed = press(&mut rig, &mut app, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 0, "a disabled command does not run");
    assert_eq!((pressed.frame.runs, pressed.frame.scopes), (0, 0), "nothing runs: {pressed:?}");
    assert_eq!(app.reader_runs, readers);
}
