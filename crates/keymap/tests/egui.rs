//! Integration tests of craft-keymap: egui.

#![cfg(feature = "egui")]

use craft_keymap::egui::Dispatcher;
use craft_keymap::{Active, Bindable, Binding, Chord, KeyInput, Keymap, Kind, Layout, Platform, Scope, Scopes};
use egui::{Event, InputState, Key as EguiKey, Modifiers};

fn chord(text: &str) -> Chord {
    Chord::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

const CTRL: Modifiers = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };

fn dispatch_keymap() -> Keymap {
    let app = |id: &str, keys: &[&str]| keys.iter().fold(Bindable::new(id, id, Scope::app()), |e, k| e.with_default(Binding::parse(k).unwrap()));
    let tool = |id: &str| app(id, &["M"]).with_kind(Kind::Tool { group: Some("select".into()) });
    let catalog = vec![
        app("edit.copy", &["Cmd+C", "Cmd+Insert"]),
        app("edit.paste", &["Cmd+V", "Shift+Insert"]),
        app("edit.pasteInPlace", &["Cmd+Shift+V"]),
        app("edit.copyMerged", &["Cmd+Shift+C"]),
        app("app.quit", &["Cmd+Alt+Q"]),
        app("type.pageNumber", &["Cmd+Alt+Shift+N"]),
        app("view.refresh", &["Cmd+Alt+F5"]),
        app("file.save", &["Cmd+S"]),
        app("view.zoomIn", &["="]),
        app("help.find", &["Shift+;"]),
        app("tool.brush", &["B"]),
        app("app.loop", &["L"]),
        Bindable::new("timeline.loop", "Loop", Scope::new("Timeline").unwrap()).with_default(Binding::parse("L").unwrap()),
        tool("tool.marquee"),
        tool("tool.ellipse"),
        app("reveal.one", &["U"]),
        app("reveal.all", &["U U"]),
    ];
    Keymap::new(catalog, Scopes::new().with_panel("Timeline"), Platform::Windows)
}

fn press(key: EguiKey, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

fn frame(time: f64, modifiers: Modifiers, events: Vec<Event>) -> InputState {
    let mut input = InputState::default();
    input.time = time;
    input.modifiers = modifiers;
    input.events = events;
    input
}

/// Dispatches one frame and returns the ids that ran and the events left.
fn run(d: &mut Dispatcher, km: &Keymap, active: &Active, mut input: InputState) -> (Vec<String>, Vec<Event>) {
    let hits = d.dispatch(&mut input, km, active);
    (hits.into_iter().map(|h| h.command.id).collect(), input.events)
}

#[test]
fn a_bound_press_runs_once_and_is_removed_with_its_text() {
    let km = dispatch_keymap();
    let events = vec![
        press(EguiKey::B, Modifiers::NONE),
        Event::Text("b".into()),
        Event::Key { key: EguiKey::B, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        press(EguiKey::Q, Modifiers::NONE),
        Event::Text("q".into()),
    ];
    let (ran, left) = run(&mut Dispatcher::new(), &km, &Active::NONE, frame(1.0, Modifiers::NONE, events));
    assert_eq!(ran, ["tool.brush"]);
    // The release and the unbound Q with its text stay.
    assert_eq!(left.len(), 3);
    assert!(matches!(left[0], Event::Key { key: EguiKey::B, pressed: false, .. }));
    assert!(matches!(left[1], Event::Key { key: EguiKey::Q, pressed: true, .. }));
    assert_eq!(left[2], Event::Text("q".into()));
}

#[test]
fn shifted_symbols_and_zoom_keys_reach_their_bindings() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let shift = Modifiers::SHIFT;
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(1.0, shift, vec![press(EguiKey::Colon, shift), Event::Text(":".into())]));
    assert_eq!(ran, ["help.find"]);
    assert!(left.is_empty());
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(2.0, Modifiers::NONE, vec![press(EguiKey::Plus, Modifiers::NONE)]));
    assert_eq!(ran, ["view.zoomIn"]);
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(3.0, shift, vec![press(EguiKey::Plus, shift)]));
    assert_eq!(ran, ["view.zoomIn"]);
}

#[test]
fn clipboard_events_stay_with_the_app_by_default() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    // egui-winit sends these in place of the Ctrl+C and Ctrl+V presses.
    let events = vec![Event::Copy, Event::Paste("text".into())];
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(1.0, CTRL, events.clone()));
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(left, events);
    // The web build sends the press and then the event: both stay.
    let events = vec![press(EguiKey::C, CTRL), Event::Copy, press(EguiKey::S, CTRL)];
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(2.0, CTRL, events.clone()));
    assert_eq!(ran, ["file.save"]);
    assert_eq!(left, events[..2]);
    // A clipboard press with no clipboard event after it is an ordinary press.
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(3.0, CTRL, vec![press(EguiKey::Insert, CTRL)]));
    assert_eq!(ran, ["edit.copy"]);
}

#[test]
fn clipboard_chords_with_more_modifiers_run_their_bindings() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let typing = Active { typing: true, ..Active::NONE };
    let ctrl_shift = Modifiers { shift: true, ..CTRL };
    // The desktop: egui-winit sends the Paste event in place of Ctrl+Shift+V.
    let mut input = frame(1.0, ctrl_shift, vec![Event::Paste("text".into())]);
    let hits = d.dispatch(&mut input, &km, &Active::NONE);
    let ran: Vec<_> = hits.iter().map(|h| (h.command.id.as_str(), h.pasted.as_deref())).collect();
    assert_eq!(ran, [("edit.pasteInPlace", Some("text"))]);
    assert!(input.events.is_empty());
    let (ran, left) = run(&mut d, &km, &typing, frame(2.0, ctrl_shift, vec![Event::Copy]));
    assert_eq!(ran, ["edit.copyMerged"]);
    assert!(left.is_empty());
    // Ctrl+Shift+X has no binding: the event stays.
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(3.0, ctrl_shift, vec![Event::Cut]));
    assert!(ran.is_empty());
    assert_eq!(left, [Event::Cut]);

    // The web: the press decides for itself and its event, also while typing.
    for (time, active) in [(4.0, &Active::NONE), (5.0, &typing)] {
        let events = vec![press(EguiKey::C, ctrl_shift), Event::Copy];
        let (ran, left) = run(&mut d, &km, active, frame(time, ctrl_shift, events));
        assert_eq!(ran, ["edit.copyMerged"]);
        assert!(left.is_empty(), "{left:?}");
    }
    let events = vec![press(EguiKey::X, ctrl_shift), Event::Cut];
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(6.0, ctrl_shift, events.clone()));
    assert!(ran.is_empty());
    assert_eq!(left, events);
}

#[test]
fn clipboard_events_run_their_chords_outside_text_fields_when_turned_on() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new().with_clipboard_events(true);
    let mut input = frame(1.0, CTRL, vec![Event::Copy, Event::Paste("text".into())]);
    let hits = d.dispatch(&mut input, &km, &Active::NONE);
    let ran: Vec<_> = hits.iter().map(|h| (h.command.id.as_str(), h.pasted.as_deref())).collect();
    assert_eq!(ran, [("edit.copy", None), ("edit.paste", Some("text"))]);
    assert!(input.events.is_empty());

    let typing = Active { typing: true, ..Active::NONE };
    let (ran, left) = run(&mut d, &km, &typing, frame(2.0, CTRL, vec![Event::Copy]));
    assert!(ran.is_empty());
    assert_eq!(left, [Event::Copy]);

    // The web build sends the key press and the clipboard event it causes: one run each.
    for (time, key, mods, event, id) in [
        (3.0, EguiKey::C, CTRL, Event::Copy, "edit.copy"),
        (4.0, EguiKey::Insert, CTRL, Event::Copy, "edit.copy"),
        (5.0, EguiKey::Insert, Modifiers::SHIFT, Event::Paste("text".into()), "edit.paste"),
    ] {
        let (ran, _) = run(&mut d, &km, &Active::NONE, frame(time, mods, vec![press(key, mods), event]));
        assert_eq!(ran, [id], "{key:?}");
    }
}

#[test]
fn a_web_paste_carries_its_text_and_takes_its_event() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new().with_clipboard_events(true);
    // The web build sends the Cmd+V press and then the Paste event with the clipboard text.
    let events = vec![press(EguiKey::V, CTRL), Event::Paste("text".into()), press(EguiKey::C, CTRL), Event::Copy];
    let mut input = frame(1.0, CTRL, events.clone());
    let mut seen = Vec::new();
    let hits = d.dispatch_filtered(&mut input, &km, &Active::NONE, |hit| {
        seen.push(hit.pasted.clone());
        true
    });
    let ran: Vec<_> = hits.iter().map(|h| (h.command.id.as_str(), h.pasted.as_deref())).collect();
    assert_eq!(ran, [("edit.paste", Some("text")), ("edit.copy", None)]);
    assert_eq!(seen, [Some("text".to_string()), None], "the filter sees the pasted text");
    assert!(input.events.is_empty(), "{:?}", input.events);

    // A declined press leaves the press and its event in place, and the event does not run again.
    let mut input = frame(2.0, CTRL, events.clone());
    let hits = d.dispatch_filtered(&mut input, &km, &Active::NONE, |hit| hit.command.id != "edit.paste");
    let ran: Vec<_> = hits.iter().map(|h| h.command.id.as_str()).collect();
    assert_eq!(ran, ["edit.copy"]);
    assert_eq!(input.events, events[..2]);
}

#[test]
fn a_held_web_paste_runs_once_when_repeats_are_off() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new().with_clipboard_events(true);
    let repeat = Event::Key { key: EguiKey::V, physical_key: Some(EguiKey::V), pressed: true, repeat: true, modifiers: CTRL };
    let paste = || Event::Paste("text".into());
    let events = vec![press(EguiKey::V, CTRL), paste(), repeat.clone(), paste()];
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(1.0, CTRL, events));
    assert_eq!(ran, ["edit.paste"]);
    assert_eq!(left, [repeat, paste()], "the repeat and its event stay in egui's input");
}

#[test]
fn declined_hits_leave_their_events_in_order() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let shift = Modifiers::SHIFT;
    let events = vec![
        press(EguiKey::B, Modifiers::NONE),
        Event::Text("b".into()),
        press(EguiKey::S, CTRL),
        press(EguiKey::Colon, shift),
        Event::Text(":".into()),
        press(EguiKey::Equals, Modifiers::NONE),
    ];
    let mut input = frame(1.0, Modifiers::NONE, events.clone());
    let mut seen = Vec::new();
    let hits = d.dispatch_filtered(&mut input, &km, &Active::NONE, |hit| {
        seen.push((hit.command.id.clone(), hit.trigger.chord.to_string(), hit.exact));
        hit.command.id != "file.save" && hit.command.id != "tool.brush"
    });
    let s = |id: &str, chord: &str, exact| (id.to_string(), chord.to_string(), exact);
    assert_eq!(seen, [s("tool.brush", "B", true), s("file.save", "Cmd+S", true), s("help.find", "Shift+;", false), s("view.zoomIn", "=", true)]);
    let ran: Vec<_> = hits.iter().map(|h| h.command.id.as_str()).collect();
    assert_eq!(ran, ["help.find", "view.zoomIn"]);
    assert_eq!(input.events, events[..3], "the declined B keeps its text, and Ctrl+S stays in its place");
}

#[test]
fn clipboard_presses_stay_with_the_text_field() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let typing = Active { typing: true, ..Active::NONE };
    // The web build's Cmd+C press and its Copy event both reach the text field.
    let events = vec![press(EguiKey::C, CTRL), Event::Copy, press(EguiKey::Insert, CTRL)];
    let (ran, left) = run(&mut d, &km, &typing, frame(1.0, CTRL, events.clone()));
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(left, events);
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(2.0, CTRL, vec![press(EguiKey::Insert, CTRL)]));
    assert_eq!(ran, ["edit.copy"]);
}

#[test]
fn ctrl_alt_is_left_to_the_text_field_only_when_it_types() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let typing = Active { typing: true, ..Active::NONE };
    // AltGr+Q on a German layout types `@`: the browser on Windows reports Ctrl+Alt+Q and the text
    // agent sends the text after it.
    let alt_gr = Modifiers { alt: true, ..CTRL };
    let events = vec![press(EguiKey::Q, alt_gr), Event::Text("@".into())];
    let (ran, left) = run(&mut d, &km, &typing, frame(1.0, alt_gr, events.clone()));
    assert!(ran.is_empty(), "{ran:?}");
    assert_eq!(left, events);
    // A real Ctrl+Alt chord types nothing, so it runs while typing.
    let all = Modifiers { shift: true, ..alt_gr };
    let (ran, left) = run(&mut d, &km, &typing, frame(2.0, all, vec![press(EguiKey::N, all)]));
    assert_eq!(ran, ["type.pageNumber"]);
    assert!(left.is_empty());
    let (ran, _) = run(&mut d, &km, &typing, frame(3.0, alt_gr, vec![press(EguiKey::Q, alt_gr)]));
    assert_eq!(ran, ["app.quit"]);
}

#[test]
fn a_second_press_runs_the_double_binding() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let key = |d: &mut Dispatcher, k: EguiKey, t: f64| run(d, &km, &Active::NONE, frame(t, Modifiers::NONE, vec![press(k, Modifiers::NONE)])).0;
    let u = |d: &mut Dispatcher, t: f64| key(d, EguiKey::U, t);
    assert_eq!(u(&mut d, 1.0), ["reveal.one"]);
    assert_eq!(u(&mut d, 1.3), ["reveal.all"]);
    // The press after a double starts over.
    assert_eq!(u(&mut d, 1.5), ["reveal.one"]);
    assert_eq!(u(&mut d, 2.5), ["reveal.one"]);
    // B in between breaks the pair.
    key(&mut d, EguiKey::B, 2.6);
    assert_eq!(u(&mut d, 2.7), ["reveal.one"]);
}

#[test]
fn a_double_press_needs_both_presses_in_one_panel() {
    let km = Keymap::new(
        vec![
            Bindable::new("x.one", "One", Scope::new("X").unwrap()).with_default(Binding::parse("A").unwrap()),
            Bindable::new("y.one", "One", Scope::new("Y").unwrap()).with_default(Binding::parse("A").unwrap()),
            Bindable::new("y.two", "Two", Scope::new("Y").unwrap()).with_default(Binding::parse("A A").unwrap()),
        ],
        Scopes::new().with_panel("X").with_panel("Y").with_panel_fallthrough(false),
        Platform::Windows,
    );
    let mut d = Dispatcher::new();
    let a = |d: &mut Dispatcher, panel, t| run(d, &km, &Active::panel(panel), frame(t, Modifiers::NONE, vec![press(EguiKey::A, Modifiers::NONE)])).0;
    assert_eq!(a(&mut d, "X", 1.0), ["x.one"]);
    assert_eq!(a(&mut d, "Y", 1.2), ["y.one"]);
    assert_eq!(a(&mut d, "Y", 1.4), ["y.two"]);
}

#[test]
fn two_presses_in_one_frame_make_a_double() {
    let km = dispatch_keymap();
    let mut d = Dispatcher::new();
    let u = press(EguiKey::U, Modifiers::NONE);
    let (ran, left) = run(&mut d, &km, &Active::NONE, frame(4.0, Modifiers::NONE, vec![u.clone(), u]));
    assert_eq!(ran, ["reveal.one", "reveal.all"]);
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn key_repeats_run_only_when_turned_on() {
    let km = Keymap::new(
        vec![
            Bindable::new("nudge.right", "Nudge", Scope::app()).with_default(Binding::parse("Right").unwrap()),
            Bindable::new("jump.right", "Jump", Scope::app()).with_default(Binding::parse("Right Right").unwrap()),
        ],
        Scopes::new(),
        Platform::Windows,
    );
    let held = |repeat: bool| Event::Key { key: EguiKey::ArrowRight, physical_key: None, pressed: true, repeat, modifiers: Modifiers::NONE };
    let events = || vec![held(false), held(true), held(true)];

    let mut on = Dispatcher::new().with_repeat(true);
    let (ran, left) = run(&mut on, &km, &Active::NONE, frame(1.0, Modifiers::NONE, events()));
    assert_eq!(ran, ["nudge.right", "nudge.right", "nudge.right"]);
    assert!(left.is_empty());
    // Repeats neither made nor broke a double: the next press within the window is one.
    let (ran, _) = run(&mut on, &km, &Active::NONE, frame(1.3, Modifiers::NONE, vec![held(false)]));
    assert_eq!(ran, ["jump.right"]);

    let (ran, left) = run(&mut Dispatcher::new(), &km, &Active::NONE, frame(1.0, Modifiers::NONE, events()));
    assert_eq!(ran, ["nudge.right"]);
    assert_eq!(left, [held(true), held(true)]);
}

#[test]
fn a_symbol_typed_with_shift_on_the_iso_key_drops_shift() {
    let app = |id: &str, key: &str| Bindable::new(id, id, Scope::app()).with_default(Binding::parse(key).unwrap());
    let km = Keymap::new(vec![app("bracket", "["), app("bracket.shift", "Shift+[")], Scopes::new(), Platform::Windows);
    // A layout on which Shift and the key next to left Shift type `[`. egui reports the physical
    // key, so the press needs Shift to type `[` and runs `[`, as Shift on other symbol keys does.
    // A `Shift+[` binding recorded from this key before it had a physical key does not run.
    let shift = Modifiers::SHIFT;
    let press = |physical_key| Event::Key { key: EguiKey::OpenBracket, physical_key, pressed: true, repeat: false, modifiers: shift };
    let input = KeyInput::from_egui(&press(Some(EguiKey::IntlBackslash)), Platform::Windows).unwrap();
    assert_eq!(Layout::new().resolve(&input), Some(chord("[")));
    let mut d = Dispatcher::new();
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(1.0, shift, vec![press(Some(EguiKey::IntlBackslash))]));
    assert_eq!(ran, ["bracket"]);
    // Without a physical key, Shift stays in the chord.
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(2.0, shift, vec![press(None)]));
    assert_eq!(ran, ["bracket.shift"]);
}

#[test]
fn letters_follow_the_latin_layout_from_egui_winit_to_the_dispatcher() {
    let app = |id: &str, key: &str| Bindable::new(id, id, Scope::app()).with_default(Binding::parse(key).unwrap());
    let km = Keymap::new(vec![app("d", "Cmd+D"), app("e", "Cmd+E")], Scopes::new(), Platform::Linux);
    let at = |key, modifiers| Event::Key { key, physical_key: Some(EguiKey::D), pressed: true, repeat: false, modifiers };

    // The QWERTY D key, as the suite's egui-winit reports it: on a layout that is not Latin, the
    // logical key is the Latin layout's letter (Dvorak types e there), and the text is Cyrillic.
    // (layout, logical key, text typed, platform report, command Ctrl runs)
    let cases = [
        ("Dvorak + Russian", EguiKey::E, "в", Some(false), "e"),
        ("QWERTY + Russian", EguiKey::D, "в", Some(false), "d"),
        ("Dvorak", EguiKey::E, "e", Some(true), "e"),
        ("QWERTY", EguiKey::D, "d", Some(true), "d"),
    ];
    for (name, key, text, latin, expected) in cases {
        // The layout learned from typing.
        let mut d = Dispatcher::new();
        run(&mut d, &km, &Active::NONE, frame(1.0, Modifiers::NONE, vec![at(key, Modifiers::NONE), Event::Text(text.into())]));
        assert_eq!(Some(d.layout().is_latin()), latin, "{name}");
        let (ran, _) = run(&mut d, &km, &Active::NONE, frame(2.0, CTRL, vec![at(key, CTRL)]));
        assert_eq!(ran, [expected], "{name}, learned");

        // The layout the platform reports, before anything is typed.
        let mut d = Dispatcher::new().with_layout(Layout::new().with_latin(latin == Some(true)));
        let (ran, _) = run(&mut d, &km, &Active::NONE, frame(1.0, CTRL, vec![at(key, CTRL)]));
        assert_eq!(ran, [expected], "{name}, reported");
    }
}

/// A press as a layout reports it (logical key, physical position, held modifiers) with the text
/// it typed after it, as egui-winit sends them.
fn typed(key: EguiKey, physical: EguiKey, modifiers: Modifiers, text: &str) -> Vec<Event> {
    let mut events = vec![Event::Key { key, physical_key: Some(physical), pressed: true, repeat: false, modifiers }];
    if !text.is_empty() {
        events.push(Event::Text(text.into()));
    }
    events
}

fn layout_keymap() -> Keymap {
    let app = |id: &str, key: &str| Bindable::new(id, id, Scope::app()).with_default(Binding::parse(key).unwrap());
    let catalog = vec![
        app("brush.smaller", "["),
        app("brush.bigger", "]"),
        app("brush.harder", "Shift+]"),
        app("view.zoomIn", "="),
        app("view.zoomOut", "-"),
        app("select.grow", "Cmd+Shift+."),
    ];
    Keymap::new(catalog, Scopes::new(), Platform::Windows)
}

fn ran(d: &mut Dispatcher, km: &Keymap, time: f64, modifiers: Modifiers, events: Vec<Event>) -> Vec<String> {
    run(d, km, &Active::NONE, frame(time, modifiers, events)).0
}

#[test]
fn german_keys_are_brackets_and_plus_is_not_a_brush_key() {
    let km = layout_keymap();
    let mut d = Dispatcher::new();
    // ö arrives as the physical `;`, # as the physical `\`, and ⇧# types `'`.
    assert_eq!(ran(&mut d, &km, 1.0, Modifiers::NONE, typed(EguiKey::Semicolon, EguiKey::Semicolon, Modifiers::NONE, "ö")), ["brush.smaller"]);
    assert_eq!(ran(&mut d, &km, 2.0, Modifiers::NONE, typed(EguiKey::Backslash, EguiKey::Backslash, Modifiers::NONE, "#")), ["brush.bigger"]);
    // ⇧# keeps its Shift: it is ⇧], not ].
    let shift = Modifiers::SHIFT;
    assert_eq!(ran(&mut d, &km, 3.0, shift, typed(EguiKey::Quote, EguiKey::Backslash, shift, "'")), ["brush.harder"]);
    // The + key sits at the US `]` position and is no bracket on a German layout (`+` and `=`
    // stand for each other, so it zooms in).
    assert_eq!(ran(&mut d, &km, 4.0, Modifiers::NONE, typed(EguiKey::Plus, EguiKey::CloseBracket, Modifiers::NONE, "+")), ["view.zoomIn"]);
    // ⇧+ types `*` and arrives as the physical `]`: it zooms in, and ⇧- zooms out.
    assert_eq!(ran(&mut d, &km, 5.0, shift, typed(EguiKey::CloseBracket, EguiKey::CloseBracket, shift, "*")), ["view.zoomIn"]);
    assert_eq!(ran(&mut d, &km, 6.0, shift, typed(EguiKey::Slash, EguiKey::Slash, shift, "_")), ["view.zoomOut"]);
}

#[test]
fn the_bracket_positions_are_brackets_on_layouts_without_a_table_row() {
    let km = layout_keymap();
    // Spanish: `+` at the `]` position runs `]`; AZERTY's dead `^` at `[` runs `[`.
    let mut d = Dispatcher::new();
    assert_eq!(ran(&mut d, &km, 1.0, Modifiers::NONE, typed(EguiKey::Plus, EguiKey::CloseBracket, Modifiers::NONE, "+")), ["brush.bigger"]);
    assert_eq!(ran(&mut d, &km, 2.0, Modifiers::NONE, typed(EguiKey::OpenBracket, EguiKey::OpenBracket, Modifiers::NONE, "^")), ["brush.smaller"]);
    // Dvorak types `[` on the US `-` key; once seen, the `]` position's `=` is zoom, not `]`.
    let mut d = Dispatcher::new();
    assert_eq!(ran(&mut d, &km, 1.0, Modifiers::NONE, typed(EguiKey::OpenBracket, EguiKey::Minus, Modifiers::NONE, "[")), ["brush.smaller"]);
    assert_eq!(ran(&mut d, &km, 2.0, Modifiers::NONE, typed(EguiKey::Equals, EguiKey::CloseBracket, Modifiers::NONE, "=")), ["view.zoomIn"]);
}

#[test]
fn altgr_presses_teach_the_layout_nothing() {
    let km = layout_keymap();
    let mut d = Dispatcher::new();
    let alt_gr =
        |pressed| Event::Key { key: EguiKey::AltRight, physical_key: Some(EguiKey::AltRight), pressed, repeat: false, modifiers: Modifiers::NONE };
    // Spanish: AltGr+` types `[` and egui-winit reports the press without modifiers. It is no
    // real bracket key, so the `+` key at the US `]` position stays `]`.
    let mut events = vec![alt_gr(true)];
    events.extend(typed(EguiKey::OpenBracket, EguiKey::Backtick, Modifiers::NONE, "["));
    assert_eq!(ran(&mut d, &km, 1.0, Modifiers::NONE, events), ["brush.smaller"]);
    let mut events = vec![alt_gr(false)];
    events.extend(typed(EguiKey::Plus, EguiKey::CloseBracket, Modifiers::NONE, "+"));
    assert_eq!(ran(&mut d, &km, 2.0, Modifiers::NONE, events), ["brush.bigger"]);
}

#[test]
fn cmd_chords_on_period_follow_the_key_position() {
    let km = layout_keymap();
    // Nordic: ⇧⌘. types `:`, which egui reports as the logical key.
    let mods = Modifiers { shift: true, ctrl: true, command: true, ..Modifiers::NONE };
    let mut d = Dispatcher::new();
    assert_eq!(ran(&mut d, &km, 1.0, mods, typed(EguiKey::Colon, EguiKey::Period, mods, ":")), ["select.grow"]);
}

fn raw_key(key: EguiKey, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }
}

/// Runs `events` through the dispatcher's raw-input hook and returns the events left.
fn raw_events(d: &Dispatcher, ctx: &egui::Context, typing: bool, events: Vec<Event>) -> Vec<Event> {
    let mut raw = egui::RawInput { events, ..Default::default() };
    d.raw_input(ctx, &mut raw, typing);
    raw.events
}

fn v_presses(events: &[Event], with_command: bool) -> usize {
    events.iter().filter(|e| matches!(e, Event::Key { key: EguiKey::V, pressed: true, modifiers, .. } if modifiers.command || !with_command)).count()
}

#[test]
fn each_paste_key_is_one_press() {
    let d = Dispatcher::new().with_clipboard_events(true);
    let ctx = egui::Context::default();
    let cmd = Modifiers::COMMAND;
    let presses = |events| v_presses(&raw_events(&d, &ctx, false, events), false);
    // A press that reached us, then its release.
    assert_eq!(presses(vec![raw_key(EguiKey::V, true, cmd)]), 1);
    assert_eq!(presses(vec![raw_key(EguiKey::V, false, cmd)]), 0, "the release of a press doesn't paste again");
    // A text paste event whose V is released after Cmd adds nothing; the next image paste still pastes.
    assert_eq!(presses(vec![Event::Paste("x".into())]), 0);
    assert_eq!(presses(vec![Event::ModifiersChanged(Modifiers::NONE), raw_key(EguiKey::V, false, Modifiers::NONE)]), 0);
    assert_eq!(presses(vec![Event::ModifiersChanged(cmd), raw_key(EguiKey::V, false, cmd)]), 1, "image paste");
}

#[test]
fn a_swallowed_paste_press_is_added_once() {
    let d = Dispatcher::new().with_clipboard_events(true);
    let cmd = Modifiers::COMMAND;
    let none = Modifiers::NONE;
    let pastes = |ctx: &egui::Context, events| v_presses(&raw_events(&d, ctx, false, events), true);
    // Cmd down, (the swallowed V press,) Cmd up, V up: in one frame or over several.
    let ctx = egui::Context::default();
    assert_eq!(pastes(&ctx, vec![Event::ModifiersChanged(cmd), Event::ModifiersChanged(none), raw_key(EguiKey::V, false, none)]), 1);
    assert_eq!(pastes(&ctx, vec![Event::ModifiersChanged(cmd)]), 0);
    assert_eq!(pastes(&ctx, vec![Event::ModifiersChanged(none)]), 0);
    assert_eq!(pastes(&ctx, vec![raw_key(EguiKey::V, false, none)]), 1, "released after Cmd");
    assert_eq!(pastes(&ctx, vec![raw_key(EguiKey::V, false, none)]), 0, "one swallowed press is one paste");
    // A plain V tap after Cmd was used is a tool key, not a paste.
    let ctx = egui::Context::default();
    assert_eq!(pastes(&ctx, vec![Event::ModifiersChanged(cmd), Event::ModifiersChanged(none)]), 0);
    assert_eq!(pastes(&ctx, vec![raw_key(EguiKey::V, true, none), raw_key(EguiKey::V, false, none)]), 0);
    // Without Cmd at all, a lone V release is nothing.
    let ctx = egui::Context::default();
    assert_eq!(pastes(&ctx, vec![raw_key(EguiKey::V, false, none)]), 0);
    // Released with Cmd still held it pastes.
    assert_eq!(pastes(&ctx, vec![Event::ModifiersChanged(cmd), raw_key(EguiKey::V, false, cmd)]), 1);
    // egui 0.37's image-only clipboard paste is the same Cmd+V press.
    let image = std::sync::Arc::new(egui::ColorImage::filled([1, 1], egui::Color32::BLACK));
    let ctx = egui::Context::default();
    assert_eq!(pastes(&ctx, vec![Event::PasteImage(image)]), 1);
}

#[test]
fn a_swallowed_paste_reaches_the_paste_binding_and_text_fields_keep_theirs() {
    let km = dispatch_keymap();
    let ctx = egui::Context::default();
    let mut d = Dispatcher::new().with_clipboard_events(true);
    let image = std::sync::Arc::new(egui::ColorImage::filled([1, 1], egui::Color32::BLACK));
    let events = raw_events(&d, &ctx, false, vec![Event::PasteImage(image.clone())]);
    let (ran, _) = run(&mut d, &km, &Active::NONE, frame(1.0, CTRL, events));
    assert_eq!(ran, ["edit.paste"]);
    // While typing the hook leaves the events to the text field, and with clipboard events off it does nothing.
    assert_eq!(raw_events(&d, &ctx, true, vec![Event::PasteImage(image.clone())]), [Event::PasteImage(image.clone())]);
    let off = Dispatcher::new();
    assert_eq!(raw_events(&off, &ctx, false, vec![Event::PasteImage(image.clone())]), [Event::PasteImage(image)]);
}

#[test]
fn insert_and_delete_chords_copy_paste_and_cut_on_linux() {
    let d = Dispatcher::new().with_clipboard_events(true);
    let ctx = egui::Context::default();
    ctx.set_os(egui::os::OperatingSystem::Nix);
    let after = |key, modifiers, typing| {
        let mut raw = egui::RawInput { events: vec![raw_key(key, true, modifiers)], ..Default::default() };
        d.raw_input(&ctx, &mut raw, typing);
        raw.events
    };
    let is_press = |events: &[Event], key| matches!(events, [Event::Key { key: k, pressed: true, modifiers, .. }] if *k == key && modifiers.command);
    // Outside text fields Ctrl+Insert and Shift+Insert are the Copy and Paste chords.
    assert!(is_press(&after(EguiKey::Insert, Modifiers::CTRL, false), EguiKey::C));
    assert!(is_press(&after(EguiKey::Insert, Modifiers::SHIFT, false), EguiKey::V));
    // Outside text fields Shift+Delete stays itself (Fill in Photoshop).
    assert!(
        matches!(after(EguiKey::Delete, Modifiers::SHIFT, false).as_slice(), [Event::Key { key: EguiKey::Delete, modifiers, .. }] if modifiers.shift)
    );
    // In a text field Ctrl+Insert and Shift+Delete are egui's events; Shift+Insert asks for the text.
    assert_eq!(after(EguiKey::Insert, Modifiers::CTRL, true), [Event::Copy]);
    assert_eq!(after(EguiKey::Delete, Modifiers::SHIFT, true), [Event::Cut]);
    assert!(after(EguiKey::Insert, Modifiers::SHIFT, true).is_empty());
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let mut raw = egui::RawInput { events: vec![raw_key(EguiKey::Insert, true, Modifiers::SHIFT)], ..Default::default() };
        d.raw_input(ui.ctx(), &mut raw, true);
    });
    out.textures_delta.clear();
    let asked = out.viewport_output.values().flat_map(|v| &v.commands).any(|c| matches!(c, egui::ViewportCommand::RequestPaste));
    assert!(asked, "the window is asked for the clipboard text");
    // Plain Insert, other modifiers, and the other platforms are left alone.
    assert!(matches!(after(EguiKey::Insert, Modifiers::NONE, false).as_slice(), [Event::Key { key: EguiKey::Insert, .. }]));
    assert!(matches!(after(EguiKey::Delete, Modifiers::SHIFT | Modifiers::CTRL, false).as_slice(), [Event::Key { key: EguiKey::Delete, .. }]));
    ctx.set_os(egui::os::OperatingSystem::Mac);
    assert!(matches!(after(EguiKey::Insert, Modifiers::CTRL, false).as_slice(), [Event::Key { key: EguiKey::Insert, .. }]));
}
