//! App commands in an `AppShell`: a body takes the app sets it lists; the ones no body takes stay in the frame's
//! output.

use super::kit::{AppCommand, Tap};
use craft_ui::contract::{Props as _, Show as _};
use craft_ui::runtime::commands::Commands as _;
use craft_ui::runtime::env::WindowKey;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::runtime::frame::take_output;
use craft_ui::shell::AppShell;
use craft_ui::testing::Frames;
use craft_ui::testing::input::key_press;
use egui::{Id, Key, Modifiers, Ui};
use std::cell::{Cell, RefCell};

#[craft_ui::testing::trial]
fn an_app_command_pressed_with_the_focus_in_a_shell_body_is_taken_by_the_body_and_not_left_in_the_output() {
    let tap = Cell::new(Id::NULL);
    let taken = RefCell::new(Vec::new());
    let mut show = |ui: &mut Ui| {
        AppShell::new(WindowKey::main())
            .body(|ui, cx| {
                while let Some((command, _)) = cx.take_command::<AppCommand>() {
                    taken.borrow_mut().push(command);
                }
                tap.set(Tap::new().id_salt("in").show(ui).id);
            })
            .show(ui);
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut show);
    request(&frames.ctx, FocusTarget::Id(tap.get()));
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::K, Modifiers::COMMAND)], &mut show);
    assert_eq!(*taken.borrow(), [AppCommand::Save], "the body took the chord's command");
    assert!(take_output(&frames.ctx).fired.is_empty(), "a command the body took is not in the frame's output");
}

#[craft_ui::testing::trial]
fn an_app_command_no_shell_body_takes_reaches_the_output_and_so_does_one_in_a_window_without_a_shell() {
    let tap = Cell::new(Id::NULL);
    let mut shell = |ui: &mut Ui| {
        AppShell::new(WindowKey::main()).body(|ui, _| tap.set(Tap::new().id_salt("in").show(ui).id)).show(ui);
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut shell);
    request(&frames.ctx, FocusTarget::Id(tap.get()));
    frames.frame(vec![], &mut shell);
    frames.frame(vec![key_press(Key::K, Modifiers::COMMAND)], &mut shell);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(fired.iter().map(|f| f.command.id.as_str()).collect::<Vec<_>>(), ["trial.save"], "no body took it, so the app gets it");

    let mut bare = |ui: &mut Ui| {
        Tap::new().id_salt("in").show(ui);
    };
    let mut frames = Frames::new();
    frames.idle(3, &mut bare);
    frames.frame(vec![key_press(Key::K, Modifiers::COMMAND)], &mut bare);
    let fired = take_output(&frames.ctx).fired;
    assert_eq!(fired.iter().map(|f| f.command.id.as_str()).collect::<Vec<_>>(), ["trial.save"], "a window without a shell fires it to the app");
}
