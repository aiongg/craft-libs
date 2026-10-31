//! Stories of the dialog frame, the modeless dialog and the footer.

use egui::{Ui, vec2};

use super::{Body, Dialog, DialogFrame, DialogLabels, DialogState, EnterRule, Footer, Outcome, Reset, Resize, ResizeMax, Width};
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Dialog"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum WidthArg {
    Compact,
    Standard,
    Wide,
}

impl WidthArg {
    fn width(self) -> Width {
        match self {
            WidthArg::Compact => Width::Compact,
            WidthArg::Standard => Width::Standard,
            WidthArg::Wide => Width::Wide,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ShapeArg {
    Standard,
    Side,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FrameArgs {
    width: WidthArg,
    #[arg(name = "close box")]
    close_box: bool,
    roomy: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ModelessArgs {
    open: bool,
    width: WidthArg,
    resizable: bool,
    #[arg(name = "scroll body")]
    scroll_body: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FooterArgs {
    open: bool,
    shape: ShapeArg,
    apply: bool,
    alternate: bool,
    #[arg(name = "ok enabled")]
    ok_enabled: bool,
    #[arg(name = "alt resets")]
    alt_resets: bool,
    #[arg(name = "body owns enter")]
    body_owns_enter: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    let mut all = vec![
        typed_story!(
            "patterns/dialog/frame",
            Level::Patterns,
            "Dialog",
            "Frame",
            FrameArgs { width: WidthArg::Standard, close_box: true, roomy: false },
            show_frame
        ),
        typed_story!(
            "patterns/dialog/modeless",
            Level::Patterns,
            "Dialog",
            "Modeless",
            ModelessArgs { open: true, width: WidthArg::Standard, resizable: false, scroll_body: false },
            show_modeless
        ),
        typed_story!(
            "patterns/dialog/footer",
            Level::Patterns,
            "Dialog",
            "Footer",
            FooterArgs {
                open: true,
                shape: ShapeArg::Standard,
                apply: true,
                alternate: false,
                ok_enabled: true,
                alt_resets: true,
                body_owns_enter: false
            },
            show_footer
        ),
    ];
    all.extend(super::stack_stories::stories());
    all.extend(super::framework_stories::stories());
    all
}

fn lines(ui: &mut Ui, count: usize) {
    for n in 1..=count {
        ui.label(format!("Line {n} of the dialog body"));
    }
}

/// The bare frame inline: the surface, the title row with its rule, a body.
fn show_frame(ui: &mut Ui, args: &mut FrameArgs) {
    let tokens = crate::theme::current(ui.ctx());
    let (min, max) = args.width.width().range(&tokens.component.dialog, 2000.0);
    let mut frame = DialogFrame::new("story-frame", "Canvas Size").roomy(args.roomy).width(min, max);
    if args.close_box {
        frame = frame.close_label(DialogLabels::of(ui).close);
    }
    frame.show(ui, |ui| lines(ui, 3));
}

/// A modeless dialog in its own area: drag the title to move it; it reopens where it was left.
fn show_modeless(ui: &mut Ui, args: &mut ModelessArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-modeless");
    let mut state: DialogState = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.open {
        let mut dialog = Dialog::new("story-modeless", "Tool window").modeless().width(args.width.width());
        if args.resizable {
            dialog = dialog.resizable(Resize { default: vec2(380.0, 240.0), min: vec2(260.0, 160.0), max: ResizeMax::Window });
        }
        let scroll = args.scroll_body;
        if scroll {
            dialog = dialog.body(Body::Scroll);
        }
        let shown = dialog.show(ui.ctx(), &mut state, &mut (), |ui, ()| lines(ui, if scroll { 60 } else { 4 }));
        if shown.outcome == Some(Outcome::Close) {
            args.open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}

/// A dialog with a footer: Enter answers OK, Esc cancels, holding Alt turns Cancel into Reset.
fn show_footer(ui: &mut Ui, args: &mut FooterArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-footer");
    let mut state: DialogState = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.open {
        let mut footer = if args.shape == ShapeArg::Side { Footer::side().extra("Add to Swatches") } else { Footer::standard() };
        footer = footer.ok_enabled(args.ok_enabled).enter(if args.body_owns_enter { EnterRule::Body } else { EnterRule::Default });
        if args.apply && args.shape != ShapeArg::Side {
            footer = footer.apply(DialogLabels::of(ui).apply);
        }
        if args.alternate && args.shape != ShapeArg::Side {
            footer = footer.alternate("Don't Save");
        }
        if args.alt_resets {
            footer = footer.reset(Reset::AltCancel("Reset".into()));
        }
        let shown = Dialog::new("story-footer", "Canvas Size").modeless().width(Width::Compact).footer(footer).show(
            ui.ctx(),
            &mut state,
            &mut (),
            |ui, ()| lines(ui, 2),
        );
        if shown.outcome.is_some_and(Outcome::closes) {
            args.open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}
