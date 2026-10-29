//! [`Tab`]: one tab of a document or panel strip, drawn at a rect the strip gives it.

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use craft_theme::{Stroke as Hairline, TypeRole};
use egui::accesskit::{Node, Role};
use egui::emath::GuiRounding as _;
use egui::os::OperatingSystem;
use egui::text::{Galley, LayoutJob};
use egui::{Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use crate::controls::TooltipExt as _;
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{FocusRing, Interactive};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};

/// The words of a strip the host translates: the overflow button's name, the × and the modified
/// state. `{tab}` stands for the tab's label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabStripText {
    /// The name of the overflow button ("More documents", "More panels").
    pub more: String,
    /// The name of a tab's × ("Close {tab}").
    pub close: String,
    /// The name of a modified tab ("{tab}, modified").
    pub modified: String,
}

impl Default for TabStripText {
    fn default() -> Self {
        Self { more: "More tabs".to_owned(), close: "Close {tab}".to_owned(), modified: "{tab}, modified".to_owned() }
    }
}

impl TabStripText {
    /// `pattern` with `{tab}` replaced by `tab`.
    pub fn fill(pattern: &str, tab: &str) -> String {
        pattern.replace("{tab}", tab)
    }
}

/// Largest meaningful extent of a tab, in logical points.
const MAX_EXTENT: f32 = 1_000_000.0;

/// A label the host shaped itself (its own fonts, glyph substitution or styling), laid out
/// without wrapping. Two are equal when they are the same layout.
#[derive(Clone)]
pub(crate) struct Shaped(Arc<Galley>);

impl PartialEq for Shaped {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Debug for Shaped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shaped({:?})", self.0.text())
    }
}

/// One tab, as the host passes it to a strip each frame.
///
/// The key identifies the tab across frames and in events; it is not a position.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "show the tab with `show_at`, or pass it to a strip"]
pub struct Tab<K> {
    pub(crate) key: K,
    pub(crate) id: Option<Id>,
    pub(crate) shaped: Option<Shaped>,
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) icon: Option<Icon>,
    pub(crate) modified: bool,
    pub(crate) closable: bool,
    pub(crate) progress: Option<f32>,
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> Tab<K> {
    /// A tab with this key and label.
    pub fn new(key: K, label: impl Into<String>) -> Self {
        Self { key, id: None, shaped: None, label: label.into(), detail: None, icon: None, modified: false, closable: false, progress: None }
    }

    /// Secondary text after the label (a colour mode, a zoom): it stays readable while the label
    /// is cut, and is drawn in `tab.detail`. Empty text counts as none.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into()).filter(|d| !d.is_empty());
        self
    }

    /// The widget id of the tab, instead of the one derived from the strip and the key
    /// ([`Tab::id`]). For a host that already owns its tabs' ids.
    pub fn with_id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// The label as the host shaped it, laid out without wrapping and in the colour the tab
    /// shows it in ([`TabStyle::label_color`]). The tab cuts it with an ellipsis where the
    /// room is short; the label text stays the tab's name and tooltip.
    pub fn shaped(mut self, galley: Arc<Galley>) -> Self {
        self.shaped = Some(Shaped(galley));
        self
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Unsaved changes: a `*` after the detail, or after the label without one.
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// The tab has a ×. A strip of panels shows none whatever this says.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// A progress bar along the tab's bottom edge, `0.0` to `1.0` (a document opening or saving).
    pub fn progress(mut self, progress: Option<f32>) -> Self {
        self.progress = progress.map(|p| p.clamp(0.0, 1.0));
        self
    }

    /// The tab's key.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The tab's label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The tab's detail text.
    pub fn detail_text(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Whether the tab has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Whether the tab has a ×.
    pub fn is_closable(&self) -> bool {
        self.closable
    }

    /// The widget id of this tab in the strip `strip`: its [`with_id`](Self::with_id) id, else
    /// [`Tab::id`].
    pub fn widget_id(&self, strip: Id) -> Id {
        self.id.unwrap_or_else(|| Self::id(strip, &self.key))
    }

    /// The widget id of the tab with `key` in the strip `strip`.
    pub fn id(strip: Id, key: &K) -> Id {
        strip.with("tab").with(key)
    }

    /// The widget id of the × of the tab with `key` in the strip `strip`.
    pub fn close_id(strip: Id, key: &K) -> Id {
        Self::id(strip, key).with("close")
    }
}

/// A tab's sizes, type roles and colours, from [`Tab::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TabStyle {
    /// Height of the strip: `tab.height`.
    pub height: f32,
    /// Height of a dock group's strip: `tab.panel_height`.
    pub panel_height: f32,
    /// Height of the strip with Large Tabs: `tab.height_large`.
    pub height_large: f32,
    /// `tab.pad_x`.
    pub pad_x: f32,
    /// `tab.gap`.
    pub gap: f32,
    /// `tab.min_width`.
    pub min_width: f32,
    /// Side of the × hit box: `tab.close_box`.
    pub close_box: f32,
    /// Side of the × glyph: `tab.close_icon`.
    pub close_icon: f32,
    /// `tab.overflow`.
    pub overflow: f32,
    /// Top corners of the active tab and of a tab under the pointer: `tab.radius`.
    pub radius: f32,
    /// `tab.dividers`.
    pub dividers: bool,
    /// `tab.drop_width`.
    pub drop_width: f32,
    /// `tab.drop_halo_width` of `tab.drop.halo`.
    pub drop_halo: Stroke,
    /// `tab.progress_height`.
    pub progress_height: f32,
    /// `tab.indicator_width`.
    pub indicator_width: f32,
    /// `tab.tear_off`.
    pub tear_off: f32,
    /// Side of the tab's icon: `size.icon.sm`.
    pub icon_size: f32,
    /// `tab.label` and `tab.label_large`.
    pub label_role: TypeRole,
    pub label_role_large: TypeRole,
    /// `tab.detail_role`.
    pub detail_role: TypeRole,
    /// Corners of the × under the pointer: `radius.control`.
    pub close_radius: f32,
    /// `tab.strip`, and `tab.active` behind the active tab and a tab under the pointer.
    pub strip: Color32,
    pub active: Color32,
    /// `tab.text` and `tab.text.inactive`.
    pub text: Color32,
    pub inactive_text: Color32,
    /// `tab.detail`.
    pub detail: Color32,
    /// The divider at a tab's trailing edge: `tab.divider`, one physical pixel.
    pub divider: Stroke,
    /// `tab.close` and `tab.close.hover`.
    pub close_color: Color32,
    pub close_hover: Color32,
    /// Behind the × under the pointer: `tab.close.fill`.
    pub close_fill: Color32,
    /// `tab.drop`.
    pub drop: Color32,
    /// `tab.indicator`: the bar along the selected tab's top edge, transparent unless high
    /// contrast sets it.
    pub indicator: Color32,
    /// The progress bar: `progress.track` and `progress.fill`.
    pub track: Color32,
    pub fill: Color32,
}

impl TabStyle {
    /// The strip's height, with Large Tabs if `large`.
    pub fn strip_height(&self, large: bool) -> f32 {
        if large { self.height_large } else { self.height }
    }

    /// The label's type role, with Large Tabs if `large`.
    pub fn label_role(&self, large: bool) -> TypeRole {
        if large { self.label_role_large } else { self.label_role }
    }

    /// The fill of a tab in `rect`: the active tab and a tab under the pointer take `tab.active`
    /// with `tab.radius` on the top corners; the others show the strip.
    pub fn paint_surface(&self, painter: &egui::Painter, rect: Rect, filled: bool) {
        if filled {
            let r = corner_radius(self.radius).nw;
            painter.rect_filled(rect, CornerRadius { nw: r, ne: r, sw: 0, se: 0 }, self.active);
        }
    }

    /// The selected tab's indicator: a bar along the top edge of `rect` (the edge the strip
    /// shows beside), or nothing when `tab.indicator` is transparent.
    pub fn paint_indicator(&self, painter: &egui::Painter, rect: Rect) {
        if self.indicator.a() > 0 {
            let bar = Rect::from_min_size(rect.min, vec2(rect.width(), self.indicator_width.min(rect.height())));
            painter.rect_filled(bar, 0.0, self.indicator);
        }
    }

    /// The colour of a tab's label: `tab.text` on the active tab, `tab.text.inactive` on the others,
    /// also under the pointer (PhotoCraft's tabs do not light up on hover).
    pub fn label_color(&self, active: bool) -> Color32 {
        if active { self.text } else { self.inactive_text }
    }

    /// The width of a tab with these text widths: the padding, the icon, the label, the detail,
    /// the mark and the ×, a `gap` between the parts (none before the mark).
    pub fn natural_width(&self, metrics: &TabMetrics, parts: TabParts) -> f32 {
        let mut width = 2.0 * self.pad_x + metrics.label + metrics.mark;
        if parts.icon {
            width += self.icon_size + self.gap;
        }
        if metrics.detail > 0.0 {
            width += self.gap + metrics.detail;
        }
        if parts.close {
            width += self.gap + self.close_box;
        }
        width
    }

    /// Where the parts of a tab in `rect` lie, the label cut first and then the detail. Parts
    /// follow the reading direction: the × before the label on macOS (`close_first`) and after
    /// the detail and mark elsewhere, mirrored when `rtl`.
    pub fn layout(&self, rect: Rect, metrics: &TabMetrics, parts: TabParts, rtl: bool, close_first: bool) -> TabLayout {
        let span = |offset: f32, width: f32| {
            let (a, b) =
                if rtl { (rect.right() - offset - width, rect.right() - offset) } else { (rect.left() + offset, rect.left() + offset + width) };
            Rect::from_x_y_ranges(a..=b, rect.y_range())
        };
        let centred = |at: Rect, side: f32| Rect::from_center_size(at.center(), vec2(side, side));
        let (mut start, mut end) = (self.pad_x, (rect.width() - self.pad_x).max(self.pad_x));
        let mut close = None;
        if parts.close {
            if close_first {
                close = Some(span(start, self.close_box));
                start += self.close_box + self.gap;
            } else {
                close = Some(span(end - self.close_box, self.close_box));
                end -= self.close_box + self.gap;
            }
        }
        let icon = parts.icon.then(|| {
            let at = span(start, self.icon_size);
            start += self.icon_size + self.gap;
            centred(at, self.icon_size)
        });
        let room = (end - start).max(0.0);
        let detail_gap = if metrics.detail > 0.0 { self.gap } else { 0.0 };
        let label = metrics.label.min((room - metrics.mark - detail_gap - metrics.detail).max(0.0));
        let detail = if metrics.detail > 0.0 { metrics.detail.min((room - metrics.mark - label - detail_gap).max(0.0)) } else { 0.0 };
        let detail_at = (detail > 0.0).then(|| span(start + label + self.gap, detail));
        let text_end = if detail > 0.0 { start + label + self.gap + detail } else { start + label };
        TabLayout {
            close,
            icon,
            label: span(start, label),
            detail: detail_at,
            mark: (metrics.mark > 0.0).then(|| span(text_end, metrics.mark)),
            label_cut: label < metrics.label,
            detail_cut: detail < metrics.detail,
        }
    }
}

/// Which parts of a tab are shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TabParts {
    /// The tab has an icon.
    pub icon: bool,
    /// The tab shows its ×.
    pub close: bool,
}

/// The widths of a tab's text runs: the label, the detail (0 without one) and the mark (0 when
/// not modified).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TabMetrics {
    pub label: f32,
    pub detail: f32,
    pub mark: f32,
}

/// Where the parts of a tab lie, from [`TabStyle::layout`]. Every rect is the tab's full height,
/// except the icon's, which is a square at the tab's middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabLayout {
    /// The × hit box, `tab.close_box` wide.
    pub close: Option<Rect>,
    /// The icon.
    pub icon: Option<Rect>,
    /// The label, as wide as it is shown.
    pub label: Rect,
    /// The detail, as wide as it is shown; `None` once the label has taken all the room.
    pub detail: Option<Rect>,
    /// The modified mark, right after the last text.
    pub mark: Option<Rect>,
    /// The label is cut with an ellipsis.
    pub label_cut: bool,
    /// The detail is cut with an ellipsis.
    pub detail_cut: bool,
}

/// How a strip shows one of its tabs.
#[derive(Clone, Copy, Debug)]
pub struct TabView<'a> {
    /// The strip's id: the tab's widget id derives from it and the tab's key ([`Tab::id`]).
    pub strip: Id,
    /// The tab is the strip's selected tab.
    pub active: bool,
    /// The keyboard focus is on this tab (a ring shows while the keyboard is the last input).
    pub focused: bool,
    /// The strip uses Large Tabs.
    pub large: bool,
    /// A divider is drawn at the tab's trailing edge (document tabs in Compact).
    pub divider: bool,
    /// A host drag is over the tab: an outline of `tab.drop` inside it.
    pub drop_highlight: bool,
    /// What the tab senses. `Sense::CLICK` is not focusable, so a strip that is one Tab stop
    /// leaves focus to itself.
    pub sense: Sense,
    /// The tab's zero-based position among all the strip's tabs, hidden ones included.
    pub position: usize,
    /// The words of the strip.
    pub text: &'a TabStripText,
    /// The id of the host's panel node this tab controls.
    pub controls: Option<Id>,
}

/// What [`Tab::show_at`] reports.
#[derive(Debug)]
pub struct TabResponse {
    /// The tab.
    pub tab: Response,
    /// The ×, when shown.
    pub close: Option<Response>,
    /// Where the parts lie.
    pub layout: TabLayout,
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> Tab<K> {
    /// The tab's sizes, type roles and colours.
    pub fn style(tokens: &UiTokens) -> TabStyle {
        let t = &tokens.component.tab;
        let p = &tokens.component.progress;
        TabStyle {
            height: t.height,
            panel_height: t.panel_height,
            height_large: t.height_large,
            pad_x: t.pad_x,
            gap: t.gap,
            min_width: t.min_width,
            close_box: t.close_box,
            close_icon: t.close_icon,
            overflow: t.overflow,
            radius: t.radius,
            dividers: t.dividers,
            drop_width: t.drop_width,
            drop_halo: Stroke::new(t.drop_halo_width, t.drop_halo.to_egui()),
            progress_height: t.progress_height,
            indicator_width: t.indicator_width,
            tear_off: t.tear_off,
            icon_size: tokens.size.icon.sm,
            label_role: t.label,
            label_role_large: t.label_large,
            detail_role: t.detail_role,
            close_radius: tokens.shape.radius.control,
            strip: t.strip.to_egui(),
            active: t.active.to_egui(),
            text: t.text.to_egui(),
            inactive_text: t.inactive_text.to_egui(),
            detail: t.detail.to_egui(),
            divider: Stroke::new(Hairline::Hairline.width(1.0), t.divider.to_egui()),
            close_color: t.close_text.to_egui(),
            close_hover: t.close_hover.to_egui(),
            close_fill: t.close_fill.to_egui(),
            drop: t.drop.to_egui(),
            indicator: t.indicator.to_egui(),
            track: p.track.to_egui(),
            fill: p.fill.to_egui(),
        }
    }

    /// The widths of this tab's text runs at `large`.
    pub fn metrics(&self, ui: &Ui, large: bool) -> TabMetrics {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let width = |text: &str, role: TypeRole| {
            let job = LayoutJob::single_section(text.to_owned(), role_format(&tokens, role, Color32::WHITE));
            ui.painter().layout_job(job).size().x
        };
        TabMetrics {
            label: self.shaped.as_ref().map_or_else(|| width(&self.label, style.label_role(large)), |s| s.0.size().x),
            detail: self.detail.as_deref().map_or(0.0, |d| width(d, style.detail_role)),
            mark: if self.modified { width("*", style.label_role(large)) } else { 0.0 },
        }
    }

    /// The parts this tab shows when its strip shows the × of closable tabs if `close`.
    pub fn parts(&self, close: bool) -> TabParts {
        TabParts { icon: self.icon.is_some(), close: close && self.closable }
    }

    /// The tab's natural width at `large`, with its × if `close` and the tab is closable.
    pub fn natural_width(&self, ui: &Ui, large: bool, close: bool) -> f32 {
        Self::style(&Tokens::of(ui)).natural_width(&self.metrics(ui, large), self.parts(close))
    }

    /// Draws the tab in `rect` and senses it, then its × (registered after the tab, so a press
    /// on the × is the ×'s). The tab does not change any selection: the host reads the
    /// responses. The × is part of the tab when the tab [`closable`](Tab::closable); a strip of
    /// panels clears that before it passes the tab.
    pub fn show_at(&self, ui: &mut Ui, rect: Rect, view: &TabView<'_>) -> TabResponse {
        let style = Self::style(&Tokens::of(ui));
        self.show_with(ui, rect, view, &style)
    }

    /// [`show_at`](Self::show_at) with `style` in place of the theme's: a host that keeps its own
    /// colours or padding changes the fields of [`Tab::style`]'s result. An empty, inverted,
    /// non-finite or oversized `rect` gets an inert, disabled response (still named, so the
    /// accessibility tree lists the tab) and paints nothing.
    pub fn show_with(&self, ui: &mut Ui, rect: Rect, view: &TabView<'_>, style: &TabStyle) -> TabResponse {
        let tokens = Tokens::of(ui);
        let mut style = *style;
        style.divider.width = Hairline::Hairline.width(ui.pixels_per_point());
        let id = self.widget_id(view.strip);
        let size = rect.size();
        if !rect.is_finite() || size.x <= 0.0 || size.y <= 0.0 || size.x > MAX_EXTENT || size.y > MAX_EXTENT {
            let tab = ui.interact(Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 0.0)), id, Sense::hover());
            tab.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, false, view.active, &self.label));
            let layout = style.layout(Rect::ZERO, &TabMetrics::default(), TabParts::default(), false, false);
            return TabResponse { tab, close: None, layout };
        }
        let rtl = direction::is_rtl(ui);
        let metrics = self.metrics(ui, view.large);
        let parts = self.parts(true);
        let close_first = ui.ctx().os() == OperatingSystem::Mac;
        let layout = style.layout(rect, &metrics, parts, rtl, close_first);

        let mut name = self.label.clone();
        if let Some(detail) = &self.detail {
            name = format!("{name}, {detail}");
        }
        if self.modified {
            name = TabStripText::fill(&view.text.modified, &name);
        }
        let sensed = Interactive::with_id(id, Role::Tab, name)
            .sense(view.sense)
            .selected(view.active)
            .state_layer(false)
            .focus_ring(false)
            .position_in_set(view.position)
            .interact_at(ui, rect);
        let tab = sensed.finish(ui);
        ui.ctx().accesskit_node_builder(id, |node: &mut Node| {
            node.clear_toggled();
            if let Some(panel) = view.controls {
                node.push_controlled(panel.accesskit_id());
            }
        });

        let close = layout.close.map(|at| {
            let child = ui.new_child(UiBuilder::new().max_rect(at).accessibility_parent(id));
            let name = TabStripText::fill(&view.text.close, &self.label);
            let sensed = Interactive::with_id(id.with("close"), Role::Button, name)
                .sense(Sense::CLICK)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(&child, at);
            sensed.finish(&child)
        });

        if ui.is_rect_visible(rect) {
            self.paint(ui, &tokens, &style, view, &layout, (&tab, close.as_ref()));
        }
        if layout.label_cut || layout.detail_cut {
            let full = match &self.detail {
                Some(detail) => format!("{}, {detail}", self.label),
                None => self.label.clone(),
            };
            let tab = tab.tooltip(full);
            return TabResponse { tab, close, layout };
        }
        TabResponse { tab, close, layout }
    }

    fn paint(
        &self,
        ui: &Ui,
        tokens: &UiTokens,
        style: &TabStyle,
        view: &TabView<'_>,
        layout: &TabLayout,
        (tab, close): (&Response, Option<&Response>),
    ) {
        let rect = tab.rect;
        let hovered = (tab.hovered() || close.is_some_and(Response::hovered)) && ui.is_enabled();
        let rtl = direction::is_rtl(ui);
        let ppp = ui.pixels_per_point();
        style.paint_surface(ui.painter(), rect, view.active || hovered);
        if view.active {
            style.paint_indicator(ui.painter(), rect);
        }
        if view.divider && style.dividers {
            let x = if rtl { rect.left() + style.divider.width / 2.0 } else { rect.right() - style.divider.width / 2.0 };
            ui.painter().vline(x.round_to_pixel_center(ppp), rect.y_range(), style.divider);
        }
        let text = style.label_color(view.active);
        let large = view.large;
        let run = |ui: &Ui, at: Rect, text_role: TypeRole, string: &str, color: Color32| {
            if at.width() < 1.0 {
                return;
            }
            let mut job = LayoutJob::single_section(string.to_owned(), role_format(tokens, text_role, color));
            job.wrap = egui::text::TextWrapping::truncate_at_width(at.width());
            let galley = ui.painter().layout_job(job);
            let x = if rtl { at.right() - galley.size().x } else { at.left() };
            let y = at.center().y - galley.size().y / 2.0;
            ui.painter().with_clip_rect(at.expand2(vec2(0.0, 1.0))).galley(pos2(x, y).round_to_pixels(ppp), galley, color);
        };
        match &self.shaped {
            Some(Shaped(galley)) if layout.label.width() >= 1.0 => {
                let at = layout.label;
                let galley = if galley.size().x <= at.width() + 0.5 {
                    galley.clone()
                } else {
                    let mut job = (*galley.job).clone();
                    job.wrap = egui::text::TextWrapping::truncate_at_width(at.width());
                    ui.painter().layout_job(job)
                };
                let x = if rtl { at.right() - galley.size().x } else { at.left() };
                let y = at.center().y - galley.size().y / 2.0;
                ui.painter().with_clip_rect(at.expand2(vec2(0.0, 1.0))).galley(pos2(x, y).round_to_pixels(ppp), galley, text);
            }
            Some(_) => {}
            None => run(ui, layout.label, style.label_role(large), &self.label, text),
        }
        if let (Some(at), Some(detail)) = (layout.detail, &self.detail) {
            run(ui, at, style.detail_role, detail, style.detail);
        }
        if let Some(at) = layout.mark {
            run(ui, at, style.label_role(large), "*", text);
        }
        if let (Some(at), Some(icon)) = (layout.icon, self.icon) {
            icons::paint_in(ui, at, icon, icons::active(ui.ctx()), style.icon_size, IconSize::Sm.stroke(), text);
        }
        if let (Some(at), Some(response)) = (layout.close, close) {
            let on = response.hovered();
            if on {
                ui.painter().rect_filled(at, corner_radius(style.close_radius), style.close_fill);
            }
            let color = if on { style.close_hover } else { style.close_color };
            icons::paint_in(ui, at, Icon::Close, icons::active(ui.ctx()), style.close_icon, IconSize::Sm.stroke(), color);
        }
        if let Some(progress) = self.progress {
            let bar = Rect::from_min_max(
                pos2(rect.left() + style.gap, rect.bottom() - 1.0 - style.progress_height),
                pos2(rect.right() - style.gap, rect.bottom() - 1.0),
            );
            ui.painter().rect_filled(bar, 0.0, style.track);
            let filled = Rect::from_x_y_ranges(
                if rtl { bar.right() - bar.width() * progress..=bar.right() } else { bar.left()..=bar.left() + bar.width() * progress },
                bar.y_range(),
            );
            ui.painter().rect_filled(filled, 0.0, style.fill);
        }
        if view.drop_highlight {
            let halo = style.drop_halo;
            let band = style.drop_width + 2.0 * halo.width;
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(band, halo.color), StrokeKind::Inside);
            ui.painter().rect_stroke(rect.shrink(halo.width), 0.0, Stroke::new(style.drop_width, style.drop), StrokeKind::Inside);
        }
        let ring = FocusRing::style(tokens);
        FocusRing::paint_if(ui, rect.shrink(2.0 + ring.outset()), CornerRadius::ZERO, view.focused);
    }
}

pub(super) fn stories() -> Vec<Story> {
    vec![Story::new(
        "composites/tab/states",
        Level::Composites,
        "Tab",
        "States",
        || Args::new().with_bool("large", false).with_bool("dividers", true).with_number("width", 180.0, 40.0, 320.0),
        show_states,
    )]
}

/// Tabs in each state on a strip: inactive, hovered by the pointer, active, modified, with a
/// detail, with an icon, opening with progress, a long label cut with its tooltip, a drop
/// highlight and the keyboard focus ring.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let (large, dividers, width) = (args.bool("large"), args.bool("dividers"), args.number("width") as f32);
    let tokens = Tokens::of(ui);
    let style = Tab::<u8>::style(&tokens);
    let height = style.strip_height(large);
    let text = TabStripText::default();
    let strip = ui.make_persistent_id("tab-states");
    let rows: Vec<(Tab<u8>, bool, bool, bool)> = vec![
        (Tab::new(0, "Inactive"), false, false, false),
        (Tab::new(1, "Active").closable(true), true, false, false),
        (Tab::new(2, "Modified").modified(true).closable(true), false, false, false),
        (Tab::new(3, "Detail").detail("RGB/8").closable(true), false, false, false),
        (Tab::new(4, "Photo").detail("RGB/8").modified(true).icon(Icon::Close).closable(true), true, false, false),
        (Tab::new(5, "Opening").progress(Some(0.4)).closable(true), false, false, false),
        (Tab::new(6, "A title too long for the tab to show whole").detail("50%").closable(true), false, false, false),
        (Tab::new(7, "Drop target"), false, true, false),
        (Tab::new(8, "Keyboard focus"), false, false, true),
    ];
    ui.spacing_mut().item_spacing.y = 0.0;
    for (index, (tab, active, highlight, focused)) in rows.iter().enumerate() {
        let (_, row) = ui.allocate_space(vec2(width, height));
        ui.painter().rect_filled(row, 0.0, style.strip);
        let view = TabView {
            strip,
            active: *active,
            focused: *focused,
            large,
            divider: dividers,
            drop_highlight: *highlight,
            sense: Sense::CLICK,
            position: index,
            text: &text,
            controls: None,
        };
        tab.show_at(ui, row, &view);
    }
}
