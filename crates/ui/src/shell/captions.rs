//! The caption record: where a title bar put its caption buttons and its drag area, published every pass for the
//! Windows hit test (snap layouts) and read back per viewport.

use crate::runtime::egui::pass;
use crate::runtime::frame::PerViewport;
use crate::runtime::windows::WindowHost;
use egui::{Pos2, Rect, Ui, ViewportId};

/// A caption button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Caption {
    /// Minimizes the window.
    Minimize,
    /// Maximizes the window, or restores it while it is maximized.
    Maximize,
    /// Closes the window.
    Close,
}

impl From<craft_window::Button> for Caption {
    fn from(button: craft_window::Button) -> Self {
        match button {
            craft_window::Button::Minimize => Caption::Minimize,
            craft_window::Button::Maximize => Caption::Maximize,
            craft_window::Button::Close => Caption::Close,
        }
    }
}

/// What lies under a point of a title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CaptionHit {
    /// A caption button.
    Button(Caption),
    /// The drag area, outside every button.
    Drag,
}

/// Where a title bar's parts sit, in points of its viewport: the window procedure answers a Windows hit test from
/// them. A bar publishes the record every pass with [`publish`](Self::publish); read it with
/// [`WindowHost::caption_regions`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CaptionRegions {
    /// The caption buttons.
    pub buttons: Vec<(Caption, Rect)>,
    /// The bar's rect, which drags the window outside its buttons and controls.
    pub drag: Rect,
    /// Whether the window procedure answers the hit test from these regions, edges included (Windows'
    /// `WM_NCHITTEST`). The window then draws no edge zones.
    pub native_hit_test: bool,
}

impl CaptionRegions {
    /// A record with the bar's drag area `drag` and no buttons.
    pub fn new(drag: Rect) -> Self {
        CaptionRegions { buttons: Vec::new(), drag, native_hit_test: false }
    }

    /// Adds the button `caption` at `rect`.
    pub fn button(mut self, caption: Caption, rect: Rect) -> Self {
        self.buttons.push((caption, rect));
        self
    }

    /// Sets whether the window procedure answers the hit test from these regions.
    pub fn native_hit_test(mut self, native: bool) -> Self {
        self.native_hit_test = native;
        self
    }

    /// What lies at `point`: a button wins over the drag area where they overlap.
    pub fn hit(&self, point: Pos2) -> Option<CaptionHit> {
        self.buttons
            .iter()
            .find(|(_, rect)| rect.contains(point))
            .map(|(caption, _)| CaptionHit::Button(*caption))
            .or_else(|| self.drag.contains(point).then_some(CaptionHit::Drag))
    }

    /// Records the regions as `viewport`'s for the running pass. A bar calls it every pass it draws; the record
    /// holds for that pass and the next, so a window whose bar stops drawing has none.
    pub fn publish(&self, ui: &Ui, viewport: ViewportId) {
        let ctx = ui.ctx();
        let written = pass::pass_nr(ctx, viewport);
        PerViewport::<Written>::with(ctx, viewport, |record| *record = Written { pass: written, regions: Some(self.clone()) });
    }
}

/// A viewport's record and the pass it was written in.
#[derive(Default)]
struct Written {
    pass: u64,
    regions: Option<CaptionRegions>,
}

impl WindowHost {
    /// The caption regions the title bar of `viewport` published in the running pass or the one before; `None`
    /// for a viewport whose bar published none.
    pub fn caption_regions(&self, viewport: ViewportId) -> Option<CaptionRegions> {
        let ctx = &self.ctx;
        let now = pass::pass_nr(ctx, viewport);
        PerViewport::<Written>::with(ctx, viewport, |record| record.regions.clone().filter(|_| now.saturating_sub(record.pass) <= 1))
    }
}
