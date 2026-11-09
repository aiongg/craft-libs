use super::kit::{AppCommand, Dock, DockCommand, Field, Opens, PANE, PaneCommand, Tap, WORKSPACE, WorkspaceCommand};
use craft_keymap::Overrides;
use craft_ui::contract::{CommandSet as _, Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::commands::{
    CommandGate, CommandInput, CommandKind, CommandRef, CommandState, Commands as _, EditCommands, FirePhase, FireSource, Fired, fire, resolve,
};
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Ui};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

fn ids(fired: &[Fired]) -> Vec<&str> {
    fired.iter().map(|f| f.command.id.as_str()).collect()
}

/// Hovers, presses and releases at `at` over three frames and settles for one; the first pass of the release
/// frame asks for a discard pass when `discard` is set.
fn click_at(frames: &mut Frames, at: Pos2, show: &mut impl FnMut(&mut Ui), discard: bool) {
    frames.frame(vec![moved(at)], &mut *show);
    frames.frame(vec![press(at)], &mut *show);
    let mut first = discard;
    frames.frame(vec![release(at)], |ui| {
        show(ui);
        if std::mem::take(&mut first) {
            ui.ctx().request_discard("command trial");
        }
    });
    frames.frame(vec![], &mut *show);
}

fn tap_center(frames: &mut Frames, command: AppCommand) -> Pos2 {
    let mut center = Pos2::ZERO;
    frames.idle(2, |ui| center = Tap::new().id_salt("go").command(command).show(ui).rect.center());
    center
}

/// Prototype: `discarded_pass_delivers_a_click_and_an_edit_once` (the click).
#[trial]
fn a_command_fired_in_a_discarded_pass_is_delivered_once() {
    let mut frames = Frames::new();
    let at = tap_center(&mut frames, AppCommand::Go);
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").command(AppCommand::Go).show(ui);
    };
    click_at(&mut frames, at, &mut show, true);
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["trial.go"]);
    frames.idle(2, &mut show);
    assert!(take_output(&frames.ctx).fired.is_empty(), "nothing fires again in later frames");
}

#[trial]
fn a_disabled_command_does_not_fire() {
    let mut frames = Frames::new();
    let at = tap_center(&mut frames, AppCommand::Go);
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").command(AppCommand::Go).show(ui);
    };
    frames.commands(CommandInput::new().state(AppCommand::Go, CommandState::new().enabled(false)));
    frames.idle(2, &mut show);
    click_at(&mut frames, at, &mut show, false);
    assert!(take_output(&frames.ctx).fired.is_empty(), "the disabled command stays unfired");
    frames.commands(CommandInput::new());
    frames.idle(2, &mut show);
    click_at(&mut frames, at, &mut show, false);
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["trial.go"], "enabled again, it fires");
}

#[trial]
fn a_shortcut_fires_its_command_through_the_gate() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").show(ui);
    };
    frames.idle(2, &mut show);
    let save = || key_press(Key::K, Modifiers::COMMAND);
    frames.frame(vec![save()], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(ids(&fired), ["trial.save"]);
    assert_eq!((fired[0].source, fired[0].target), (FireSource::Key, None));

    frames.commands(CommandInput::new().gate(CommandGate::new(|command: &CommandRef| command.id != "trial.save")));
    frames.frame(vec![save()], &mut show);
    assert!(take_output(&frames.ctx).fired.is_empty(), "the gate refuses the command");

    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new().enabled(false)));
    frames.frame(vec![save()], &mut show);
    assert!(take_output(&frames.ctx).fired.is_empty(), "a disabled command does not fire from its shortcut");
}

/// A key equivalent the platform's menu took reaches the application as the key press: its command fires from a key.
#[trial]
fn a_registered_key_equivalent_fires_its_command() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").show(ui);
    };
    frames.idle(2, &mut show);
    let chord = craft_keymap::Chord::new(craft_keymap::Mods::CMD, craft_keymap::Key::K);
    craft_ui::runtime::windows::WindowHost::of(&frames.ctx)
        .key_equivalent(egui::ViewportId::ROOT, craft_ui::runtime::windows::KeyEquivalent::Press(chord));
    frames.frame(vec![], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(ids(&fired), ["trial.save"]);
    assert_eq!(fired[0].source, FireSource::Key);
}

#[trial]
fn a_command_bound_component_takes_the_commands_label_and_state() {
    let mut frames = Frames::new();
    let seen = RefCell::new((String::new(), true));
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("save").command(AppCommand::Save).seen(&seen).show(ui);
    };
    frames.idle(2, &mut show);
    assert_eq!(*seen.borrow(), ("Save".to_owned(), true), "the command's label, enabled by default");
    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new().title("Save Copy").enabled(false)));
    frames.idle(2, &mut show);
    assert_eq!(*seen.borrow(), ("Save Copy".to_owned(), false), "the state's title and enabled flag reach the component's step 1");
}

/// A click in one pane reaches that pane's body next pass; each pane reads its own state.
#[trial]
fn a_scoped_command_goes_to_the_instance_it_was_fired_in() {
    let mut frames = Frames::new();
    let taken = RefCell::new(Vec::<(&'static str, Fired)>::new());
    let mut show = |ui: &mut Ui| {
        for (name, enabled) in [("a", true), ("b", false)] {
            super::kit::Pane::new().id_salt(name).show(ui, |ui, cx| {
                while let Some((PaneCommand::ZoomIn, fired)) = cx.take_command::<PaneCommand>() {
                    taken.borrow_mut().push((name, fired));
                }
                cx.publish(PaneCommand::ZoomIn, CommandState::new().enabled(enabled));
                Tap::new().id_salt("zoom").command(PaneCommand::ZoomIn).show(ui);
            });
        }
    };
    frames.idle(3, &mut show);
    // The two panes' buttons are laid out one under the other; the second pane's command is disabled.
    let mut centers = Vec::new();
    frames.frame(vec![], |ui| {
        centers.clear();
        for (name, _) in [("a", true), ("b", false)] {
            super::kit::Pane::new()
                .id_salt(name)
                .show(ui, |ui, _| centers.push(Tap::new().id_salt("zoom").command(PaneCommand::ZoomIn).show(ui).rect.center()));
        }
    });
    click_at(&mut frames, centers[1], &mut show, false);
    frames.idle(2, &mut show);
    assert!(taken.borrow().is_empty(), "pane b published the command disabled, so its button does not fire");
    click_at(&mut frames, centers[0], &mut show, false);
    frames.idle(2, &mut show);
    let taken = taken.borrow();
    assert_eq!(taken.len(), 1, "taken once");
    assert_eq!(taken[0].0, "a");
    assert!(taken[0].1.target.is_some());
    assert!(take_output(&frames.ctx).fired.is_empty(), "a scoped command is not an app command");
}

/// A command fired from a surface with no component goes to the first live instance, and not at all without one.
#[trial]
fn a_surface_fires_a_scoped_command_to_a_live_instance_only() {
    let mut frames = Frames::new();
    let taken = RefCell::new(Vec::<&'static str>::new());
    let with_panes = std::cell::Cell::new(true);
    let mut show = |ui: &mut Ui| {
        if !with_panes.get() {
            return;
        }
        for name in ["a", "b"] {
            super::kit::Pane::new().id_salt(name).show(ui, |_, cx| {
                while cx.take_command::<PaneCommand>().is_some() {
                    taken.borrow_mut().push(name);
                }
            });
        }
    };
    frames.idle(3, &mut show);
    assert!(fire(&frames.ctx, PaneCommand::ZoomIn, FireSource::Palette));
    frames.idle(2, &mut show);
    assert_eq!(*taken.borrow(), ["a"], "with no key focus the first live instance takes it");

    with_panes.set(false);
    frames.idle(3, &mut show);
    assert!(!fire(&frames.ctx, PaneCommand::ZoomIn, FireSource::Palette), "no live instance, nothing to deliver");
    assert_eq!(PANE.name(), "trial.pane");
}

/// Step 17: a command-bound component reports no click and fires its command instead.
#[trial]
fn a_command_bound_click_fires_instead_of_reporting_a_click() {
    let mut frames = Frames::new();
    let at = tap_center(&mut frames, AppCommand::Go);
    let mut clicked = false;
    let mut show = |ui: &mut Ui| {
        clicked |= craft_ui::testing::contract::clicked(&Tap::new().id_salt("go").command(AppCommand::Go).show(ui));
    };
    click_at(&mut frames, at, &mut show, false);
    assert!(!clicked, "no click query reports true");
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["trial.go"], "and the command fired once");
}

fn focus_on(frames: &mut Frames, id: egui::Id, show: &mut impl FnMut(&mut Ui)) {
    request(&frames.ctx, FocusTarget::Id(id));
    frames.frame(vec![], &mut *show);
}

/// 9.3: a key the focused widget activates with is not also a shortcut.
#[trial]
fn a_shortcut_does_not_fire_on_a_key_the_focused_widget_activates_with() {
    let at = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        let response = Tap::new().id_salt("go").command(AppCommand::Go).show(ui);
        at.set(Some(response.id));
    };
    // Play is bound to Space for these frames only: the binary's registry binds no app command to Space, which the
    // canvas's momentary Hand takes when the focus is outside every canvas.
    let play_on_space = || {
        let mut overrides = Overrides::default();
        overrides.insert(AppCommand::Play.id(), vec!["Space".to_owned()]);
        let mut frames = Frames::new();
        frames.commands(CommandInput::new().overrides(Arc::new(overrides)));
        frames
    };
    let mut frames = play_on_space();
    frames.idle(2, &mut show);
    focus_on(&mut frames, at.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::Space, Modifiers::NONE)], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(ids(&fired), ["trial.go"], "the button's activation fires, and the Space shortcut does not");
    assert_eq!(fired[0].source, FireSource::Pointer);

    let mut frames = play_on_space();
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::Space, Modifiers::NONE)], &mut show);
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["trial.play"], "with nothing focused the shortcut fires");
}

/// 11.3: a scoped shortcut goes to the instance that holds key focus.
#[trial]
fn a_scoped_shortcut_goes_to_the_instance_that_holds_focus() {
    let taps = RefCell::new(Vec::<egui::Id>::new());
    let taken = RefCell::new(Vec::<&'static str>::new());
    let mut show = |ui: &mut Ui| {
        taps.borrow_mut().clear();
        for name in ["a", "b"] {
            super::kit::Pane::new().id_salt(name).show(ui, |ui, cx| {
                while cx.take_command::<PaneCommand>().is_some() {
                    taken.borrow_mut().push(name);
                }
                taps.borrow_mut().push(Tap::new().id_salt("t").show(ui).id);
            });
        }
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    let second = taps.borrow()[1];
    focus_on(&mut frames, second, &mut show);
    frames.frame(vec![key_press(Key::K, Modifiers::COMMAND | Modifiers::ALT)], &mut show);
    assert_eq!(*taken.borrow(), ["b"], "the second pane holds focus, so it takes the command, not the first");
}

/// 9.3 and 11.3: with focus in a pane inside a dock, the dock's shortcut beats an application shortcut on the same
/// chord; with focus outside, the application's does.
#[trial]
fn an_enclosing_scopes_shortcut_beats_the_applications() {
    let tap = Cell::new(None);
    let closed = Cell::new(0);
    let outside = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        Dock::new().id_salt("dock").show(ui, |ui, cx| {
            while cx.take_command::<DockCommand>().is_some() {
                closed.set(closed.get() + 1);
            }
            super::kit::Pane::new().id_salt("pane").show(ui, |ui, _| tap.set(Some(Tap::new().id_salt("in").show(ui).id)));
        });
        outside.set(Some(Tap::new().id_salt("out").show(ui).id));
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    focus_on(&mut frames, tap.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::J, Modifiers::COMMAND | Modifiers::ALT)], &mut show);
    assert_eq!((closed.get(), take_output(&frames.ctx).fired.len()), (1, 0), "the dock takes it");
    focus_on(&mut frames, outside.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::J, Modifiers::COMMAND | Modifiers::ALT)], &mut show);
    assert_eq!((closed.get(), ids(&take_output(&frames.ctx).fired)), (1, vec!["trial.jump"]), "outside the dock the application's command fires");
}

/// 11.3: an edit command fired from a surface reaches the focused component that lists it, and stays an app command
/// otherwise.
#[trial]
fn an_edit_command_from_a_surface_reaches_the_focused_component() {
    let log = RefCell::new(Vec::<String>::new());
    let field = Cell::new(None);
    let mut show = |ui: &mut Ui| field.set(Some(Field::new(&log).id_salt("f").show(ui).id));
    let mut frames = Frames::new();
    frames.idle(2, &mut show);
    assert!(fire(&frames.ctx, EditCommands::Copy, FireSource::Menu));
    frames.idle(2, &mut show);
    assert!(log.borrow().is_empty(), "nothing is focused");
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["edit.copy"], "so Copy stays an app command");

    focus_on(&mut frames, field.get().unwrap(), &mut show);
    assert!(fire(&frames.ctx, EditCommands::Copy, FireSource::Menu));
    frames.idle(2, &mut show);
    assert_eq!(*log.borrow(), ["Copy"]);
    assert!(take_output(&frames.ctx).fired.is_empty(), "the focused field took it");
}

/// 11.3: the paste text reaches the component that takes `Intent::Edit(Paste)`.
#[trial]
fn the_paste_text_reaches_the_component_that_takes_the_paste() {
    let log = RefCell::new(Vec::<String>::new());
    let field = Cell::new(None);
    let mut show = |ui: &mut Ui| field.set(Some(Field::new(&log).id_salt("f").show(ui).id));
    let mut frames = Frames::new();
    frames.idle(2, &mut show);
    focus_on(&mut frames, field.get().unwrap(), &mut show);
    frames.frame(vec![egui::Event::Paste("hello".into())], &mut show);
    assert_eq!(*log.borrow(), ["Paste", "text hello"]);
}

/// 10.4: a component bound to a toggle command shows its `checked` state.
#[trial]
fn a_toggle_command_bound_component_shows_the_checked_state() {
    let on = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("grid").command(AppCommand::Grid).on(&on).show(ui);
    };
    let mut frames = Frames::new();
    frames.commands(CommandInput::new().state(AppCommand::Grid, CommandState::new().checked(true)));
    frames.idle(2, &mut show);
    assert!(on.get());
    frames.commands(CommandInput::new().state(AppCommand::Grid, CommandState::new().checked(false)));
    frames.idle(2, &mut show);
    assert!(!on.get());
}

/// 11.3: a surface resolves a command against the instance a fire would reach.
#[trial]
fn a_surface_resolves_a_command_against_its_target() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        for (name, enabled) in [("a", false), ("b", true)] {
            super::kit::Pane::new().id_salt(name).show(ui, |_, cx| cx.publish(PaneCommand::ZoomIn, CommandState::new().enabled(enabled)));
        }
    };
    frames.idle(3, &mut show);
    let zoom = resolve(&frames.ctx, PaneCommand::ZoomIn);
    assert!(!zoom.enabled, "the first live instance is the target, and it published the command disabled");
    assert_eq!(&*zoom.label, "Zoom In");
    assert!(zoom.chord.is_some());
    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new().enabled(false)));
    frames.idle(2, &mut show);
    assert!(!resolve(&frames.ctx, AppCommand::Save).enabled);
    assert_eq!(resolve(&frames.ctx, AppCommand::Grid).kind, CommandKind::Toggle);
    assert_eq!(resolve(&frames.ctx, AppCommand::Brush).kind, CommandKind::Tool { group: Some("tools".into()) });
}

/// A command whose state is marked resolves marked, with the state's hint as the marker's description.
#[trial]
fn a_marked_command_resolves_marked() {
    let mut frames = Frames::new();
    frames.commands(CommandInput::new().state(AppCommand::Save, CommandState::new().marked(true).hint("Two comments")));
    frames.idle(2, |_| {});
    let save = resolve(&frames.ctx, AppCommand::Save);
    assert!(save.marked);
    assert_eq!(save.hint.as_deref(), Some("Two comments"));
    assert!(!resolve(&frames.ctx, AppCommand::Grid).marked, "a command with no state is not marked");
}

/// 1.8 and 7.2: the gate is app code, so it runs with no runtime lock held and may use the board.
#[trial]
fn the_gate_may_resolve_commands() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let inside = std::sync::atomic::AtomicBool::new(false);
    frames.commands(CommandInput::new().gate(CommandGate::new(move |_: &CommandRef| {
        // The nested `resolve` runs the gate again, which stops here.
        inside.swap(true, std::sync::atomic::Ordering::SeqCst) || {
            let enabled = resolve(&ctx, AppCommand::Go).enabled;
            inside.store(false, std::sync::atomic::Ordering::SeqCst);
            enabled
        }
    })));
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").command(AppCommand::Save).show(ui);
    };
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::K, Modifiers::COMMAND)], &mut show);
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["trial.save"]);
}

/// 9.2 rule 3: a secondary release and an untaken `ContextMenu` intent record a context request.
#[trial]
fn a_context_request_is_recorded_for_the_component() {
    let seen = Cell::new(false);
    let at = Cell::new(Pos2::ZERO);
    let tap = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        let response = Tap::new().id_salt("t").show(ui);
        at.set(response.rect.center());
        tap.set(Some(response.id));
        seen.set(seen.get() || craft_ui::testing::focus::context_request(ui.ctx(), response.id).is_some());
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    frames.frame(vec![moved(at.get())], &mut show);
    frames.frame(vec![craft_ui::testing::pointer::secondary(at.get(), true)], &mut show);
    assert!(!seen.get(), "a press alone asks for nothing");
    frames.frame(vec![craft_ui::testing::pointer::secondary(at.get(), false)], &mut show);
    assert!(seen.get(), "the secondary release asks for a context menu");

    seen.set(false);
    focus_on(&mut frames, tap.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::F10, Modifiers::SHIFT)], &mut show);
    assert!(seen.get(), "Shift+F10 on a component that does not take it asks for a context menu");
}

/// 9.2 rule 3: the context request exists when the component's overlay step runs.
#[trial]
fn a_context_request_is_there_for_the_components_own_overlay_step() {
    let seen = Cell::new(false);
    let at = Cell::new(Pos2::ZERO);
    let mut show = |ui: &mut Ui| {
        at.set(Opens::new(&seen, ui.ctx().clone()).id_salt("o").show(ui).rect.center());
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    frames.frame(vec![moved(at.get())], &mut show);
    frames.frame(vec![craft_ui::testing::pointer::secondary(at.get(), true)], &mut show);
    frames.frame(vec![craft_ui::testing::pointer::secondary(at.get(), false)], &mut show);
    assert!(seen.get(), "the overlay step reads the request of its own pass");
}

/// 11.3: a momentary command fires on the press and again on the release.
#[trial]
fn a_momentary_command_fires_on_press_and_on_release() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        Tap::new().id_salt("go").show(ui);
    };
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::H, Modifiers::NONE)], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!((ids(&fired), fired[0].phase), (vec!["trial.hold"], FirePhase::Press));
    frames.frame(vec![craft_ui::testing::focus::key_release(Key::H, Modifiers::NONE)], &mut show);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!((ids(&fired), fired[0].phase), (vec!["trial.hold"], FirePhase::Release));
}

/// 11.3: a scoped entry waits for its instance and is dropped when the instance stops being live.
#[trial]
fn an_untaken_scoped_entry_is_dropped_with_its_instance() {
    let panes = Cell::new(true);
    let taking = Cell::new(false);
    let taken = Cell::new(0);
    let mut show = |ui: &mut Ui| {
        if panes.get() {
            super::kit::Pane::new().id_salt("a").show(ui, |_, cx| {
                if taking.get() {
                    taken.set(taken.get() + usize::from(cx.take_command::<PaneCommand>().is_some()));
                }
            });
        }
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    assert!(fire(&frames.ctx, PaneCommand::ZoomIn, FireSource::Palette));
    panes.set(false);
    frames.idle(3, &mut show);
    panes.set(true);
    taking.set(true);
    frames.idle(3, &mut show);
    assert_eq!(taken.get(), 0, "the entry died with its instance");
}

/// Core 9.2: `Move` and `Size` have no default chord on macOS, where the title-row menu offers them.
#[trial]
fn window_move_and_size_have_no_default_chord_on_macos() {
    use craft_host::target::Os;
    use craft_ui::runtime::commands::WindowCommand;
    use craft_ui::testing::Fixture;
    let mut fixture = Fixture::new(|_| {});
    let chords = |fixture: &mut Fixture, os: Os| {
        fixture.os(os).frame(vec![]);
        let chord = |command| resolve(fixture.ctx(), command).chord;
        (chord(WindowCommand::Move), chord(WindowCommand::Size))
    };
    assert_eq!(chords(&mut fixture, Os::Mac), (None, None), "no default chord on macOS");
    let (r#move, size) = chords(&mut fixture, Os::Windows);
    assert_eq!(
        (r#move.map(|c| c.key), size.map(|c| c.key)),
        (Some(craft_keymap::Key::F7), Some(craft_keymap::Key::F8)),
        "Alt+F7 and Alt+F8 elsewhere"
    );
}

/// 11.4: a modal layer blocks the application's `ModalRule::Declared` edit commands, except for an instance inside it
/// that declares them: a dialog that publishes the undo state takes it, and the command resolves enabled there.
#[trial]
fn undo_in_a_dialog_reaches_the_dialog_and_not_the_application() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::Dialog;
    use craft_ui::runtime::focus::Initial;
    use craft_ui::sections::{DialogRole, FooterButton};

    let undone = Cell::new(0);
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        Dialog::new("Options")
            .opened(Bind::bound(&mut open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept)])
            .show(ui, |_, cx| {
                cx.publish(EditCommands::Undo, CommandState::new());
                while let Some((command, _)) = cx.take_command::<EditCommands>() {
                    if command == EditCommands::Undo {
                        undone.set(undone.get() + 1);
                    }
                }
            });
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    assert!(resolve(&frames.ctx, EditCommands::Undo).enabled, "the declared undo is enabled in the dialog");
    assert!(!resolve(&frames.ctx, EditCommands::Redo).enabled, "the redo the dialog does not declare is not");
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(undone.get(), 1, "the dialog took the undo");
    assert!(take_output(&frames.ctx).fired.is_empty(), "the application's undo stays blocked");
}

/// 11.4: a dialog that takes no edit commands still keeps Cmd+Z from the document behind it, and the native Edit
/// menu shows Undo disabled there.
#[trial]
fn undo_in_a_dialog_that_takes_no_edit_commands_fires_nothing() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::Dialog;
    use craft_ui::runtime::focus::Initial;
    use craft_ui::sections::{DialogRole, FooterButton};

    let mut show = |ui: &mut Ui| {
        let mut open = true;
        Dialog::new("Options")
            .opened(Bind::bound(&mut open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept)])
            .show(ui, |_, _| {});
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    assert!(!resolve(&frames.ctx, EditCommands::Undo).enabled, "undo resolves disabled in a dialog that declares none");
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert!(take_output(&frames.ctx).fired.is_empty(), "no command reaches the application");
}

/// The same pair for a full-window dialog, which is an instance of the view scope.
#[trial]
fn undo_in_a_full_window_dialog_follows_what_the_dialog_declares() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::FullWindowDialog;

    let undone = Cell::new(0);
    let declares = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        FullWindowDialog::new("Liquify").opened(Bind::bound(&mut open)).show(ui, |_, cx| {
            if declares.get() {
                cx.publish(EditCommands::Undo, CommandState::new());
                while let Some((command, _)) = cx.take_command::<EditCommands>() {
                    if command == EditCommands::Undo {
                        undone.set(undone.get() + 1);
                    }
                }
            }
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    assert!(!resolve(&frames.ctx, EditCommands::Undo).enabled, "undo is disabled in a full-window dialog that declares none");
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(undone.get(), 0);
    assert!(take_output(&frames.ctx).fired.is_empty(), "no command reaches the application");

    declares.set(true);
    frames.idle(4, &mut show);
    assert!(resolve(&frames.ctx, EditCommands::Undo).enabled, "undo is enabled once the dialog declares it");
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(undone.get(), 1, "the full-window dialog took the undo");
    assert!(take_output(&frames.ctx).fired.is_empty(), "the application's undo stays blocked");
}

/// 11.4: a declared edit command is delivered to the instance that declares it. A dialog under the top one, whatever
/// it takes, does not see the top dialog's undo.
#[trial]
fn undo_in_stacked_dialogs_reaches_only_the_dialog_that_declares_it() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::Dialog;
    use craft_ui::runtime::focus::Initial;
    use craft_ui::sections::{DialogRole, FooterButton};

    let (lower, top) = (Cell::new(0), Cell::new(0));
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        Dialog::new("Preferences")
            .opened(Bind::bound(&mut open))
            .buttons([FooterButton::new("Done", DialogRole::Accept)])
            .show(ui, |_, cx| lower.set(lower.get() + usize::from(cx.take_command::<EditCommands>().is_some())));
        let mut open = true;
        Dialog::new("Shortcuts")
            .opened(Bind::bound(&mut open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("Done", DialogRole::Accept)])
            .show(ui, |_, cx| {
                cx.publish(EditCommands::Undo, CommandState::new());
                top.set(top.get() + usize::from(matches!(cx.take_command::<EditCommands>(), Some((EditCommands::Undo, _)))));
            });
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!((top.get(), lower.get()), (1, 0), "only the declaring dialog took the undo");
    assert!(take_output(&frames.ctx).fired.is_empty(), "the application's undo stays blocked");
}

/// 11.3: a focused component that lists the edit takes it from its own history, whatever the application says about
/// the command.
#[trial]
fn a_focused_field_in_a_dialog_takes_undo_whatever_the_application_says() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::Dialog;
    use craft_ui::sections::{DialogRole, FooterButton};

    let log = RefCell::new(Vec::<String>::new());
    let field = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        Dialog::new("Rename").opened(Bind::bound(&mut open)).buttons([FooterButton::new("OK", DialogRole::Accept)]).show(ui, |ui, _| {
            field.set(Some(Field::new(&log).id_salt("f").show(ui).id));
        });
    };
    let mut frames = Frames::new();
    frames.commands(CommandInput::new().state(EditCommands::Undo, CommandState::new().enabled(false)));
    frames.idle(4, &mut show);
    focus_on(&mut frames, field.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(*log.borrow(), ["Undo"], "the field took the undo");
    assert!(take_output(&frames.ctx).fired.is_empty(), "the application did not");
}

/// 16.5: a full-window dialog with its own scope receives that scope's chords and the view commands, with the focus
/// on no control of the body.
#[trial]
fn a_full_window_dialog_scope_chains_under_the_view_scope() {
    use craft_ui::canvas::ViewCommands;
    use craft_ui::contract::Bind;
    use craft_ui::dialog::FullWindowDialog;

    let (zoomed, brushed) = (Cell::new(0), Cell::new(0));
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        FullWindowDialog::new("Workspace").opened(Bind::bound(&mut open)).command_scope(WORKSPACE).show(ui, |_, cx| {
            while let Some((command, _)) = cx.take_command::<ViewCommands>() {
                zoomed.set(zoomed.get() + usize::from(command == ViewCommands::ZoomIn));
            }
            while cx.take_command::<WorkspaceCommand>().is_some() {
                brushed.set(brushed.get() + 1);
            }
        });
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    frames.frame(vec![key_press(Key::Equals, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(zoomed.get(), 1, "the zoom chord reaches the dialog");
    frames.frame(vec![key_press(Key::W, Modifiers::COMMAND | Modifiers::ALT)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(brushed.get(), 1, "the workspace chord reaches it too");
}

/// A modal full-window dialog with nothing focused routes a plain letter to its own scope; once a stop is focused, a
/// plain Enter still submits it.
#[trial]
fn a_modal_full_window_dialog_with_nothing_focused_takes_its_letters_and_enter() {
    use craft_ui::contract::Bind;
    use craft_ui::dialog::FullWindowDialog;
    use craft_ui::sections::{DialogRole, FooterButton};

    let (nudged, closed) = (Cell::new(0), Cell::new(false));
    let mut show = |ui: &mut Ui| {
        let mut open = true;
        let shown = FullWindowDialog::new("Workspace")
            .opened(Bind::bound(&mut open))
            .command_scope(WORKSPACE)
            .buttons([FooterButton::new("OK", DialogRole::Accept)])
            .show(ui, |_, cx| {
                while let Some((command, _)) = cx.take_command::<WorkspaceCommand>() {
                    nudged.set(nudged.get() + usize::from(command == WorkspaceCommand::Nudge));
                }
            });
        closed.set(closed.get() || shown.output.closed.is_some());
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    frames.frame(vec![key_press(Key::N, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(nudged.get(), 1, "the scope's letter reaches the dialog with nothing focused");
    assert!(!closed.get(), "a letter does not submit");
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    assert!(closed.get(), "a plain Enter submits the dialog");
}

/// 11.3: a focused field takes the edit commands for its own text wherever it is; with none focused the app takes them.
#[trial]
fn a_focused_field_outside_a_dialog_takes_undo_and_the_app_gets_none() {
    let log = RefCell::new(Vec::<String>::new());
    let field = Cell::new(None);
    let mut show = |ui: &mut Ui| field.set(Some(Field::new(&log).id_salt("f").show(ui).id));
    let mut frames = Frames::new();
    frames.commands(CommandInput::new().state(EditCommands::Undo, CommandState::new()));
    frames.idle(3, &mut show);
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(ids(&take_output(&frames.ctx).fired), ["edit.undo"], "with no field focused the app takes the undo");

    focus_on(&mut frames, field.get().unwrap(), &mut show);
    frames.frame(vec![key_press(Key::Z, Modifiers::COMMAND)], &mut show);
    frames.idle(2, &mut show);
    assert_eq!(*log.borrow(), ["Undo"], "the focused field took the undo");
    assert!(take_output(&frames.ctx).fired.is_empty(), "the app got none");
}
