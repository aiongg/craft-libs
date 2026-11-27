//! Trials of text input, intents, commands and selection parts on the real trait: a typed character, an IME step, a
//! dialog's Submit, the cancel rule and a command shortcut run only what they must; a selection part selects by the
//! pointer and copies; a press on empty space gives the focus up unless a trapping scope or a canvas keeps it; a press
//! on a shape that takes no focus gives up only a text stop's focus; a lone-Alt claim takes the Alt before the menu
//! bar. Relations only, no times.

use crate::skeleton::IconButton;
use crate::text_field::TextField;
use craft_behavior::{CancelRule, IconId, PartKey};
use craft_host::target::Os;
use craft_keymap::{Binding, Chord, CommandDef, Keymap, Scope, Scopes};
use craft_ui_contract::contract::overlay::{Overlay, show_dialog};
use craft_ui_contract::contract::{
    Bind, Common, Component, Cx, Focus as _, Input, MeasureCx, MemoKey, MemoProps, MemoView, PaintUi, PartsCx, Response, Role, Show, Ui,
};
use craft_ui_contract::primitives::TextRun;
use craft_ui_contract::runtime::commands::{CommandBoard, CommandState};
use craft_ui_contract::runtime::focus::{FocusPolicy, KeyUse};
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_platform_headless::Input as Platform;
use craft_ui_rt::{ImeRequest, Key, Mods, Pos2, Rect, Sense, StepReport, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use std::cell::Cell;
use std::rc::Rc;

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
    commands: bool,
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
            commands: false,
        }
    }
}

/// The field's view: keyed by the text it edits; it counts its runs.
struct FieldView<'a> {
    text: &'a mut String,
    runs: &'a mut u32,
}

impl MemoProps for FieldView<'_> {
    fn key(&self, key: &mut MemoKey) {
        key.feed_hash(self.text.as_str());
    }
}

fn field_view(props: &mut FieldView<'_>, ui: &mut Ui) -> Rect {
    *props.runs += 1;
    TextField::new(Bind::bound(&mut *props.text)).show(ui).response.rect
}

/// A view with no input but its run counter, which feeds nothing.
struct Runs<'a>(&'a mut u32);

impl MemoProps for Runs<'_> {
    fn key(&self, _: &mut MemoKey) {}
}

fn reader_view(runs: &mut Runs<'_>, ui: &mut Ui) -> Option<bool> {
    *runs.0 += 1;
    ui.command_state("view.grid").is_checked()
}

fn other_view(runs: &mut Runs<'_>, _: &mut Ui) {
    *runs.0 += 1;
}

fn app_ui(ui: &mut Ui<'_, '_>, app: &mut App) {
    app.a = IconButton::new(IconId::new("icon"), "A").show(ui).rect;
    let props = FieldView { text: &mut app.text, runs: &mut app.view_runs };
    app.field = MemoView::new("field", props, field_view).show(ui).expect("in view").value;
    if app.commands {
        app.checked = MemoView::new("reader", Runs(&mut app.reader_runs), reader_view).show(ui).and_then(|shown| shown.value);
        let _ = MemoView::new("other", Runs(&mut app.other_runs), other_view).show(ui);
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

fn harness(app: App, board: Option<CommandBoard>) -> Harness<App> {
    let builder = Harness::builder().with_size(Vec2::new(400.0, 600.0));
    let builder = match board {
        Some(board) => builder.with_commands(board),
        None => builder,
    };
    let mut h = builder.build_ui_state(app_ui, app);
    h.os(Os::Mac);
    h.run();
    h
}

/// The key's press as one frame (its report), then its release.
fn press(h: &mut Harness<App>, key: Key, mods: Mods) -> StepReport {
    h.send(Platform::Key { key, mods, down: true });
    let report = h.last_report();
    h.send(Platform::Key { key, mods, down: false });
    report
}

/// Types `c` as a platform does: its key press with its text; the report of the press's frame.
fn type_char(h: &mut Harness<App>, key: Key, c: &str) -> StepReport {
    h.send(Platform::Key { key, mods: Mods::NONE, down: true });
    h.type_text(c);
    let report = h.last_report();
    h.send(Platform::Key { key, mods: Mods::NONE, down: false });
    report
}

/// Clicks the middle of `at` and runs to rest; the report of the press's frame.
fn click(h: &mut Harness<App>, at: Rect) -> StepReport {
    h.send(Platform::mouse_move(at.center()));
    h.send(Platform::mouse_down(at.center()));
    let down = h.last_report();
    h.send(Platform::mouse_up(at.center()));
    h.run();
    down
}

fn focused(h: &Harness<App>) -> Option<Rect> {
    h.focused().map(|target| target.bounds)
}

#[test]
fn typing_into_a_focused_field_runs_only_the_field_s_path_per_character() {
    let mut h = harness(App::new(Layer::None), None);
    let field = h.state().field;
    click(&mut h, field);
    assert_eq!(focused(&h), Some(field), "a press focuses the field");
    let main = h.window();
    assert!(h.driver.platform.ime_for(main).any(|r| matches!(r, ImeRequest::Start { .. })), "the field's focus starts the window's IME session");
    let views = h.state().view_runs;
    for (key, c) in [(Key::H, "h"), (Key::I, "i")] {
        let typed = type_char(&mut h, key, c);
        assert_eq!(typed.frame.runs, 1, "the field alone runs: {typed:?}");
    }
    assert_eq!(h.state().text, "hi", "each character is inserted once");
    assert_eq!(h.state().view_runs, views + 2, "the view ran only to reach the field");
    let back = press(&mut h, Key::Backspace, Mods::NONE);
    assert_eq!(h.state().text, "h", "Backspace deletes through the intents");
    assert_eq!(back.frame.runs, 1);
    let cursor = h.driver.platform.ime_for(main).filter_map(|r| if let ImeRequest::Cursor(c) = r { Some(*c) } else { None }).last();
    assert!(cursor.is_some_and(|c| field.contains(c.center())), "the caret rect follows the typing: {cursor:?}");
}

#[test]
fn ime_composition_runs_the_field_only_and_its_commit_inserts_once() {
    let mut h = harness(App::new(Layer::None), None);
    let field = h.state().field;
    click(&mut h, field);
    for text in ["k", "か"] {
        h.ime_preedit(text);
        let composing = h.last_report();
        assert_eq!(composing.frame.runs, 1, "a preedit runs the field: {composing:?}");
        assert_eq!(h.state().text, "", "a preedit is not text");
    }
    let held = press(&mut h, Key::Enter, Mods::NONE);
    assert_eq!(held.frame.runs, 0, "keys during the composition are the input method's: {held:?}");
    h.ime_commit("か");
    assert_eq!(h.last_report().frame.runs, 1);
    assert_eq!(h.state().text, "か", "the commit inserts once");
    h.ime_preedit("");
    assert_eq!(h.state().text, "か", "an empty preedit after the commit inserts nothing");
}

#[test]
fn a_click_elsewhere_during_composition_commits_then_moves_the_focus() {
    let mut h = harness(App::new(Layer::None), None);
    let field = h.state().field;
    click(&mut h, field);
    h.ime_preedit("ab");
    assert_eq!(h.state().text, "");
    let a = h.state().a;
    let down = click(&mut h, a);
    assert_eq!(h.state().text, "ab", "the composition is committed when the press moves the focus");
    assert!(down.frame.runs >= 1, "the press frame runs the field for the commit: {down:?}");
    assert_eq!(focused(&h), Some(a), "the focus moved to the pressed button");
    let main = h.window();
    assert!(h.driver.platform.ime_for(main).any(|r| *r == ImeRequest::Stop), "the IME session stopped with the focus");
    h.ime_commit("ab");
    assert_eq!(h.state().text, "ab", "a late commit from the platform reaches no field");
}

#[test]
fn enter_in_a_dialog_submits_once_and_runs_no_component() {
    let mut h = harness(App::new(Layer::Dialog), None);
    type_char(&mut h, Key::X, "x");
    assert_eq!(h.state().query, "x", "the dialog's field took the focus when the dialog appeared (FirstField)");
    let enter = press(&mut h, Key::Enter, Mods::NONE);
    assert_eq!(h.state().submits, 1, "the Enter the field left is the dialog's Submit");
    assert_eq!(enter.frame.runs, 0, "the dialog's scope runs, its field keeps: {enter:?}");
    press(&mut h, Key::Enter, Mods::NONE);
    assert_eq!(h.state().submits, 2, "one Submit per Enter");
    h.step();
    assert!(!h.last_report().ran && h.state().submits == 2, "nothing replays the Submit");
}

#[test]
fn escape_under_clear_then_close_clears_first_and_closes_on_the_second() {
    let mut h = harness(App::new(Layer::Palette), None);
    type_char(&mut h, Key::A, "a");
    type_char(&mut h, Key::B, "b");
    assert_eq!(h.state().query, "ab");
    let first = press(&mut h, Key::Escape, Mods::NONE);
    assert_eq!(h.state().query, "", "the first Escape clears the palette's content");
    assert_eq!(h.state().layer, Layer::Palette, "and leaves it open");
    assert_eq!(first.frame.runs, 1, "the palette's cleared field runs: {first:?}");
    h.run();
    press(&mut h, Key::Escape, Mods::NONE);
    h.run();
    assert_eq!(h.state().layer, Layer::None, "the second Escape closes it");
}

/// An app with the command `view.grid` on Cmd+G, whose handler flips the command's checked state. Returns the
/// harness and the handler's run count.
fn command_app() -> (Harness<App>, Rc<Cell<u32>>) {
    let def = CommandDef::new("view.grid", "Show Grid", Scope::app()).with_default(Binding::new(Chord::new(Mods::CMD, Key::G)));
    let board = CommandBoard::new(Keymap::new(vec![def], Scopes::new(), Os::Mac));
    let runs = Rc::new(Cell::new(0));
    let (handler_board, handler_runs) = (board.clone(), runs.clone());
    let checked = Rc::new(Cell::new(false));
    board.on("view.grid", move |_, store| {
        handler_runs.set(handler_runs.get() + 1);
        checked.set(!checked.get());
        handler_board.set_state(store, "view.grid", CommandState::new().checked(checked.get()));
    });
    let mut app = App::new(Layer::None);
    app.commands = true;
    (harness(app, Some(board)), runs)
}

#[test]
fn a_command_shortcut_runs_the_command_and_reruns_only_the_views_that_read_its_state() {
    let (mut h, runs) = command_app();
    let (readers, others) = (h.state().reader_runs, h.state().other_runs);
    let fired = press(&mut h, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 1, "the shortcut ran the command once");
    assert_eq!(h.state().checked, Some(true), "the reader shows the new state in the frame of the key");
    assert_eq!(h.state().reader_runs, readers + 1, "the view that reads the state ran");
    assert_eq!(h.state().other_runs, others, "the view that does not read it kept");
    assert_eq!(fired.frame.runs, 0, "the reader's scope runs, no component: {fired:?}");
    let field = h.state().field;
    click(&mut h, field);
    type_char(&mut h, Key::G, "g");
    assert_eq!((runs.get(), h.state().text.as_str()), (1, "g"), "a plain key types in the focused field, it runs no command");
    press(&mut h, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 2, "a command chord still runs while the field types");
}

#[test]
fn a_disabled_command_s_shortcut_runs_nothing() {
    let (mut h, runs) = command_app();
    h.command_state("view.grid", CommandState::new().enabled(false));
    h.run();
    let readers = h.state().reader_runs;
    let pressed = press(&mut h, Key::G, Mods::CMD);
    assert_eq!(runs.get(), 0, "a disabled command does not run");
    assert_eq!((pressed.frame.runs, pressed.frame.scopes), (0, 0), "nothing runs: {pressed:?}");
    assert_eq!(h.state().reader_runs, readers);
}

// ----- selection parts

const WORDS: PartKey = PartKey::new("words");
const SENTENCE: &str = "alpha beta gamma";

/// Read-only text with one selection part over its whole width, as `SelectableText` shows a label.
#[derive(Component)]
pub struct Words {
    #[common]
    common: Common,
}

impl Component for Words {
    const ROLE: Role = Role::LABEL;
    const INPUT: Input = Input::Display;
    const OVERLAYS: bool = false;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: WordsStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(300.0, cx.tokens.size.button)
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.selectable_text(WORDS, &TextRun::new(SENTENCE));
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.selectable_text(WORDS, cx.content_rect, &TextRun::new(SENTENCE));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The words in a window, and their rect.
fn words() -> (Harness<()>, Rc<Cell<Rect>>) {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let r = Rc::clone(&rect);
    let mut h = Harness::builder().build_ui(move |ui: &mut Ui<'_, '_>| r.set(Words { common: Common::new() }.show(ui).rect));
    h.run();
    (h, rect)
}

/// Copies the focused selection with the platform's Copy chord and returns the clipboard's text.
fn copy(h: &mut Harness<()>) -> Option<String> {
    let mods = if h.ctx().os().is_mac() { Mods { cmd: true, ..Mods::NONE } } else { Mods { ctrl: true, ..Mods::NONE } };
    h.key(Key::C, mods);
    h.run();
    h.copied()
}

#[test]
fn a_drag_selects_in_a_selection_part_and_copy_takes_the_selection() {
    let (mut h, rect) = words();
    let start = Pos2::new(rect.get().left() + 1.0, rect.get().center().y);
    h.drag_at(start, Vec2::new(20.0, 0.0));
    h.run();
    assert!(h.focused().is_some(), "the press focuses the selection part");
    let copied = copy(&mut h).unwrap_or_default();
    assert!(
        !copied.is_empty() && copied.len() < SENTENCE.len() && SENTENCE.starts_with(&copied),
        "the drag selected the start of the text: {copied:?}"
    );
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_the_line() {
    let (mut h, rect) = words();
    let at = Pos2::new(rect.get().left() + 2.0, rect.get().center().y);
    h.click_at(at).click_at(at);
    h.run();
    assert_eq!(copy(&mut h).as_deref(), Some("alpha"), "a double click selects the word under the pointer");
    h.click_at(at).click_at(at).click_at(at);
    h.run();
    assert_eq!(copy(&mut h).as_deref(), Some(SENTENCE), "a triple click selects the line");
}

// ----- presses on empty space

/// A stop that keeps the focus on presses no shape takes: a trapping scope of its own (`trap`), else the window's
/// document canvas.
#[derive(Component)]
pub struct Keeper {
    #[common]
    common: Common,
    trap: bool,
}

impl Component for Keeper {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: KeeperStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::FOCUS
    }

    fn keys(&self) -> KeyUse {
        let mut keys = KeyUse::of(Input::Display);
        keys.chrome_key = !self.trap;
        keys
    }

    fn focus_scope(&self) -> FocusPolicy {
        if self.trap { FocusPolicy::DIALOG } else { FocusPolicy::NONE }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Empty space below everything the windows below show.
const EMPTY: Pos2 = Pos2::new(200.0, 560.0);

#[test]
fn a_press_on_empty_space_commits_the_focused_field_and_leaves_no_focus() {
    let committed = Rc::new(Cell::new(false));
    let field = Rc::new(Cell::new(Rect::NOTHING));
    let (c, f) = (Rc::clone(&committed), Rc::clone(&field));
    let mut text = String::new();
    let mut h = Harness::builder().with_size(Vec2::new(400.0, 600.0)).build_ui(move |ui: &mut Ui<'_, '_>| {
        let shown = TextField::new(Bind::bound(&mut text)).show(ui);
        c.set(c.get() || shown.events.committed().is_some());
        f.set(shown.response.rect);
    });
    h.run();
    h.click_at(field.get().center());
    h.type_text("x");
    h.run();
    assert!(h.focused().is_some() && !committed.get(), "the field types a draft");
    h.click_at(EMPTY);
    h.run();
    assert!(h.focused().is_none(), "a press nothing takes gives the focus up");
    assert!(committed.get(), "the field commits through its focus loss");
}

#[test]
fn a_press_on_empty_space_keeps_the_focus_in_a_trapping_scope_and_on_a_canvas() {
    for trap in [true, false] {
        let keeper = Rc::new(Cell::new(Rect::NOTHING));
        let k = Rc::clone(&keeper);
        let mut h = Harness::builder()
            .with_size(Vec2::new(400.0, 600.0))
            .build_ui(move |ui: &mut Ui<'_, '_>| k.set(Keeper { common: Common::new(), trap }.show(ui).rect));
        h.run();
        h.click_at(keeper.get().center());
        h.run();
        let focus = h.focused().map(|target| target.id);
        assert!(focus.is_some(), "a press focuses the keeper (trap {trap})");
        h.click_at(EMPTY);
        h.run();
        assert_eq!(h.focused().map(|target| target.id), focus, "a press nothing takes leaves the focus where it is (trap {trap})");
    }
}

/// A shape a press acts on that takes no focus (a swatch, a static row).
#[derive(Component)]
pub struct Tile {
    #[common]
    common: Common,
}

impl Component for Tile {
    const ROLE: Role = Role::GROUP;
    const STOPS: bool = false;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TileStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A plain stop a press focuses (a button that is a Tab stop).
#[derive(Component)]
pub struct Knob {
    #[common]
    common: Common,
}

impl Component for Knob {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: KnobStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::FOCUS
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[test]
fn a_press_on_a_shape_that_takes_no_focus_commits_a_field_and_leaves_a_button_focused() {
    let committed = Rc::new(Cell::new(false));
    let rects = Rc::new(Cell::new([Rect::NOTHING; 3]));
    let (c, r) = (Rc::clone(&committed), Rc::clone(&rects));
    let mut text = String::new();
    let mut h = Harness::builder().with_size(Vec2::new(400.0, 600.0)).build_ui(move |ui: &mut Ui<'_, '_>| {
        let shown = TextField::new(Bind::bound(&mut text)).show(ui);
        c.set(c.get() || shown.events.committed().is_some());
        let tile = Tile { common: Common::new() }.show(ui).rect;
        let knob = Knob { common: Common::new() }.show(ui).rect;
        r.set([shown.response.rect, tile, knob]);
    });
    h.run();
    let [field, tile, knob] = rects.get();
    h.click_at(field.center());
    h.type_text("x");
    h.run();
    assert!(h.focused().is_some() && !committed.get(), "the field types a draft");
    h.click_at(tile.center());
    h.run();
    assert!(h.focused().is_none(), "a press on a shape that takes no focus gives the field's focus up");
    assert!(committed.get(), "the field commits through its focus loss");
    h.click_at(knob.center());
    h.run();
    let focus = h.focused().map(|target| target.id);
    assert!(focus.is_some(), "a press focuses the button");
    h.click_at(tile.center());
    h.run();
    assert_eq!(h.focused().map(|target| target.id), focus, "the button keeps the focus");
}

#[test]
fn a_press_on_a_shape_that_takes_no_focus_leaves_a_dialog_s_field_focused() {
    let rects = Rc::new(Cell::new([Rect::NOTHING; 2]));
    let r = Rc::clone(&rects);
    let mut text = String::new();
    let mut h = Harness::builder().with_size(Vec2::new(400.0, 600.0)).build_ui(move |ui: &mut Ui<'_, '_>| {
        let text = &mut text;
        let _ = show_dialog(ui, 1, &Overlay::modal(), |ui| {
            let field = TextField::new(Bind::bound(&mut *text)).show(ui).response.rect;
            let tile = Tile { common: Common::new() }.show(ui).rect;
            r.set([field, tile]);
        });
    });
    h.run();
    let [field, tile] = rects.get();
    h.click_at(field.center());
    h.type_text("x");
    h.run();
    let focus = h.focused().map(|target| target.id);
    assert!(focus.is_some(), "the dialog's field holds the focus");
    h.click_at(tile.center());
    h.run();
    assert_eq!(h.focused().map(|target| target.id), focus, "focus stays in the trapping scope");
}

// ----- lone-Alt claims

/// A menu bar stand-in that counts the menu keys it gets.
#[derive(Component)]
pub struct MenuStub {
    #[common]
    common: Common,
    #[key(skip)]
    keys: Rc<Cell<usize>>,
}

impl Component for MenuStub {
    const ROLE: Role = Role::MENU_BAR;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: MenuStubStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(10.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.register_menu_bar();
        let keys = cx.take_menu_keys().len();
        self.keys.set(self.keys.get() + keys);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A canvas tool stand-in that claims the lone Alt and counts the taps it gets.
#[derive(Component)]
pub struct AltTool {
    #[common]
    common: Common,
    #[key(skip)]
    taps: Rc<Cell<usize>>,
}

impl Component for AltTool {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: AltToolStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(4.0 * cx.tokens.size.button, cx.tokens.size.button)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.claim_lone_alt();
        if cx.take_lone_alt() {
            self.taps.set(self.taps.get() + 1);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn alt(h: &mut Harness<bool>) {
    h.send(Platform::Modifiers(Mods { alt: true, ..Mods::NONE }));
    h.send(Platform::Modifiers(Mods::NONE));
    h.run();
}

#[test]
fn a_lone_alt_claim_takes_the_alt_before_the_menu_bar() {
    let (keys, taps) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (k, t) = (Rc::clone(&keys), Rc::clone(&taps));
    let view = move |ui: &mut Ui<'_, '_>, claims: &mut bool| {
        MenuStub { common: Common::new(), keys: Rc::clone(&k) }.show(ui);
        if *claims {
            AltTool { common: Common::new(), taps: Rc::clone(&t) }.show(ui);
        }
    };
    let mut h = Harness::builder().build_ui_state(view, true);
    h.os(Os::Windows);
    h.run();
    alt(&mut h);
    assert_eq!((taps.get(), keys.get()), (1, 0), "the claiming tool takes the lone Alt, the menu bar gets none");
    *h.state_mut() = false;
    h.run();
    alt(&mut h);
    assert_eq!((taps.get(), keys.get()), (1, 1), "with no claim shown the lone Alt reaches the menu bar");
}
