//! A story on its surface: the frame it is drawn in and the canvas it is shown on.
use craft_theme::SystemAppearance;
use craft_ui::docking::DockLook;
use craft_ui::story::{Args, Story, Surface};
use craft_ui::theme::{ToEgui, Tokens, TokensOf, margin, scope};
use egui::{Color32, Frame, Margin, Stroke, Ui};

use super::applied::shown_mode;
use crate::appearance::Appearance;
use crate::inspector::Inspector;

/// The dock colour with the panel padding around what a view holds.
pub(crate) fn canvas_frame(tokens: &Tokens) -> Frame {
    Frame::new().fill(tokens.color.surface.background.to_egui()).inner_margin(margin(tokens.space.pad.panel))
}

/// How a story is framed: the fill and edge of its surface, and the margin round the story.
pub(crate) struct StoryFrame {
    pub(super) fill: Option<Color32>,
    pub(super) edge: Stroke,
    pub(super) margin: Margin,
    /// The gap between stacked controls, when the surface is a panel body.
    body_gap: Option<f32>,
}

impl StoryFrame {
    /// The frame of `surface` under `ui`'s theme. A panel is a dock panel body with its margin and
    /// item spacing, edge and fill from [`DockLook`]; canvas and chrome are their surface colours
    /// with the dock's edge; an own surface adds nothing.
    pub(crate) fn of(ui: &Ui, surface: Surface) -> Self {
        let tokens = Tokens::of(ui);
        let look = DockLook::of(ui);
        let surfaces = &tokens.color.surface;
        let bar = |fill: Color32| Self { fill: Some(fill), edge: look.border, margin: margin(tokens.space.pad.dialog), body_gap: None };
        match surface {
            Surface::Panel => {
                Self { fill: Some(look.background), edge: look.border, margin: margin(tokens.space.pad.panel), body_gap: Some(look.body_gap) }
            }
            Surface::Canvas => bar(surfaces.canvas.to_egui()),
            Surface::Chrome => bar(surfaces.chrome.to_egui()),
            Surface::Own => Self { fill: None, edge: Stroke::NONE, margin: margin(tokens.space.pad.dialog), body_gap: None },
        }
    }

    /// The frame to show the story in.
    fn frame(&self) -> Frame {
        let frame = Frame::new().inner_margin(self.margin);
        match self.fill {
            Some(fill) => frame.fill(fill).stroke(self.edge),
            None => frame,
        }
    }

    /// Applies the surface's item spacing to the story's `ui`.
    pub(super) fn space(&self, ui: &mut Ui) {
        if let Some(gap) = self.body_gap {
            ui.spacing_mut().item_spacing.y = gap;
        }
    }
}

/// The story on its surface under the bar's appearance.
pub(crate) fn canvas(
    ui: &mut Ui,
    story: &Story,
    args: &mut Args,
    appearance: &Appearance,
    system: SystemAppearance,
    inspector: Option<&mut Inspector>,
) {
    let shown = shown_mode(ui.ctx());
    let overrides = |o: &mut craft_ui::theme::Overrides| {
        appearance.overrides(o);
        o.mode = Some(appearance.mode(shown));
        o.contrast = Some(appearance.contrast(system));
    };
    scope(ui, overrides, |ui| framed(ui, story, args, inspector, "Canvas", appearance.rtl));
}

/// The story in its frame, with the inspector over it when there is one.
pub(crate) fn framed(ui: &mut Ui, story: &Story, args: &mut Args, inspector: Option<&mut Inspector>, name: &str, rtl: bool) {
    let tokens = Tokens::of(ui);
    let surface = StoryFrame::of(ui, story.surface);
    let framed = surface.frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        surface.space(ui);
        ui.push_id(story.id, |ui| {
            (story.show)(ui, args);
            ui.unique_id()
        })
        .inner
    });
    if let Some(inspector) = inspector {
        inspector.overlay(ui, name.to_owned(), framed.response.rect, framed.inner.accesskit_id(), tokens.size.control.sm, rtl);
    }
}
