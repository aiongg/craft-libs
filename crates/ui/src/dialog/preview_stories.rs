//! Story of the dialog Preview check box, hold-to-compare and dialog-local history.

use egui::Ui;

use super::{Dialog, DialogState, Footer, Outcome, Width};
use crate::controls::{Button, Slider};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

/// The components this module has stories for (`Dialog` is listed by the dialog stories).
pub const COMPONENTS: &[&str] = &[];

#[derive(Clone, Debug, PartialEq)]
struct Fields {
    brightness: f64,
    contrast: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PreviewArgs {
    open: bool,
    preview: bool,
    #[arg(name = "local history")]
    local_history: bool,
    #[arg(name = "compare button")]
    compare_button: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/dialog/preview",
        Level::Patterns,
        "Dialog",
        "Preview",
        PreviewArgs { open: true, preview: true, local_history: true, compare_button: true },
        show_preview
    )]
}

/// Brightness and Contrast with a Preview check box: Cmd/Ctrl+Z steps the sliders, the compare
/// button shows the original while held.
fn show_preview(ui: &mut Ui, args: &mut PreviewArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-preview");
    let mut state: DialogState<Fields> = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.open {
        let mut fields = ui.data_mut(|d| d.get_temp::<Fields>(key)).unwrap_or(Fields { brightness: 0.0, contrast: 0.0 });
        let mut footer = Footer::standard().ok("OK");
        if args.preview {
            footer = footer.preview("Preview");
        }
        let compare = args.compare_button;
        let shows = state.preview().shows_result();
        let shown = Dialog::new("story-preview", "Brightness/Contrast")
            .modeless()
            .width(Width::Compact)
            .footer(footer)
            .history(args.local_history)
            .show(ui.ctx(), &mut state, &mut fields, |ui, fields| {
                for (label, value) in [("Brightness", &mut fields.brightness), ("Contrast", &mut fields.contrast)] {
                    if let Some(&v) = Slider::new(label, label, *value, -100.0..=100.0).show(ui).event.as_ref().and_then(|e| e.value()) {
                        *value = v;
                    }
                }
                ui.label(if shows { "The document shows the result." } else { "The document shows the original." });
            });
        if compare {
            let target = ui.add(Button::new("Hold to compare"));
            state.preview_mut().hold_to_compare(&target);
        }
        if shown.outcome.is_some_and(Outcome::closes) {
            args.open = false;
        } else {
            ui.data_mut(|d| d.insert_temp(key, fields));
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}
