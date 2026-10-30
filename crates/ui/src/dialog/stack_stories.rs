//! Stories of the dialog stack, prompt, alert and blocking overlay.

use egui::{Id, Ui};

use super::{Alert, BlockingOverlay, ButtonRole, Dialog, DialogStack, DialogState, Footer, Outcome, Prompt, dialog_id};
use crate::story::{Args, Level, Story};

pub(super) fn stories() -> Vec<Story> {
    let stack = || Args::new().with_bool("open", true).with_bool("child open", false);
    let question = || Args::new().with_choice("kind", &["prompt", "alert"], 0).with_bool("open", true).with_bool("translated labels", false);
    let blocking = || Args::new().with_bool("busy for 2 s", false).with_bool("scrim", false);
    vec![
        Story::new("patterns/dialog/stack", Level::Patterns, "Dialog", "Stack", stack, show_stack),
        Story::new("patterns/dialog/prompt", Level::Patterns, "Dialog", "Prompt and alert", question, show_question),
        Story::new("patterns/dialog/blocking", Level::Patterns, "Dialog", "Blocking overlay", blocking, show_blocking),
    ]
}

/// A parent dialog with a button that opens a child: only the child answers keys while it is open.
fn show_stack(ui: &mut Ui, args: &mut Args) {
    let key = Id::unique("craft_ui::dialog::story-stack");
    let (mut stack, mut parent, mut child): (DialogStack, DialogState, DialogState) = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    let ctx = ui.ctx().clone();
    if args.bool("open") {
        stack.push(dialog_id("story-parent"), None);
        if args.bool("child open") {
            stack.push(dialog_id("story-child"), Some(dialog_id("story-parent")));
        }
        let mut child_outcome = None;
        let shown = Dialog::new("story-parent", "Layer Style").stack(&stack).footer(Footer::standard()).show(&ctx, &mut parent, &mut (), |ui, ()| {
            ui.label("Enter and Esc go to the dialog on top.");
            if ui.button("Edit contour").clicked() {
                child_outcome = Some(Outcome::Reset);
            }
            if args.bool("child open") {
                child_outcome = Dialog::new("story-child", "Contour Editor")
                    .stack(&stack)
                    .footer(Footer::standard())
                    .show(&ctx, &mut child, &mut (), |ui, ()| {
                        ui.label("The parent is disabled while this is open.");
                    })
                    .outcome;
            }
        });
        match child_outcome {
            Some(Outcome::Reset) => args.set_bool("child open", true),
            Some(outcome) if outcome.closes() => {
                args.set_bool("child open", false);
                stack.remove(dialog_id("story-child"));
            }
            _ => {}
        }
        if shown.outcome.is_some_and(Outcome::closes) {
            args.set_bool("open", false);
            args.set_bool("child open", false);
            stack.remove_tree(dialog_id("story-parent"));
        }
    }
    ui.data_mut(|d| d.insert_temp(key, (stack, parent, child)));
}

fn show_question(ui: &mut Ui, args: &mut Args) {
    if !args.bool("open") {
        return;
    }
    let ctx = ui.ctx().clone();
    if args.choice("kind") == "alert" {
        if Alert::new("story-alert", "About", "Craft design system story. Enter or Esc closes this notice.").show(&ctx) {
            args.set_bool("open", false);
        }
        return;
    }
    let german = args.bool("translated labels");
    let (yes, no, cancel) = if german { ("Ja", "Nein", "Abbrechen") } else { ("Yes", "No", "Cancel") };
    let answer = Prompt::new("story-prompt", "Unsaved changes", "Do you want to save the changes before closing?")
        .answer(ButtonRole::Default, yes, Some(egui::Key::Y))
        .answer(ButtonRole::Alternate, no, Some(egui::Key::N))
        .answer(ButtonRole::Cancel, cancel, Some(egui::Key::C))
        .show(&ctx);
    if answer.is_some() {
        args.set_bool("open", false);
    }
}

/// The window takes no input for two seconds after the box is ticked.
fn show_blocking(ui: &mut Ui, args: &mut Args) {
    let key = Id::unique("craft_ui::dialog::story-blocking");
    let now = ui.input(|i| i.time);
    let until: Option<f64> = ui.data(|d| d.get_temp(key));
    if args.bool("busy for 2 s") {
        let until = until.unwrap_or(now + 2.0);
        ui.data_mut(|d| d.insert_temp(key, until));
        if now >= until {
            args.set_bool("busy for 2 s", false);
            ui.data_mut(|d| d.remove_temp::<f64>(key));
        } else {
            BlockingOverlay::new("story-busy").scrim(args.bool("scrim")).name("Working").show(ui.ctx());
            ui.ctx().request_repaint();
        }
    }
    ui.label("Tick the box to block the window for two seconds.");
}
