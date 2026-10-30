//! Stories of the dialog frame, the modeless dialog and the footer.

use egui::{Id, Ui, vec2};

use super::{Body, Dialog, DialogFrame, DialogState, EnterRule, Footer, Outcome, Reset, Resize, ResizeMax, Width};
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Dialog"];

const WIDTHS: &[&str] = &["compact", "standard", "wide"];

pub(crate) fn stories() -> Vec<Story> {
    let frame = || Args::new().with_choice("width", WIDTHS, 1).with_bool("close box", true).with_bool("roomy", false);
    let modeless =
        || Args::new().with_bool("open", true).with_choice("width", WIDTHS, 1).with_bool("resizable", false).with_bool("scroll body", false);
    let footer = || {
        Args::new()
            .with_bool("open", true)
            .with_choice("shape", &["standard", "side"], 0)
            .with_bool("apply", true)
            .with_bool("alternate", false)
            .with_bool("ok enabled", true)
            .with_bool("alt resets", true)
            .with_bool("body owns enter", false)
    };
    let mut all = vec![
        Story::new("patterns/dialog/frame", Level::Patterns, "Dialog", "Frame", frame, show_frame),
        Story::new("patterns/dialog/modeless", Level::Patterns, "Dialog", "Modeless", modeless, show_modeless),
        Story::new("patterns/dialog/footer", Level::Patterns, "Dialog", "Footer", footer, show_footer),
    ];
    all.extend(super::stack_stories::stories());
    all
}

fn width_arg(args: &Args) -> Width {
    match args.choice("width") {
        "compact" => Width::Compact,
        "wide" => Width::Wide,
        _ => Width::Standard,
    }
}

fn lines(ui: &mut Ui, count: usize) {
    for n in 1..=count {
        ui.label(format!("Line {n} of the dialog body"));
    }
}

/// The bare frame inline: the surface, the title row with its rule, a body.
fn show_frame(ui: &mut Ui, args: &mut Args) {
    let tokens = crate::theme::current(ui.ctx());
    let (min, max) = width_arg(args).range(&tokens.component.dialog, 2000.0);
    let mut frame = DialogFrame::new("story-frame", "Canvas Size").roomy(args.bool("roomy")).width(min, max);
    if args.bool("close box") {
        frame = frame.close_label("Close");
    }
    frame.show(ui, |ui| lines(ui, 3));
}

/// A modeless dialog in its own area: drag the title to move it; it reopens where it was left.
fn show_modeless(ui: &mut Ui, args: &mut Args) {
    let key = Id::unique("craft_ui::dialog::story-modeless");
    let mut state: DialogState = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.bool("open") {
        let mut dialog = Dialog::new("story-modeless", "Tool window").modeless().width(width_arg(args));
        if args.bool("resizable") {
            dialog = dialog.resizable(Resize { default: vec2(380.0, 240.0), min: vec2(260.0, 160.0), max: ResizeMax::Window });
        }
        let scroll = args.bool("scroll body");
        if scroll {
            dialog = dialog.body(Body::Scroll { reserve: 0.0 });
        }
        let shown = dialog.show(ui.ctx(), &mut state, &mut (), |ui, ()| lines(ui, if scroll { 60 } else { 4 }));
        if shown.outcome == Some(Outcome::Close) {
            args.set_bool("open", false);
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}

/// A dialog with a footer: Enter answers OK, Esc cancels, holding Alt turns Cancel into Reset.
fn show_footer(ui: &mut Ui, args: &mut Args) {
    let key = Id::unique("craft_ui::dialog::story-footer");
    let mut state: DialogState = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.bool("open") {
        let mut footer = if args.choice("shape") == "side" { Footer::side().extra("Add to Swatches") } else { Footer::standard() };
        footer = footer.ok_enabled(args.bool("ok enabled")).enter(if args.bool("body owns enter") { EnterRule::Body } else { EnterRule::Default });
        if args.bool("apply") && args.choice("shape") != "side" {
            footer = footer.apply("Apply");
        }
        if args.bool("alternate") && args.choice("shape") != "side" {
            footer = footer.alternate("Don't Save");
        }
        if args.bool("alt resets") {
            footer = footer.reset(Reset::AltCancel("Reset".into()));
        }
        let shown = Dialog::new("story-footer", "Canvas Size").modeless().width(Width::Compact).footer(footer).show(
            ui.ctx(),
            &mut state,
            &mut (),
            |ui, ()| lines(ui, 2),
        );
        if shown.outcome.is_some_and(Outcome::closes) {
            args.set_bool("open", false);
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}
