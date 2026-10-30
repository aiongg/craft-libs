//! Story of the embedded viewport window.

use egui::{Id, Ui, ViewportBuilder, ViewportId, vec2};

use super::{Hosting, ViewportWindow};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ViewportWindow"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/viewport/embedded",
        Level::Patterns,
        "ViewportWindow",
        "Embedded",
        EmbeddedArgs { open: true, app_modal: false, scrim: false },
        show_embedded
    )]
}

/// The window hosted inside the gallery; a native window is not shown here.
#[derive(StoryArgs, Debug, PartialEq)]
struct EmbeddedArgs {
    open: bool,
    #[arg(name = "app modal")]
    app_modal: bool,
    scrim: bool,
}

fn show_embedded(ui: &mut Ui, args: &mut EmbeddedArgs) {
    if !args.open {
        return;
    }
    let builder = ViewportBuilder::default().with_inner_size(vec2(360.0, 200.0)).with_min_inner_size(vec2(240.0, 140.0));
    let mut window = ViewportWindow::new(ViewportId(Id::unique("craft_ui::viewport::story")), builder).title("Settings").embedded(true);
    if args.app_modal {
        window = window.app_modal(args.scrim);
    }
    let shown = window.show(ui.ctx(), |ui, hosting| {
        ui.label(match hosting {
            Hosting::Native => "Hosted in an OS window.",
            Hosting::Embedded => "Hosted in the main window.",
        });
    });
    if shown.close_requested {
        args.open = false;
    }
}
