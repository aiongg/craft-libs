use super::kit::{AppCommand, PANE, PaneCommand, Tap};
use craft_ui::contract::{Props as _, Show as _, ShowContainer as _};
use craft_ui::runtime::commands::{CommandGate, CommandInput, CommandRef, CommandState, Commands as _, FireSource, Fired, fire};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Ui};
use std::cell::RefCell;

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
