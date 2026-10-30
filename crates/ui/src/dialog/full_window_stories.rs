//! Story of the full-window dialog.

use egui::{Rect, Ui, pos2, vec2};

use super::{ButtonRole, DialogState, FooterButton, FullWindowDialog, Outcome};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FullWindowDialog"];

#[derive(StoryArgs, Debug, PartialEq)]
struct WorkspaceArgs {
    open: bool,
    #[arg(name = "title strip")]
    title_strip: bool,
    apply: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/full-window/workspace",
        Level::Patterns,
        "FullWindowDialog",
        "Workspace",
        WorkspaceArgs { open: true, title_strip: true, apply: false },
        show_workspace
    )]
}

/// A workspace over the story's area: a view on the left, settings and the footer on the right.
fn show_workspace(ui: &mut Ui, args: &mut WorkspaceArgs) {
    let key = ui.make_persistent_id("craft_ui::dialog::story-full-window");
    let mut state: DialogState<f32> = ui.data_mut(|d| d.remove_temp(key)).unwrap_or_default();
    if args.open {
        let area = Rect::from_min_size(ui.max_rect().min, vec2(ui.available_width().min(720.0), 320.0));
        ui.allocate_rect(area, egui::Sense::hover());
        let apply = args.apply;
        let mut dialog = FullWindowDialog::new("story-full-window").rect(area);
        if args.title_strip {
            dialog = dialog.title("Liquify");
        }
        let mut brush = 50.0_f32;
        let shown = dialog.show(ui.ctx(), &mut state, &mut brush, |ui, brush, fx| {
            let room = ui.max_rect();
            let side = room.shrink2(vec2(0.0, 0.0));
            let panel = Rect::from_min_max(pos2(side.right() - 200.0, side.top()), side.max).shrink(8.0);
            ui.label("The view fills the rest of the window.");
            ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(panel.left(), panel.top() + 36.0), panel.max)), |ui| {
                ui.add(egui::Slider::new(brush, 1.0..=100.0).text("Brush size"));
            });
            let mut buttons = vec![FooterButton::new(ButtonRole::Default, "OK"), FooterButton::new(ButtonRole::Cancel, "Cancel")];
            if apply {
                buttons.push(FooterButton::new(ButtonRole::Apply, "Apply"));
            }
            fx.buttons(ui, Rect::from_min_max(pos2(panel.left(), panel.bottom() - 28.0), panel.max), buttons);
        });
        if shown.outcome.is_some_and(Outcome::closes) {
            args.open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}
