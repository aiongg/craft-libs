//! The app's own bar as the window's title bar: caption buttons, the drag area and, on Linux,
//! resize zones along the window's edges.
//!
//! [`TitleBar`] lays out a bar for the frame [`craft_window::policy::decide_for`] chose:
//!
//! | Frame | Leading end | Trailing end | Empty area | Resize zones |
//! |---|---|---|---|---|
//! | [`Frame::Native`] | — | — | — (the OS bar moves the window) | — |
//! | [`Frame::Inset`] | the traffic lights' width | — | drags, double-click per the macOS setting | — |
//! | [`Frame::Integrated`] | the layout's left buttons | the layout's right buttons | drags, double-click maximizes | Linux |
//!
//! In full screen there is no inset, no button and no drag. The app draws the bar's background
//! and its contents (menus, tabs, search) in [`BarSlots::content`].
//!
//! ```ignore
//! let colors = CaptionColors { glyph, glyph_inactive, hover, pressed, focus };
//! let bar = TitleBar::new(&decision.frame, 32.0, colors).labels(&labels).show(ui, |ui, slots| {
//!     menus(ui);
//! });
//! if bar.close_requested {
//!     app.quit(ctx); // asks about unsaved documents first
//! }
//! ```
//!
//! The app also calls [`release_after_os_resize`] from its `raw_input_hook`, and on macOS
//! [`craft_window::title_bar::sync_movable`] once per frame.

mod buttons;
mod colors;
mod resize;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod snapshot_tests;
#[cfg(test)]
mod tests;

pub use buttons::{
    BREEZE_CLOSE_HOVER, BREEZE_CLOSE_PRESSED, Caption, CaptionButtons, CaptionIcon, CaptionIcons, CaptionLabels, RenderIconFn, WINDOWS_BUTTON_WIDTH,
    WINDOWS_CLOSE, caption_buttons, caption_id,
};
pub use colors::CaptionColors;
pub use craft_window::policy::{CaptionFamily, Captions, Frame, MacInset};
pub use resize::{CORNER, EDGE, release_after_os_resize};

use craft_window::policy::Os;
use egui::{Id, Rect, Ui, UiBuilder, ViewportCommand, vec2};

use crate::direction::{self, is_rtl};

/// Where the bar's parts sit, for a later Windows snap-layout hit test (the window procedure
/// answers `WM_NCHITTEST` from them). The bar reports every frame; buttons win over the drag area
/// where they overlap.
///
/// Every method does nothing by default, and `()` is the sink that records nothing.
pub trait CaptionRegions {
    /// A caption button's rect, in points.
    fn button(&mut self, _caption: Caption, _rect: Rect) {}
    /// The bar's rect, which drags the window outside its buttons and controls.
    fn drag(&mut self, _rect: Rect) {}
}

impl CaptionRegions for () {}

/// The parts of the bar the app fills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarSlots {
    /// Between the leading inset or buttons and the trailing buttons.
    pub content: Rect,
    /// The height the bar's controls centre on: the traffic lights' centre under
    /// [`Frame::Inset`], else the bar's middle.
    pub centre_line: f32,
}

/// What happened in the bar this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitleBarResponse {
    /// The caption button clicked. Minimize and maximize have already sent their commands.
    pub caption_clicked: Option<Caption>,
    /// Close was clicked. The bar never closes the window: the app runs its own quit path.
    pub close_requested: bool,
}

/// The app's bar as the title bar. Build it every frame.
pub struct TitleBar<'a> {
    frame: &'a Frame,
    height: f32,
    colors: CaptionColors,
    id: Id,
    labels: Option<&'a CaptionLabels>,
    icons: Option<&'a CaptionIcons>,
    regions: Option<&'a mut dyn CaptionRegions>,
}

impl<'a> TitleBar<'a> {
    pub fn new(frame: &'a Frame, height: f32, colors: CaptionColors) -> Self {
        Self { frame, height, colors, id: Id::unique("title_bar"), labels: None, icons: None, regions: None }
    }

    /// Distinguishes the bars of several windows drawn from one `Ui` tree.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Id::unique(("title_bar", salt));
        self
    }

    /// The caption buttons' tooltips and accessibility labels; English without.
    #[must_use]
    pub fn labels(mut self, labels: &'a CaptionLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Glyphs from the desktop's icon theme for the round button families.
    #[must_use]
    pub fn icons(mut self, icons: Option<&'a CaptionIcons>) -> Self {
        self.icons = icons;
        self
    }

    /// Where to report the buttons and the drag area each frame.
    #[must_use]
    pub fn regions(mut self, regions: &'a mut dyn CaptionRegions) -> Self {
        self.regions = Some(regions);
        self
    }

    /// Lays out [leading inset | app content | caption buttons] in a bar as wide as `ui` and
    /// [`height`](Self::new) tall, and turns the rest into the drag area.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui, BarSlots)) -> TitleBarResponse {
        let (_, bar) = ui.allocate_space(vec2(ui.available_width(), self.height));
        let ctx = ui.ctx().clone();
        let fullscreen = ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        let window = ctx.content_rect();
        let titled = !fullscreen && !matches!(self.frame, Frame::Native);

        let mut slot = bar;
        let mut centre_line = bar.center().y;
        match self.frame {
            Frame::Inset(inset) if titled => {
                slot.min.x += inset.leading;
                centre_line = bar.top() + inset.centre_from_top;
            }
            Frame::Integrated(captions) if titled => {
                let (left, right) = buttons::arranged(captions, is_rtl(ui));
                let family = captions.family;
                slot.min.x = slot.min.x.max(window.left() + buttons::side_width(family, &left));
                slot.max.x = slot.max.x.min(window.right() - buttons::side_width(family, &right));
            }
            _ => {}
        }

        // Below the controls, which take their own clicks.
        let empty = titled.then(|| {
            let starts = || ctx.viewport(|v| v.commands.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count());
            let before = starts();
            let resp = craft_window::title_bar::empty_area(ui, bar);
            if starts() > before {
                resize::hand_off(&ctx);
            }
            resp.id
        });

        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(slot).layout(direction::row(ui)));
        content(&mut child, BarSlots { content: slot, centre_line });
        let layer = ui.layer_id();
        let mut holes: Vec<Rect> = ctx.viewport(|v| {
            v.this_pass
                .widgets
                .get_layer(layer)
                .filter(|w| w.sense.interactive() && Some(w.id) != empty && w.interact_rect.intersects(slot))
                .map(|w| w.interact_rect)
                .collect()
        });

        let mut clicked = None;
        if let (Frame::Integrated(captions), true) = (self.frame, titled) {
            let mut b = CaptionButtons::new(captions, &self.colors).icons(self.icons);
            if let Some(labels) = self.labels {
                b = b.labels(labels);
            }
            let (c, rects) = b.show_rects(ui, bar);
            clicked = c;
            if let Some(regions) = self.regions {
                regions.drag(bar);
                for &(c, r) in &rects {
                    regions.button(c, r);
                }
            }
            holes.extend(rects.into_iter().map(|(_, r)| r));
            // Windows' window procedure resizes from the frame itself.
            if Os::current() == Os::Linux {
                if !maximized {
                    resize::resize_zones(ui, &holes);
                }
                resize::reshow_cursor_after_os_resize(&ctx);
            }
        } else if let (Some(regions), true) = (self.regions, titled) {
            regions.drag(bar);
        }
        TitleBarResponse { caption_clicked: clicked, close_requested: clicked == Some(Caption::Close) }
    }
}
