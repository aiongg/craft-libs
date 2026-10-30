//! Stories of the dialog stack, prompt, alert and blocking overlay.

use egui::Ui;

use super::{Alert, BlockingOverlay, ButtonRole, Dialog, DialogStack, DialogState, Footer, Outcome, Prompt, dialog_id};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct StackArgs {
    open: bool,
    #[arg(name = "child open")]
    child_open: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Prompt,
    Alert,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct QuestionArgs {
    kind: KindArg,
    open: bool,
    #[arg(name = "translated labels")]
    translated_labels: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BlockingArgs {
    #[arg(name = "busy for 2 s")]
    busy: bool,
    scrim: bool,
    #[arg(name = "wait card")]
    card: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/dialog/stack", Level::Patterns, "Dialog", "Stack", StackArgs { open: true, child_open: false }, show_stack),
        typed_story!(
            "patterns/dialog/prompt",
            Level::Patterns,
            "Dialog",
            "Prompt and alert",
            QuestionArgs { kind: KindArg::Prompt, open: true, translated_labels: false },
            show_question
        ),
        typed_story!(
            "patterns/dialog/blocking",
            Level::Patterns,
            "Dialog",
            "Blocking overlay",
            BlockingArgs { busy: false, scrim: false, card: false },
            show_blocking
        ),
    ]
}

/// A parent dialog with a button that opens a child: only the child answers keys while it is open.
fn show_stack(ui: &mut Ui, args: &mut StackArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-stack");
    let (mut stack, mut parent, mut child): (DialogStack, DialogState, DialogState) = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    let ctx = ui.ctx().clone();
    if crate::story::dialog_shown(ui, args.open) {
        stack.push(dialog_id("story-parent"), None);
        if args.child_open {
            stack.push(dialog_id("story-child"), Some(dialog_id("story-parent")));
        }
        let mut child_outcome = None;
        let shown = Dialog::new("story-parent", "Layer Style").stack(&stack).footer(Footer::standard()).show(&ctx, &mut parent, &mut (), |ui, ()| {
            ui.label("Enter and Esc go to the dialog on top.");
            if ui.button("Edit contour").clicked() {
                child_outcome = Some(Outcome::Reset);
            }
            if args.child_open {
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
            Some(Outcome::Reset) => args.child_open = true,
            Some(outcome) if outcome.closes() => {
                args.child_open = false;
                stack.remove(dialog_id("story-child"));
            }
            _ => {}
        }
        if shown.outcome.is_some_and(Outcome::closes) {
            crate::story::dialog_closed(ui, &mut args.open);
            args.child_open = false;
            stack.remove_tree(dialog_id("story-parent"));
        }
    }
    ui.data_mut(|d| d.insert_temp(key, (stack, parent, child)));
}

fn show_question(ui: &mut Ui, args: &mut QuestionArgs) {
    if !crate::story::dialog_shown(ui, args.open) {
        return;
    }
    let ctx = ui.ctx().clone();
    if args.kind == KindArg::Alert {
        if Alert::new("story-alert", "About", "Craft design system story. Enter or Esc closes this notice.").show(&ctx) {
            crate::story::dialog_closed(ui, &mut args.open);
        }
        return;
    }
    let german = args.translated_labels;
    let (yes, no, cancel) = if german { ("Ja", "Nein", "Abbrechen") } else { ("Yes", "No", "Cancel") };
    let answer = Prompt::new("story-prompt", "Unsaved changes", "Do you want to save the changes before closing?")
        .answer(ButtonRole::Default, yes, Some(egui::Key::Y))
        .answer(ButtonRole::Alternate, no, Some(egui::Key::N))
        .answer(ButtonRole::Cancel, cancel, Some(egui::Key::C))
        .show(&ctx);
    if answer.is_some() {
        crate::story::dialog_closed(ui, &mut args.open);
    }
}

/// The window takes no input for two seconds after the box is ticked.
fn show_blocking(ui: &mut Ui, args: &mut BlockingArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-blocking");
    let now = ui.input(|i| i.time);
    let until: Option<f64> = ui.data(|d| d.get_temp(key));
    if args.busy {
        let until = until.unwrap_or(now + 2.0);
        ui.data_mut(|d| d.insert_temp(key, until));
        if now >= until {
            args.busy = false;
            ui.data_mut(|d| d.remove_temp::<f64>(key));
        } else {
            let mut overlay = BlockingOverlay::new("story-busy").scrim(args.scrim).name("Working");
            if args.card {
                overlay = overlay.message("Pick a screen pixel or press Esc to cancel").cancel("Cancel");
            }
            if overlay.show(ui.ctx()) {
                args.busy = false;
                ui.data_mut(|d| d.remove_temp::<f64>(key));
            }
            ui.ctx().request_repaint();
        }
    }
    ui.label("Tick the box to block the window for two seconds.");
}
