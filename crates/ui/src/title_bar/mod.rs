//! The app's own bar as the window's title bar: caption buttons, the drag area and resize zones
//! along the window's edges.
//!
//! [`TitleBar`] lays out a bar for the frame [`craft_window::policy::decide_for`] chose:
//!
//! | Frame | Leading end | Trailing end | Empty area | Resize zones |
//! |---|---|---|---|---|
//! | [`Frame::Native`] | — | — | — (the OS bar moves the window) | — |
//! | [`Frame::Inset`] | the traffic lights' width | — | drags, double-click per the macOS setting | — |
//! | [`Frame::Integrated`] | the layout's left buttons | the layout's right buttons | drags, double-click maximizes | until a native hit test resizes |
//!
//! In full screen there is no inset, no button and no drag. The app draws the bar's background
//! and its contents (menus, tabs, search) in [`BarSlots::content`]. Height and colours come from
//! the installed theme ([`TitleBarStyle`]); every id and the OS hand-off belong to one window, named
//! by [`TitleBar::id_salt`] (default: the viewport).
//!
//! ```ignore
//! let bar = TitleBar::new(&decision.frame).labels(&labels).show(ui, |ui, slots| {
//!     menus(ui);
//! });
//! if bar.close_requested {
//!     app.quit(ctx); // asks about unsaved documents first
//! }
//! ```
//!
//! The resize zones stay on until the window procedure answers the hit test itself: the app says
//! so with [`TitleBar::native_hit_test`] or [`CaptionRegions::native_hit_test`].
//!
//! The app also calls [`release_after_os_resize`] from its `raw_input_hook`, and on macOS
//! [`craft_window::title_bar::sync_movable`] once per frame.

mod captions;
mod resize;
#[cfg(test)]
mod snapshot_tests;
mod style;
#[cfg(test)]
mod tests;

pub use captions::{
    Caption, CaptionButtons, CaptionIcon, CaptionIcons, CaptionLabels, RenderIconFn, WINDOWS_BUTTON_WIDTH, caption_buttons, caption_id,
};
pub use craft_window::policy::{CaptionFamily, Captions, Frame, MacInset};
pub use resize::{release_after_os_resize, resize_zones_only};
pub use style::TitleBarStyle;

use egui::{Id, Rect, Ui, UiBuilder, ViewportCommand, pos2, vec2};

use crate::direction::{self, is_rtl};
use crate::theme::{Tokens, TokensOf as _, UiTokens};

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
    /// Whether the window procedure answers the hit test from these regions, edges included
    /// (Windows' `WM_NCHITTEST`). The bar then draws no resize zones. False by default.
    fn native_hit_test(&self) -> bool {
        false
    }
}

impl CaptionRegions for () {}

/// The parts of the bar the app fills.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BarSlots {
    /// Between the leading inset or buttons and the trailing buttons: excludes both
    /// [`leading_caption`](Self::leading_caption) and [`trailing_caption`](Self::trailing_caption).
    pub content: Rect,
    /// Points reserved at the leading end (the UI direction's start): the macOS inset or the
    /// caption buttons on that side, 0 when none. On macOS under RTL the lights stay on the
    /// physical left, so the reservation is the trailing one.
    pub leading_caption: f32,
    /// Points reserved at the trailing end: the caption buttons on that side, 0 when none.
    pub trailing_caption: f32,
    /// The height the bar's controls centre on: the traffic lights' centre under
    /// [`Frame::Inset`], else the bar's middle.
    pub centre_line: f32,
    /// The bar's height: [`TitleBarStyle::height`], raised to [`TitleBarStyle::height_adwaita`]
    /// with Adwaita caption buttons.
    pub height: f32,
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
    salt: Option<Id>,
    labels: Option<&'a CaptionLabels>,
    icons: Option<&'a CaptionIcons>,
    regions: Option<&'a mut dyn CaptionRegions>,
    native_hit_test: bool,
    separator: bool,
}

impl<'a> TitleBar<'a> {
    pub fn new(frame: &'a Frame) -> Self {
        Self { frame, salt: None, labels: None, icons: None, regions: None, native_hit_test: false, separator: false }
    }

    /// The sizes and colours the bar paints with, for the installed theme.
    pub fn style(tokens: &UiTokens) -> TitleBarStyle {
        TitleBarStyle::from_tokens(tokens)
    }

    /// Names the window the bar belongs to: its ids, resize zones and caption buttons derive from
    /// `window` (see [`caption_id`]). Needed once several windows draw a bar; the default is the
    /// window's viewport id.
    #[must_use]
    pub fn id_salt(mut self, window: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.salt = Some(captions::window_scope(window));
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

    /// The window procedure answers the hit test, edges included, so the bar draws no resize
    /// zones. Off by default: an undecorated window gets the bar's zones.
    #[must_use]
    pub fn native_hit_test(mut self, native: bool) -> Self {
        self.native_hit_test = native;
        self
    }

    /// A hairline in [`TitleBarStyle::separator`] along the bar's bottom edge, one device pixel
    /// thick, so the app needs no per-family bar height to place it.
    #[must_use]
    pub fn separator(mut self, separator: bool) -> Self {
        self.separator = separator;
        self
    }

    /// Lays out [leading inset | app content | caption buttons] in a bar as wide as `ui` and
    /// [`BarSlots::height`] tall, and turns the rest into the drag area.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui, BarSlots)) -> TitleBarResponse {
        let style = TitleBarStyle::from_tokens(&Tokens::of(ui));
        let ctx = ui.ctx().clone();
        let scope = self.salt.unwrap_or_else(|| captions::window_scope(ctx.viewport_id()));
        let adwaita = matches!(self.frame, Frame::Integrated(c) if c.family == CaptionFamily::Adwaita);
        let height = if adwaita { style.height.max(style.height_adwaita) } else { style.height };
        let (_, bar) = ui.allocate_space(vec2(ui.available_width(), height));
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
                let (left, right) = captions::arranged(captions, is_rtl(ui));
                let family = captions.family;
                slot.min.x = slot.min.x.max(window.left() + captions::side_width(family, &left));
                slot.max.x = slot.max.x.min(window.right() - captions::side_width(family, &right));
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

        let mut child = ui.new_child(UiBuilder::new().id_salt(scope).max_rect(slot).layout(direction::row(ui)));
        let (left_w, right_w) = (slot.min.x - bar.min.x, bar.max.x - slot.max.x);
        let (leading_caption, trailing_caption) = if is_rtl(ui) { (right_w, left_w) } else { (left_w, right_w) };
        content(&mut child, BarSlots { content: slot, leading_caption, trailing_caption, centre_line, height });
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
            let mut b = CaptionButtons::new(captions, &style).icons(self.icons);
            if let Some(labels) = self.labels {
                b = b.labels(labels);
            }
            let (c, rects) = b.window_scope(scope).show_rects(ui, bar);
            clicked = c;
            let mut native = self.native_hit_test;
            if let Some(regions) = self.regions {
                native |= regions.native_hit_test();
                regions.drag(bar);
                for &(c, r) in &rects {
                    regions.button(c, r);
                }
            }
            holes.extend(rects.into_iter().map(|(_, r)| r));
            if !native {
                if !maximized {
                    resize::resize_zones(ui, &holes, scope, &style);
                }
                resize::reshow_cursor_after_os_resize(&ctx);
            }
        } else if let (Some(regions), true) = (self.regions, titled) {
            regions.drag(bar);
        }
        if self.separator {
            let width = craft_theme::Stroke::Hairline.width(ui.pixels_per_point());
            ui.painter().rect_filled(Rect::from_min_max(pos2(bar.left(), bar.bottom() - width), bar.right_bottom()), 0.0, style.separator);
        }
        TitleBarResponse { caption_clicked: clicked, close_requested: clicked == Some(Caption::Close) }
    }
}
