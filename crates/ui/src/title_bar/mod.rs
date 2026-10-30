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
//! In full screen there is no inset, no button and no drag. The bar paints its background; the
//! app draws its contents (menus, tabs, search) in [`BarSlots::content`]. Height and colours come
//! from the installed theme ([`TitleBarStyle`]); every id and the OS hand-off belong to one window,
//! named by [`TitleBar::id_salt`] (default: the viewport).
//!
//! ```ignore
//! let bar = TitleBar::new(&decision.frame)
//!     .labels(&labels)
//!     .title(Title::new("Poster.psd").unsaved(true))
//!     .show_titled(ui, |ui, slots| { menus(ui); LeadingEnd::from_ui(ui) }, |ui, slots| controls(ui));
//! if bar.close_requested {
//!     app.quit(ctx); // asks about unsaved documents first
//! }
//! ```
//!
//! [`TitleBar::show`] takes one closure for a bar without a title. The title sits on the window's
//! centre line when it fits between the two groups, slides into the gap, or is elided.
//!
//! Under an `egui::Modal` the bar gets no input: after drawing every window the shell calls
//! [`TitleBar::modal_drag_overlay`] with the [`ModalRects`] the open dialogs pushed.
//!
//! The resize zones stay on until the window procedure answers the hit test itself: the app says
//! so with [`TitleBar::native_hit_test`] or [`CaptionRegions::native_hit_test`].
//!
//! The app also calls [`release_after_os_resize`] from its `raw_input_hook`, and on macOS
//! [`craft_window::title_bar::sync_movable`] once per frame.

mod captions;
mod drag;
mod measure;
mod modal;
mod resize;
#[cfg(test)]
mod snapshot_tests;
pub(crate) mod stories;
mod style;
#[cfg(test)]
mod tests;
mod title;

pub use captions::{
    Caption, CaptionButtons, CaptionIcon, CaptionIcons, CaptionLabels, RenderIconFn, WINDOWS_BUTTON_WIDTH, caption_buttons, caption_id,
};
pub use craft_window::policy::{CaptionFamily, Captions, Frame, MacInset};
pub use modal::ModalRects;
pub use resize::{release_after_os_resize, resize_zones_only};
pub use style::TitleBarStyle;
pub use title::Title;

use egui::{Id, Rect, Ui, UiBuilder, pos2, vec2};

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
    /// The free gap between the two groups of [`TitleBar::show_titled`] on the last pass: the
    /// bar's full height, from the leading group's inner edge to the trailing group's. Measured
    /// for every frame, [`Frame::Native`] and full screen included; empty until a pass has
    /// measured it, and after the bar's width changed.
    pub gap: Rect,
    /// The bar is the window's title bar on this pass: it has a drag area and, with caption
    /// buttons, resize zones. False under [`Frame::Native`], in full screen and for a
    /// [`preview`](TitleBar::preview).
    pub titled: bool,
}

/// The window state a [`preview`](TitleBar::preview) bar shows, in place of the viewport's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowState {
    /// The window is maximized: the caption buttons show Restore.
    pub maximized: bool,
    /// The window has keyboard focus: the caption glyphs are at full strength.
    pub active: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self { maximized: false, active: true }
    }
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
    background: bool,
    title: Option<Title>,
    title_when_native: bool,
    preview: Option<WindowState>,
}

impl<'a> TitleBar<'a> {
    pub fn new(frame: &'a Frame) -> Self {
        Self {
            frame,
            salt: None,
            labels: None,
            icons: None,
            regions: None,
            native_hit_test: false,
            separator: false,
            background: true,
            title: None,
            title_when_native: false,
            preview: None,
        }
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

    /// The document title, centred between the groups of [`show_titled`](Self::show_titled). Drawn
    /// only when the OS does not draw one: not under [`Frame::Native`] unless
    /// [`title_when_native`](Self::title_when_native) is set.
    #[must_use]
    pub fn title(mut self, title: Title) -> Self {
        self.title = Some(title);
        self
    }

    /// Draws the [`title`](Self::title) under [`Frame::Native`] too, placed as in an integrated
    /// bar. For hosts with no OS title to show it, such as an embedder or the web build. Off by
    /// default: the OS bar draws the title.
    #[must_use]
    pub fn title_when_native(mut self, draw: bool) -> Self {
        self.title_when_native = draw;
        self
    }

    /// Draws the bar as an inert picture of a window in `state`, for galleries and documentation:
    /// the caption buttons show hover but send no viewport command, and the bar has no drag area,
    /// resize zones, region reports or modal overlay. It reads nothing from the viewport, and its
    /// buttons sit at the bar's own ends, so it may be narrower or wider than the window.
    #[must_use]
    pub fn preview(mut self, state: WindowState) -> Self {
        self.preview = Some(state);
        self
    }

    /// Whether the bar fills its rect with [`TitleBarStyle::bar`]; on by default. Off, the app
    /// paints behind the bar.
    #[must_use]
    pub fn background(mut self, background: bool) -> Self {
        self.background = background;
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
    /// [`BarSlots::height`] tall, and turns the rest into the drag area. For a bar with a centred
    /// title use [`show_titled`](Self::show_titled).
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui, BarSlots)) -> TitleBarResponse {
        self.run(ui, false, |ui, slots| {
            content(ui, slots);
            None
        })
    }

    /// Lays out the bar like [`show`](Self::show) with the app's content split in two groups and
    /// the [`title`](Self::title) centred between them.
    ///
    /// `leading` fills the slot from its reading start and returns the x of its inner edge, the one
    /// facing the title ([`LeadingEnd::from_ui`] reads it from the `Ui`; with the macOS native menu
    /// bar it is the cursor edge). `trailing` then fills the rest from the trailing end, in the
    /// trailing layout, from that edge plus [`TitleBarStyle::title_gap`]. A side without caption
    /// buttons keeps [`TitleBarStyle::pad`] at its end; the macOS inset replaces the margin at the
    /// leading end. The free gap between the two groups is [`BarSlots::gap`] on the next pass.
    pub fn show_titled(
        self,
        ui: &mut Ui,
        leading: impl FnOnce(&mut Ui, BarSlots) -> f32,
        trailing: impl FnOnce(&mut Ui, BarSlots),
    ) -> TitleBarResponse {
        let gap = Tokens::of(ui).component.title_bar.title_gap;
        let rtl = is_rtl(ui);
        self.run(ui, true, move |ui, slots| {
            let inner = leading(ui, slots);
            let area = slots.content;
            let rect = if rtl {
                Rect::from_min_max(area.min, pos2((inner - gap).max(area.min.x), area.max.y))
            } else {
                Rect::from_min_max(pos2((inner + gap).min(area.max.x), area.min.y), area.max)
            };
            let mut group = ui.new_child(UiBuilder::new().id_salt("trailing").max_rect(rect).layout(direction::trailing(ui)));
            trailing(&mut group, slots);
            Some(Edges { leading: inner, trailing: trailing_edge(&group, area, rtl) })
        })
    }

    /// The bar, its drag area, the app's content and the caption buttons. `pad` gives a side
    /// without caption buttons its end margin. `content` returns the inner edges of the two groups
    /// when the bar has a title.
    fn run(mut self, ui: &mut Ui, pad: bool, content: impl FnOnce(&mut Ui, BarSlots) -> Option<Edges>) -> TitleBarResponse {
        let style = TitleBarStyle::from_tokens(&Tokens::of(ui));
        let ctx = ui.ctx().clone();
        let scope = self.salt.unwrap_or_else(|| captions::window_scope(ctx.viewport_id()));
        let rtl = is_rtl(ui);
        let adwaita = matches!(self.frame, Frame::Integrated(c) if c.family == CaptionFamily::Adwaita);
        let height = if adwaita { style.height.max(style.height_adwaita) } else { style.height };
        let (_, bar) = ui.allocate_space(vec2(ui.available_width(), height));
        let preview = self.preview;
        let fullscreen = preview.is_none() && ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
        let maximized = preview.map_or_else(|| ctx.input(|i| i.viewport().maximized).unwrap_or(false), |p| p.maximized);
        let window = if preview.is_some() { bar } else { ctx.content_rect() };
        let titled = !fullscreen && !matches!(self.frame, Frame::Native);
        if self.background {
            ui.painter().rect_filled(bar, 0.0, style.bar);
        }

        let mut slot = bar;
        let mut centre_line = bar.center().y;
        match self.frame {
            Frame::Inset(inset) if titled => {
                slot.min.x += inset.leading;
                centre_line = bar.top() + inset.centre_from_top;
            }
            Frame::Integrated(captions) if titled => {
                let (left, right) = captions::arranged(captions, rtl);
                let family = captions.family;
                // The buttons sit at the bar's ends, which a bar narrower than the window does
                // not share with it.
                slot.min.x = slot.min.x.max(bar.left().max(window.left()) + captions::side_width(family, &left));
                slot.max.x = slot.max.x.min(bar.right().min(window.right()) - captions::side_width(family, &right));
            }
            _ => {}
        }
        let (left_w, right_w) = (slot.min.x - bar.min.x, bar.max.x - slot.max.x);
        let (leading_caption, trailing_caption) = if rtl { (right_w, left_w) } else { (left_w, right_w) };
        if pad {
            let (start, end) = (leading_caption <= 0.0, trailing_caption <= 0.0);
            let (left, right) = if rtl { (end, start) } else { (start, end) };
            slot.min.x += if left { style.pad } else { 0.0 };
            slot.max.x -= if right { style.pad } else { 0.0 };
        }

        let title = self.title.take();
        let empty = (titled && preview.is_none()).then(|| drag::area(ui, bar, scope, title.as_ref()));

        let last_gap = measure::last(&ctx, scope, bar.width()).unwrap_or(Rect::NOTHING);
        let mut child = ui.new_child(UiBuilder::new().id_salt(scope).max_rect(slot).layout(direction::row(ui)));
        let edges = content(
            &mut child,
            BarSlots { content: slot, leading_caption, trailing_caption, centre_line, height, gap: last_gap, titled: empty.is_some() },
        );
        if let Some(edges) = edges {
            let (from, to) = if rtl { (edges.trailing, edges.leading) } else { (edges.leading, edges.trailing) };
            let gap = Rect::from_min_max(pos2(from, bar.top()), pos2(to.max(from), bar.bottom()));
            measure::store(&ctx, scope, bar.width(), gap, empty.is_some());
            if let Some(title) = title.filter(|_| self.title_when_native || !matches!(self.frame, Frame::Native)) {
                title::paint(ui, &style, &title, title::Placement { bar, centre_line, left: from, right: to, rtl });
            }
        }
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
            if let Some(state) = preview {
                b = b.preview(state);
            }
            let (c, rects) = b.window_scope(scope).show_rects(ui, bar);
            clicked = c;
            if preview.is_none() {
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
            }
        } else if let (Some(regions), true, true) = (self.regions, titled, preview.is_none()) {
            regions.drag(bar);
        }
        if self.separator {
            let width = craft_theme::Stroke::Hairline.width(ui.pixels_per_point());
            ui.painter().rect_filled(Rect::from_min_max(pos2(bar.left(), bar.bottom() - width), bar.right_bottom()), 0.0, style.separator);
        }
        TitleBarResponse { caption_clicked: clicked, close_requested: clicked == Some(Caption::Close) }
    }
}

/// The x of the inner edges (the ones facing the title) of the leading and trailing groups.
struct Edges {
    leading: f32,
    trailing: f32,
}

/// The edge of the leading group that faces the title, for [`TitleBar::show_titled`]'s `leading`
/// closure to return.
pub struct LeadingEnd;

impl LeadingEnd {
    /// The inner edge of what `ui` (the closure's `Ui`) has laid out: its right edge, or its left
    /// edge in a right-to-left UI; the reading start of its area when nothing was placed.
    pub fn from_ui(ui: &Ui) -> f32 {
        let (used, area) = (ui.min_rect(), ui.max_rect());
        match (is_rtl(ui), used.width() > 0.0) {
            (false, true) => used.right(),
            (true, true) => used.left(),
            (false, false) => area.left(),
            (true, false) => area.right(),
        }
    }
}

/// The inner edge of the trailing group `group`, which faces the title: the end of the slot
/// `area` when nothing was placed.
fn trailing_edge(group: &Ui, area: Rect, rtl: bool) -> f32 {
    let used = group.min_rect();
    match (rtl, used.width() > 0.0) {
        (false, true) => used.left(),
        (true, true) => used.right(),
        (false, false) => area.right(),
        (true, false) => area.left(),
    }
}
