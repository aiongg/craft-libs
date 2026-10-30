//! Story of the embedded viewport window.

use egui::{Id, Ui, ViewportBuilder, ViewportId, vec2};

use super::{Hosting, ViewportWindow};
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ViewportWindow"];

pub(crate) fn stories() -> Vec<Story> {
    let embedded = || Args::new().with_bool("open", true).with_bool("app modal", false).with_bool("scrim", false);
    vec![Story::new("patterns/viewport/embedded", Level::Patterns, "ViewportWindow", "Embedded", embedded, show_embedded)]
}

/// The window hosted inside the gallery; a native window is not shown here.
fn show_embedded(ui: &mut Ui, args: &mut Args) {
    if !args.bool("open") {
        return;
    }
    let builder = ViewportBuilder::default().with_inner_size(vec2(360.0, 200.0)).with_min_inner_size(vec2(240.0, 140.0));
    let mut window = ViewportWindow::new(ViewportId(Id::unique("craft_ui::viewport::story")), builder).title("Settings").embedded(true);
    if args.bool("app modal") {
        window = window.app_modal(args.bool("scrim"));
    }
    let shown = window.show(ui.ctx(), |ui, hosting| {
        ui.label(match hosting {
            Hosting::Native => "Hosted in an OS window.",
            Hosting::Embedded => "Hosted in the main window.",
        });
    });
    if shown.close_requested {
        args.set_bool("open", false);
    }
}
