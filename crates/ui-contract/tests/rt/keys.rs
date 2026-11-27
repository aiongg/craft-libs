//! Count trials of the window's command keys on craft-ui-rt: a chord bound to a command runs it once per press, waits
//! while a text stop has the keyboard, and follows the keymap's overrides. Relations only, no times.

use crate::skeleton::IconButton;
use craft_behavior::IconId;
use craft_keymap::{Binding, CommandDef, Keymap, Overrides, Platform, Scope, Scopes};
use craft_ui_contract::contract::{Common, Component, Input as InputKind, MeasureCx, Response, Role, Show, Ui};
use craft_ui_contract::runtime::commands::CommandBoard;
use craft_ui_contract::style::StyleCx;
use craft_ui_contract::testing::Harness;
use craft_ui_rt::{Key, Mods, Sense, Vec2};
use craft_ui_test_kit::contract::PlainStyle;
use std::cell::Cell;
use std::rc::Rc;

/// A text stop: a focused `Field` has the keyboard for typing.
#[derive(Component)]
struct Field {
    #[common]
    common: Common,
}

impl Component for Field {
    const ROLE: Role = Role::BUTTON;
    const INPUT: InputKind = InputKind::Text;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::splat(24.0)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER | Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    fn finish(self, cx: craft_ui_contract::contract::Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// A window with a button, then a text field, whose board counts the commands that run.
fn window(overrides: Option<Overrides>) -> (Harness<()>, Rc<Cell<usize>>) {
    let go = CommandDef::new("trial.go", "Go", Scope::app()).with_default(Binding::parse("J").expect("a valid binding"));
    let mut keymap = Keymap::new(vec![go], Scopes::new(), Platform::Linux);
    if let Some(overrides) = overrides {
        keymap = keymap.with_overrides(overrides);
    }
    let board = CommandBoard::new(keymap);
    let runs = Rc::new(Cell::new(0));
    let seen = Rc::clone(&runs);
    board.on_app(move |_, _| seen.set(seen.get() + 1));
    let mut h = Harness::builder().with_commands(board).build_ui(|ui: &mut Ui<'_, '_>| {
        IconButton::new(IconId::new("go"), "Go").show(ui);
        drop(Show::show(Field { common: Common::new() }, ui));
    });
    h.run();
    (h, runs)
}

#[test]
fn a_bound_key_runs_its_command_once_per_press_and_waits_while_a_text_stop_types() {
    let (mut h, runs) = window(None);
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 1, "from the button the bound key runs the command once");
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 1, "while the field has the keyboard the plain key is typing");
}

#[test]
fn a_rebound_command_runs_on_its_new_key_only() {
    let mut overrides = Overrides::default();
    overrides.insert("trial.go", vec!["F9".to_owned()]);
    let (mut h, runs) = window(Some(overrides));
    h.key_press(Key::Tab);
    h.key_press(Key::J);
    assert_eq!(runs.get(), 0, "the default key no longer runs it");
    h.key(Key::F9, Mods::NONE);
    assert_eq!(runs.get(), 1, "the new key does");
}

/// Trials of the window root's key stage as production has it: the bare-Tab chrome toggle, the pane keys, Escape
/// chords, Quit, a modal dialog's hold on window shortcuts, the typing rule for Alt chords and the text a shortcut
/// withholds, the paste chord, and Escape over a framework drag. Behaviour follows production, not WAI-ARIA.
mod window {
    use crate::skeleton::Stack;
    use craft_behavior::{EditCommand, Intent};
    use craft_keymap::{Binding, CommandDef, Keymap, Overrides, Platform, Scope, Scopes, TypingRule};
    use craft_ui_contract::contract::overlay::{Overlay, show_dialog};
    use craft_ui_contract::contract::{Common, Component, Cx, DragPreview, DragSource, MeasureCx, NodeInput, Props as _, Response, Role, Ui};
    use craft_ui_contract::runtime::commands::{
        ChromeCommand, CommandBoard, CommandSet as _, CommandState, EditCommands, NavCommand, QuitCommand, linked_keymap,
    };
    use craft_ui_contract::runtime::env::RootEnv;
    use craft_ui_contract::runtime::focus::{EditSet, KeyUse};
    use craft_ui_contract::style::StyleCx;
    use craft_ui_contract::testing::Harness;
    use craft_ui_platform_headless::Input;
    use craft_ui_rt::{Key, Mods, Sense, Vec2};
    use craft_ui_test_kit::contract::PlainStyle;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// What a stop's runs saw.
    #[derive(Default)]
    struct Seen {
        cancels: usize,
        edits: Vec<EditCommand>,
        typed: String,
        closes: usize,
    }

    type Log = Rc<RefCell<Seen>>;

    /// A Tab stop with the key use `keys`, which holds Escape for a draft and records the intents and close requests its
    /// runs see. `drag`: it is a drag source that keeps the focus where it is when pressed. `guard`: it is the window's
    /// close guard.
    #[derive(Component)]
    #[component(unregistered)]
    struct Stop {
        #[common]
        common: Common,
        #[key(skip)]
        keys: KeyUse,
        #[key(skip)]
        seen: Log,
        drag: bool,
        guard: bool,
    }

    impl Stop {
        fn new(label: &str, keys: KeyUse, seen: &Log) -> Stop {
            Stop { common: Common::new(), keys, seen: Rc::clone(seen), drag: false, guard: false }.label(label.to_owned())
        }

        fn show(self, ui: &mut Ui<'_, '_>) {
            drop(craft_ui_contract::contract::Show::show(self, ui));
        }
    }

    impl Component for Stop {
        const ROLE: Role = Role::BUTTON;
        type Style = PlainStyle;
        type Output = Response;

        fn style(_: &StyleCx<'_>, _: StopStyleKey) -> PlainStyle {
            PlainStyle::default()
        }

        fn keys(&self) -> KeyUse {
            self.keys
        }

        fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
            Vec2::splat(24.0)
        }

        fn sense(&self) -> Sense {
            if self.drag {
                Sense::HOVER | Sense::CLICK | Sense::DRAG | Sense::FOCUS | Sense::KEEP_FOCUS
            } else {
                Sense::HOVER | Sense::CLICK | Sense::FOCUS
            }
        }

        fn drag_source(&self) -> Option<DragSource> {
            self.drag.then(|| DragSource::new(1_u32, DragPreview::None))
        }

        fn close_guard(&self) -> bool {
            self.guard
        }

        fn update(&mut self, cx: &mut Cx<'_, Self>) {
            cx.hold_escape();
            let mut seen = self.seen.borrow_mut();
            for input in cx.input() {
                match input {
                    NodeInput::Intent { intent: Intent::Cancel, .. } => seen.cancels += 1,
                    NodeInput::Intent { intent: Intent::Edit(edit), .. } => seen.edits.push(*edit),
                    NodeInput::Intent { intent: Intent::Text(text), .. } => seen.typed.push_str(text),
                    NodeInput::Close(_) => seen.closes += 1,
                    _ => {}
                }
            }
        }

        fn finish(self, cx: Cx<'_, Self>) -> Response {
            cx.response.clone()
        }
    }

    fn button() -> KeyUse {
        KeyUse::of(craft_behavior::Input::Pointer)
    }

    fn field() -> KeyUse {
        KeyUse::of(craft_behavior::Input::Text)
    }

    /// The linked command sets' keymap on Linux under `overrides`.
    fn linked(overrides: Option<Overrides>) -> Keymap {
        let keymap = linked_keymap(false, Platform::Linux).expect("the linked command sets are valid");
        match overrides {
            Some(overrides) => keymap.with_overrides(overrides),
            None => keymap,
        }
    }

    /// A keymap of one app command `id` bound to `chord`.
    fn app_keymap(id: &str, chord: &str, rule: Option<TypingRule>) -> Keymap {
        let def = CommandDef::new(id, id, Scope::app()).with_default(Binding::parse(chord).expect("a valid binding"));
        let keymap = Keymap::new(vec![def], Scopes::new(), Platform::Linux);
        match rule {
            Some(rule) => keymap.with_typing_rule(rule),
            None => keymap,
        }
    }

    /// Counts the runs of `command` on `board`.
    fn count(board: &CommandBoard, command: &'static str) -> Rc<RefCell<usize>> {
        let runs = Rc::new(RefCell::new(0));
        let seen = Rc::clone(&runs);
        board.on(command, move |_, _| *seen.borrow_mut() += 1);
        runs
    }

    /// Records the id of every command that reaches the app scope on `board`.
    fn app_log(board: &CommandBoard) -> Rc<RefCell<Vec<String>>> {
        let ids = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&ids);
        board.on_app(move |fired, _| seen.borrow_mut().push(fired.command.id.to_string()));
        ids
    }

    fn harness(board: CommandBoard, tab_toggles: bool, view: impl FnMut(&mut Ui<'_, '_>) + 'static) -> Harness<()> {
        let mut h = Harness::builder().with_commands(board).build_ui(view);
        let mut root = RootEnv::default();
        root.tab_toggles_chrome = tab_toggles;
        h.root_env(&root);
        h.run();
        h
    }

    fn focused(h: &Harness<()>) -> Option<String> {
        h.focused().map(|t| t.label)
    }

    /// Focuses the stop labelled `label` by a request (an assistive technology's Focus).
    fn request(h: &mut Harness<()>, label: &str) {
        let target = h.get_by_label(label);
        h.access(accesskit::Action::Focus, &target);
        h.run();
    }

    /// Focuses the stop labelled `label` by a press on it.
    fn click(h: &mut Harness<()>, label: &str) {
        let target = h.get_by_label(label);
        h.click(&target);
        h.run();
    }

    /// A window with a document canvas: a button, the canvas and a text field.
    fn with_canvas(seen: &Log) -> impl FnMut(&mut Ui<'_, '_>) + 'static {
        let seen = Rc::clone(seen);
        move |ui| {
            Stop::new("button", button(), &seen).show(ui);
            Stop::new("canvas", KeyUse::CANVAS, &seen).show(ui);
            Stop::new("field", field(), &seen).show(ui);
        }
    }

    #[test]
    fn tab_toggles_the_panels_where_the_gate_is_on_and_moves_focus_elsewhere() {
        let board = CommandBoard::new(linked(None));
        let toggles = count(&board, ChromeCommand::TogglePanels.id());
        let mut h = harness(board, true, with_canvas(&Log::default()));
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (1, None), "nothing focused: Tab toggles and moves no focus");
        request(&mut h, "canvas");
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (2, Some("canvas".to_owned())), "the canvas focused: Tab toggles");
        request(&mut h, "button");
        h.key_press(Key::Tab);
        assert_eq!(*toggles.borrow(), 2, "a requested component's Tab moves focus");
        assert_ne!(focused(&h).as_deref(), Some("button"), "focus left it");
        click(&mut h, "button");
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (3, Some("button".to_owned())), "a pressed component leaves Tab to the toggle");
        click(&mut h, "field");
        h.key_press(Key::Tab);
        assert_eq!(*toggles.borrow(), 3, "a text field keeps Tab for the next field, also when a press focused it");
        assert_ne!(focused(&h).as_deref(), Some("field"), "focus left the field");
    }

    #[test]
    fn tab_moves_focus_without_a_canvas_or_with_the_switch_off() {
        let seen = Log::default();
        let board = CommandBoard::new(linked(None));
        let toggles = count(&board, ChromeCommand::TogglePanels.id());
        let shown = Rc::clone(&seen);
        let mut h = harness(board, true, move |ui| {
            Stop::new("button", button(), &shown).show(ui);
            Stop::new("slider", button().with_value(true), &shown).show(ui);
        });
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (0, Some("button".to_owned())), "without a canvas Tab moves focus");

        let board = CommandBoard::new(linked(None));
        let toggles = count(&board, ChromeCommand::TogglePanels.id());
        let mut h = harness(board, false, with_canvas(&seen));
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (0, Some("button".to_owned())), "with the app's switch off Tab moves focus");
    }

    #[test]
    fn a_rebound_toggle_runs_on_its_new_key_and_tab_then_moves_focus() {
        let mut overrides = Overrides::default();
        overrides.insert(ChromeCommand::TogglePanels.id(), vec!["F9".to_owned()]);
        let board = CommandBoard::new(linked(Some(overrides)));
        let toggles = count(&board, ChromeCommand::TogglePanels.id());
        let mut h = harness(board, true, with_canvas(&Log::default()));
        h.key_press(Key::F9);
        assert_eq!(*toggles.borrow(), 1, "the new key toggles");
        h.key_press(Key::Tab);
        assert_eq!((*toggles.borrow(), focused(&h)), (1, Some("button".to_owned())), "Tab moves focus once its command is rebound");
    }

    /// A bare Tab an app command took from Show or Hide Panels runs that command where Tab would toggle the panels, and
    /// still moves focus from a text field.
    #[test]
    fn tab_runs_the_app_command_bound_to_it_where_it_would_toggle_the_panels() {
        let board = CommandBoard::new(app_keymap("trialTab.dock", "Tab", None));
        let ran = app_log(&board);
        let mut h = harness(board, true, with_canvas(&Log::default()));
        h.key_press(Key::Tab);
        assert_eq!((ran.borrow().clone(), focused(&h)), (vec!["trialTab.dock".to_owned()], None), "nothing focused: the app's command runs");
        request(&mut h, "field");
        h.key_press(Key::Tab);
        assert_eq!(ran.borrow().len(), 1, "a text field keeps Tab as its focus move");
        assert_ne!(focused(&h).as_deref(), Some("field"));
    }

    /// Two panes, each a pane scope with one button; `open` shows a modal dialog with one button.
    fn panes(seen: &Log) -> impl FnMut(&mut Ui<'_, '_>, &mut bool) + 'static {
        let seen = Rc::clone(seen);
        move |ui, open| {
            for name in ["left", "right"] {
                Stack::new().focus(craft_ui_contract::runtime::focus::FocusPolicy::PANE).show(ui, |ui| Stop::new(name, button(), &seen).show(ui));
            }
            if *open && show_dialog(ui, 1, &Overlay::dialog(), |ui| Stop::new("ok", button(), &seen).show(ui)).is_err() {
                *open = false;
            }
        }
    }

    fn pane_harness(board: CommandBoard) -> Harness<bool> {
        let mut h = Harness::builder().with_commands(board).build_ui_state(panes(&Log::default()), false);
        h.run();
        h
    }

    fn focused_in(h: &Harness<bool>) -> Option<String> {
        h.focused().map(|t| t.label)
    }

    fn request_in(h: &mut Harness<bool>, label: &str) {
        let target = h.get_by_label(label);
        h.access(accesskit::Action::Focus, &target);
        h.run();
    }

    #[test]
    fn an_app_binding_on_f6_beats_the_pane_move() {
        let f6 = |overrides: Option<Overrides>| {
            let board = CommandBoard::new(linked(overrides));
            let deletes = count(&board, EditCommands::Delete.id());
            let mut h = pane_harness(board);
            request_in(&mut h, "left");
            h.key_press(Key::F6);
            (focused_in(&h), *deletes.borrow())
        };
        assert_eq!(f6(None), (Some("right".to_owned()), 0), "by default F6 is the window's pane move");
        let mut bound = Overrides::default();
        bound.insert(NavCommand::NextPane.id(), Vec::new());
        bound.insert(EditCommands::Delete.id(), vec!["F6".to_owned()]);
        assert_eq!(f6(Some(bound)), (Some("left".to_owned()), 1), "a command bound to F6 takes it");
        let mut unbound = Overrides::default();
        unbound.insert(NavCommand::NextPane.id(), Vec::new());
        assert_eq!(f6(Some(unbound)), (Some("left".to_owned()), 0), "with the pane move unbound F6 moves nothing");
    }

    /// A modal dialog's focus trap holds F6: a pane move would carry focus out of the dialog.
    #[test]
    fn f6_inside_a_modal_dialog_keeps_the_focus_in_it() {
        let mut h = pane_harness(CommandBoard::new(linked(None)));
        request_in(&mut h, "left");
        h.key_press(Key::F6);
        assert_eq!(focused_in(&h).as_deref(), Some("right"), "outside a dialog F6 moves to the next pane");
        *h.state_mut() = true;
        h.run();
        request_in(&mut h, "ok");
        h.key_press(Key::F6);
        assert_eq!(focused_in(&h).as_deref(), Some("ok"), "F6 keeps the focus in the dialog");
        h.key(Key::F6, Mods { shift: true, ..Mods::NONE });
        assert_eq!(focused_in(&h).as_deref(), Some("ok"), "so does Shift+F6");
        *h.state_mut() = false;
        h.run();
        request_in(&mut h, "left");
        h.key_press(Key::F6);
        assert_eq!(focused_in(&h).as_deref(), Some("right"), "with the dialog gone F6 moves panes again");
    }

    /// A window shortcut runs while the keyboard is on a pane's title, and neither from a button of an open modal dialog
    /// (the command's modal rule, `Blocked` by default, keeps it from running) nor from a text field (a plain key there
    /// is typing).
    #[test]
    fn a_window_shortcut_runs_from_a_pane_title_and_not_from_a_modal_dialog_or_a_text_field() {
        let board = CommandBoard::new(app_keymap("trial.replace", "Q", None));
        let ran = app_log(&board);
        let seen = Log::default();
        let mut h = Harness::builder().with_commands(board).build_ui_state(
            move |ui, open: &mut bool| {
                Stack::new().focus(craft_ui_contract::runtime::focus::FocusPolicy::PANE).show(ui, |ui| {
                    Stop::new("title", button(), &seen).show(ui);
                    Stop::new("field", field(), &seen).show(ui);
                });
                if *open && show_dialog(ui, 1, &Overlay::dialog(), |ui| Stop::new("in dialog", button(), &seen).show(ui)).is_err() {
                    *open = false;
                }
            },
            false,
        );
        h.run();
        request_in(&mut h, "title");
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 1, "the pane's title leaves the shortcut to the window");
        request_in(&mut h, "field");
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 1, "a text field keeps the plain key for its typing");
        *h.state_mut() = true;
        h.run();
        request_in(&mut h, "in dialog");
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 1, "a modal dialog keeps the window's shortcut from running");
        *h.state_mut() = false;
        h.run();
        request_in(&mut h, "title");
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 2, "with the dialog closed the title runs it again");
    }

    /// Shift+Escape bound to a command runs it (as production's keymap ran it under a loaded place cursor); an unbound
    /// Shift+Escape and a plain Escape stay the focused draft's Cancel.
    #[test]
    fn a_command_bound_to_shift_escape_runs_and_plain_escape_still_cancels() {
        let escape = |overrides: Option<Overrides>, mods: Mods| {
            let board = CommandBoard::new(linked(overrides));
            let deletes = count(&board, EditCommands::Delete.id());
            let seen = Log::default();
            let shown = Rc::clone(&seen);
            let mut h = harness(board, false, move |ui| Stop::new("draft", button(), &shown).show(ui));
            request(&mut h, "draft");
            h.key(Key::Escape, mods);
            let cancels = seen.borrow().cancels;
            (cancels, *deletes.borrow())
        };
        let shift = Mods { shift: true, ..Mods::NONE };
        assert_eq!(escape(None, shift), (1, 0), "an unbound Shift+Escape is the draft's Cancel");
        let mut bound = Overrides::default();
        bound.insert(EditCommands::Delete.id(), vec!["Shift+Escape".to_owned()]);
        assert_eq!(escape(Some(bound.clone()), shift), (0, 1), "the bound Shift+Escape runs its command");
        assert_eq!(escape(Some(bound), Mods::NONE), (1, 0), "a plain Escape is still the draft's Cancel");
    }

    /// Quit run from its key asks the window to close through its close path (the close guard is asked once); a command
    /// that binds the chord takes it, and with Quit unbound nothing asks. The chord is F12 here, so the trial does not
    /// depend on the platform's command modifier.
    #[test]
    fn quit_asks_the_window_to_close_once_and_an_app_binding_replaces_it() {
        let quit = |overrides: Overrides| {
            let board = CommandBoard::new(linked(Some(overrides)));
            let ran = app_log(&board);
            let seen = Log::default();
            let shown = Rc::clone(&seen);
            let mut h = harness(board, false, move |ui| {
                let mut guard = Stop::new("document", button(), &shown);
                guard.guard = true;
                guard.show(ui);
            });
            h.key_press(Key::F12);
            h.run();
            let closes = seen.borrow().closes;
            let ran = ran.borrow().clone();
            (closes, ran)
        };
        let mut on_f12 = Overrides::default();
        on_f12.insert(QuitCommand::Quit.id(), vec!["F12".to_owned()]);
        assert_eq!(quit(on_f12.clone()), (1, vec![QuitCommand::Quit.id().to_owned()]), "the quit key asks the window to close once");
        let mut taken = Overrides::default();
        taken.insert(QuitCommand::Quit.id(), Vec::new());
        taken.insert(EditCommands::Delete.id(), vec!["F12".to_owned()]);
        assert_eq!(quit(taken), (0, vec![EditCommands::Delete.id().to_owned()]), "a command bound to the chord takes it");
        let mut unbound = Overrides::default();
        unbound.insert(QuitCommand::Quit.id(), Vec::new());
        assert_eq!(quit(unbound), (0, Vec::new()), "with Quit unbound nothing asks the window to close");
    }

    /// What an Alt+`key` press with `text` sent with it did from a focused text stop, with the app command bound to the
    /// chord `enabled`, under the typing rule `rule`: the commands it ran and the text the stop was given.
    fn alt_while_typing(chord: &str, key: Key, mods: Mods, text: &str, rule: Option<TypingRule>, enabled: bool) -> (usize, String) {
        let board = CommandBoard::new(app_keymap("trial.search", chord, rule));
        let ran = app_log(&board);
        let seen = Log::default();
        let shown = Rc::clone(&seen);
        let mut h = harness(board, false, move |ui| Stop::new("document", field(), &shown).show(ui));
        h.command_state("trial.search", CommandState::new().enabled(enabled));
        h.run();
        request(&mut h, "document");
        let window = h.window();
        h.driver.platform.inject(window, Input::Key { key, mods, down: true });
        h.driver.platform.inject(window, Input::Text(text.to_owned()));
        h.step();
        h.send(Input::Key { key, mods, down: false });
        h.run();
        let typed = seen.borrow().typed.clone();
        (ran.borrow().len(), typed)
    }

    /// An app sets the typing rule through its keymap: by default an Alt+letter chord stays with the text (Option types
    /// characters on the Mac); under `PlainKeysWait` it fires, as Word's Alt+Q opens search from the page, and the text
    /// the platform sends with it is withheld. An Alt+function-key chord fires from a text field under the default rule.
    #[test]
    fn the_typing_rule_lets_an_alt_chord_fire_while_typing_and_its_text_is_withheld() {
        let alt = Mods { alt: true, ..Mods::NONE };
        assert_eq!(alt_while_typing("Alt+Q", Key::Q, alt, "x", None, true), (0, "x".to_owned()), "by default the Alt chord types");
        assert_eq!(alt_while_typing("Alt+Q", Key::Q, alt, "x", Some(TypingRule::PlainKeysWait), true), (1, String::new()), "it fires");
        assert_eq!(alt_while_typing("Alt+F7", Key::F7, alt, "", None, true).0, 1, "Alt+F7 fires from the text field");
    }

    /// The text of a bound Alt chord is withheld only when its command runs: disabled, the shortcut does nothing and the
    /// character is typed, as Word types Option+Shift+-'s em dash outside Outline view.
    #[test]
    fn a_disabled_alt_chord_leaves_its_text_typed() {
        let alt_shift = Mods { alt: true, shift: true, ..Mods::NONE };
        let press = |enabled| alt_while_typing("Alt+Shift+-", Key::Minus, alt_shift, "\u{2014}", Some(TypingRule::PlainKeysWait), enabled);
        assert_eq!(press(false), (0, "\u{2014}".to_owned()), "disabled: the em dash is typed");
        assert_eq!(press(true), (1, String::new()), "enabled: the command runs and types nothing");
    }

    /// The paste chord runs the app's Paste when no focused component lists Paste, as a picture's paste does: on
    /// craft-ui-rt the chord is always a key, and the app reads the clipboard's text or picture itself. A focused stop
    /// that lists Paste takes the chord as its edit, and Paste does not run. The chord is F4 here, so the trial does not
    /// depend on the platform's command modifier.
    #[test]
    fn the_paste_chord_is_the_app_s_paste_unless_the_focused_stop_lists_it() {
        let mut overrides = Overrides::default();
        overrides.insert(EditCommands::Paste.id(), vec!["F4".to_owned()]);
        let board = CommandBoard::new(linked(Some(overrides))).with_edits::<EditCommands>();
        let pastes = count(&board, EditCommands::Paste.id());
        let seen = Log::default();
        let shown = Rc::clone(&seen);
        let mut pasting = KeyUse::CANVAS;
        pasting.edits = EditSet::NONE.with(EditCommand::Paste);
        let mut h = harness(board, false, move |ui| {
            Stop::new("canvas", KeyUse::CANVAS, &shown).show(ui);
            Stop::new("pasting canvas", pasting, &shown).show(ui);
        });
        h.key_press(Key::F4);
        assert_eq!(*pastes.borrow(), 1, "nothing focused: the app's Paste runs");
        request(&mut h, "canvas");
        h.key_press(Key::F4);
        assert_eq!(*pastes.borrow(), 2, "a canvas without Paste: the app's Paste runs");
        request(&mut h, "pasting canvas");
        h.key_press(Key::F4);
        assert_eq!((*pastes.borrow(), seen.borrow().edits.clone()), (2, vec![EditCommand::Paste]), "the canvas's paste is its edit");
    }

    /// Escape during a framework drag cancels the drag and goes to the drag alone: the focused draft gets no Cancel.
    #[test]
    fn escape_cancels_a_framework_drag_while_a_text_stop_has_focus() {
        let seen = Log::default();
        let shown = Rc::clone(&seen);
        let mut h = harness(CommandBoard::new(linked(None)), false, move |ui| {
            Stop::new("field", field(), &shown).show(ui);
            let mut source = Stop::new("source", KeyUse::of(craft_behavior::Input::Pointer), &shown);
            source.drag = true;
            source.show(ui);
        });
        request(&mut h, "field");
        let from = h.get_by_label("source").center();
        h.drag_hold_at(from, Vec2::new(40.0, 0.0));
        assert!(h.driver.runtime.drag().is_some(), "the drag runs while the button is held");
        assert_eq!(focused(&h).as_deref(), Some("field"), "the drag source left the focus on the field");
        h.key_press(Key::Escape);
        assert!(h.driver.runtime.drag().is_none(), "Escape cancelled the drag");
        assert_eq!(seen.borrow().cancels, 0, "the Escape went to the drag alone");
        h.release_at(from + Vec2::new(40.0, 0.0));
    }

    /// The command stage works out where a key stands against the modal layers once per key: the lookup that skips a
    /// command the modal rule refuses and the fire share it, with a dialog open or not.
    #[test]
    fn a_key_takes_one_modal_check_with_a_dialog_open_or_not() {
        use craft_ui_contract::testing::modal_checks;
        let board = CommandBoard::new(app_keymap("trial.replace", "Q", None));
        let ran = app_log(&board);
        let seen = Log::default();
        let mut h = Harness::builder().with_commands(board).build_ui_state(
            move |ui, open: &mut bool| {
                Stop::new("title", button(), &seen).show(ui);
                if *open && show_dialog(ui, 1, &Overlay::dialog(), |ui| Stop::new("in dialog", button(), &seen).show(ui)).is_err() {
                    *open = false;
                }
            },
            false,
        );
        h.run();
        request_in(&mut h, "title");
        let before = modal_checks();
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 1, "the shortcut ran");
        assert_eq!(modal_checks() - before, 1, "one modal check for the key");
        *h.state_mut() = true;
        h.run();
        request_in(&mut h, "in dialog");
        let before = modal_checks();
        h.key_press(Key::Q);
        assert_eq!(ran.borrow().len(), 1, "the dialog keeps the shortcut from running");
        assert_eq!(modal_checks() - before, 1, "still one modal check for the key");
    }

    /// An edit command the focused stop declares resolves as the stop's (#3569): enabled though the app disabled it,
    /// with the app's title and without the app's hint. Off the stop, the app's state shows again.
    #[test]
    fn a_declared_edit_resolves_enabled_with_the_app_title_and_no_app_hint() {
        use craft_ui_contract::runtime::commands::ResolvedCommand;
        use std::sync::Arc;
        let board = CommandBoard::new(linked(None)).with_edits::<EditCommands>();
        let shown: Rc<RefCell<Option<Arc<ResolvedCommand>>>> = Rc::default();
        let seen = Log::default();
        let out = Rc::clone(&shown);
        let mut h = harness(board.clone(), false, move |ui| {
            Stop::new("button", button(), &seen).show(ui);
            Stop::new("field", field(), &seen).show(ui);
            *out.borrow_mut() = ui.resolve_command(EditCommands::Copy);
        });
        let state = CommandState::new().enabled(false).title("Copy Layer").hint("Copies the layer");
        board.set_state(&mut h.driver.runtime, EditCommands::Copy, state);
        request(&mut h, "button");
        let off = shown.borrow().clone().expect("Copy resolves at the app");
        assert!(!off.enabled, "a stop that does not take Copy leaves the app's state");
        assert_eq!(off.hint.as_deref(), Some("Copies the layer"), "with the app's hint");
        request(&mut h, "field");
        let on = shown.borrow().clone().expect("Copy resolves at the app");
        assert!(on.enabled, "the text field takes Copy: it resolves enabled");
        assert_eq!(&*on.label, "Copy Layer", "with the app's title");
        assert_eq!(on.hint, None, "and without the app's hint");
    }
}
