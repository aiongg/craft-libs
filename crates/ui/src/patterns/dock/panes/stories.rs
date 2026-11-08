//! Stories of `PaneArea`.

use super::{PaneArea, PaneContent, PaneSpec, PaneState};
use crate::contract::{BodyCx, PaneEdge, Show as _};
use crate::display::Text;
use crate::runtime::env::Env;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Ui, Vec2};

/// Two panes: the first on the start edge, the second on the bottom edge.
struct Panes {
    float: Vec2,
    side: f32,
}

impl PaneContent<u32> for Panes {
    fn spec(&self, pane: &u32) -> PaneSpec {
        let edge = if *pane == 0 { PaneEdge::Start } else { PaneEdge::Bottom };
        PaneSpec::new(format!("Pane {pane}"), PaneState::docked(edge, self.side), self.float)
    }

    fn body(&mut self, pane: &u32, ui: &mut Ui, _: &mut BodyCx<'_>) {
        Text::new(format!("Contents of pane {pane}")).show(ui);
    }
}

#[story(PaneArea<'static, u32>, covers = [], drive = Drive::Never)]
fn pane_area(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let size = &Env::of(ui).tokens.size;
    let mut content = Panes { float: Vec2::new(size.float.width, size.float.height), side: size.length.field };
    let open = [0_u32, 1];
    PaneArea::new("Panes", &open, &mut content)
        .show(ui, |ui, _| {
            Text::new("Document").show(ui);
        })
        .into()
}
