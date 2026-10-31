//! [`SplitViews`]: one canvas area shown as 1, 2 or 4 views of the same document, as After Effects'
//! View › Switch View Layout (EffectCraft). One view is active: it is the full interactive view
//! (tools, overlays, keys); the others show the document from their own view and become active on
//! a click. With Share View Options on, the grid, guides and rulers settings of the active view
//! apply to every view.
//!
//! Each pane has its own [`CanvasView`] the app keeps; a linked pane follows the active view's
//! zoom and position (View › Match Zoom and Location for panes).
//!
//! ```no_run
//! use craft_ui::canvas::{SplitLayout, SplitViews};
//! # fn demo(ui: &mut egui::Ui, rect: egui::Rect) {
//! let response = SplitViews::new("comp-1", SplitLayout::Two).show(ui, rect, |ui, pane, rect| {
//!     // draw pane `pane` in `rect`; pane 0 is the main view
//! });
//! # let _ = response;
//! # }
//! ```

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, PointerButton, Rect as ScreenRect, Sense, StrokeKind, Ui, UiBuilder, WidgetInfo, pos2};

use super::view::CanvasView;
use crate::localized::Labels;
use crate::theme::{ToEgui, Tokens, TokensOf};

/// The focus edge's width on the active pane.
const ACTIVE_EDGE: f32 = 1.0;

/// How the canvas area divides (View › Switch View Layout).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SplitLayout {
    /// 1 View.
    #[default]
    One,
    /// 2 Views: side by side, the main view on the right.
    Two,
    /// 4 Views: quarters, the main view at the bottom right.
    Four,
}

impl SplitLayout {
    /// Points between panes.
    pub const GAP: f32 = 2.0;

    /// Number of panes.
    pub fn panes(self) -> usize {
        match self {
            SplitLayout::One => 1,
            SplitLayout::Two => 2,
            SplitLayout::Four => 4,
        }
    }

    /// The panes' rects in `full`, the main view (pane 0) first, the others in reading order.
    // effectcraft: crates/ui-egui/src/panels/viewer/view.rs:5 `aux_views`.
    pub fn rects(self, full: ScreenRect) -> Vec<ScreenRect> {
        let half = Self::GAP / 2.0;
        let c = full.center();
        let quad = |x0, y0, x1, y1| ScreenRect::from_min_max(pos2(x0, y0), pos2(x1, y1));
        match self {
            SplitLayout::One => vec![full],
            SplitLayout::Two => vec![quad(c.x + half, full.min.y, full.max.x, full.max.y), quad(full.min.x, full.min.y, c.x - half, full.max.y)],
            SplitLayout::Four => vec![
                quad(c.x + half, c.y + half, full.max.x, full.max.y),
                quad(full.min.x, full.min.y, c.x - half, c.y - half),
                quad(c.x + half, full.min.y, full.max.x, c.y - half),
                quad(full.min.x, c.y + half, c.x - half, full.max.y),
            ],
        }
    }
}

/// One pane's view: its own, or linked to the main view's zoom and position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPane {
    pub view: CanvasView,
    /// The pane follows the main view's centre and zoom (its rotation stays its own).
    pub linked: bool,
}

/// What the split did in a pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitResponse {
    /// The pane pressed this pass that was not the active one: the app makes it active.
    pub activated: Option<usize>,
}

/// The split of one canvas area. The active pane is the app's (it routes tools and keys there).
pub struct SplitViews {
    id_salt: IdSalt,
    layout: SplitLayout,
    active: usize,
}

impl SplitViews {
    pub fn new(id_salt: impl AsIdSalt, layout: SplitLayout) -> Self {
        Self { id_salt: IdSalt::new(id_salt), layout, active: 0 }
    }

    /// The active pane (0 by default): drawn with a 1 pt edge in `line.focus`
    /// when there is more than one pane.
    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    /// Lay out the panes in `rect` and call `pane(ui, index, rect)` for each, clipped to it; a
    /// press on an inactive pane reports it in [`SplitResponse::activated`] and does not reach
    /// the pane's tool. Each pane is an AccessKit group named with [`SplitLabels::view`] (`View
    /// {n}`, counted from 1 in reading order).
    // effectcraft: crates/ui-egui/src/panels/viewer/view.rs:5 `aux_views`, panels/viewers.rs:208 `activate`.
    pub fn show(self, ui: &mut Ui, rect: ScreenRect, mut pane: impl FnMut(&mut Ui, usize, ScreenRect)) -> SplitResponse {
        let rects = self.layout.rects(rect);
        let labels = SplitLabels::of(ui);
        let base = ui.make_persistent_id(self.id_salt);
        let pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        let mut response = SplitResponse::default();
        // Reading order is the screen's: top-left first, the main view where the layout puts it.
        let mut order: Vec<usize> = (0..rects.len()).collect();
        order.sort_by(|&a, &b| {
            let (ra, rb) = (rects[a], rects[b]);
            ra.min.y.total_cmp(&rb.min.y).then(ra.min.x.total_cmp(&rb.min.x))
        });
        for (slot, &i) in order.iter().enumerate() {
            let r = rects[i];
            let taken = rects.len() > 1 && i != self.active && pressed && ui.rect_contains_pointer(r);
            if taken && response.activated.is_none() {
                response.activated = Some(i);
            }
            let mut child = ui.new_child(UiBuilder::new().max_rect(r).id_salt(base.with(("pane", i))));
            child.set_clip_rect(r.intersect(ui.clip_rect()));
            if taken {
                // The press is the pane's activation, not its tool's: a disabled Ui senses nothing.
                child.visuals_mut().disabled_alpha = 1.0;
                child.disable();
            }
            let group = ui.interact(r, base.with(("group", i)), Sense::hover());
            let name = labels.view.replace("{n}", &(slot + 1).to_string());
            group.widget_info(|| WidgetInfo::labeled(Role::Group, ui.is_enabled(), &name));
            pane(&mut child, i, r);
        }
        if rects.len() > 1 {
            let edge = Tokens::of(ui).color.line.focus.to_egui();
            ui.painter().rect_stroke(rects[self.active.min(rects.len() - 1)], 0.0, egui::Stroke::new(ACTIVE_EDGE, edge), StrokeKind::Inside);
        }
        response
    }

    /// Copy the main view's centre and zoom into the linked panes, each keeping its own rect
    /// and rotation.
    pub fn follow(main: &CanvasView, panes: &mut [SplitPane]) {
        for pane in panes.iter_mut().filter(|p| p.linked) {
            pane.view.center = main.center;
            pane.view.zoom = main.zoom;
        }
    }
}

crate::words! {
    /// The words of split views.
    #[non_exhaustive]
    pub struct SplitLabels {
        /// A pane's AccessKit name; `{n}` counts from 1.
        view: "split.view" = "View {n}";
    }
}
