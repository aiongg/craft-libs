//! [`Histogram`]: a plot of bins, one tone channel or RGB ribbons.

use craft_theme::{Mode, Rgba, TypeRole};
use egui::accesskit::ActionData;
use egui::accesskit::{Action, Role};
use egui::{
    Color32, CornerRadius, CursorIcon, EventFilter, Id, Key, Mesh, Modifiers, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};
use std::ops::RangeInclusive;

use crate::controls::TooltipExt as _;
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{Corner, FocusRing, Interactive};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, tokens_in_mode};
use crate::typed_story;

/// The colour of a channel in curves and histograms: a data token.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChannelColor {
    /// The composite, CMYK Black and Lab Lightness: `data.luma` (`text.muted` in a histogram).
    #[default]
    Neutral,
    /// `data.red`.
    Red,
    /// `data.green`.
    Green,
    /// `data.blue`.
    Blue,
    /// `data.cyan`.
    Cyan,
    /// `data.magenta`, also Lab a.
    Magenta,
    /// `data.yellow`, also Lab b.
    Yellow,
}

impl ChannelColor {
    /// Every channel colour, in declaration order.
    pub const ALL: [ChannelColor; 7] = [
        ChannelColor::Neutral,
        ChannelColor::Red,
        ChannelColor::Green,
        ChannelColor::Blue,
        ChannelColor::Cyan,
        ChannelColor::Magenta,
        ChannelColor::Yellow,
    ];

    /// The channel's data token in `tokens`.
    pub fn color(self, tokens: &UiTokens) -> Color32 {
        let d = &tokens.color.data;
        match self {
            ChannelColor::Neutral => d.luma,
            ChannelColor::Red => d.red,
            ChannelColor::Green => d.green,
            ChannelColor::Blue => d.blue,
            ChannelColor::Cyan => d.cyan,
            ChannelColor::Magenta => d.magenta,
            ChannelColor::Yellow => d.yellow,
        }
        .to_egui()
    }

    /// The colour of the channel's curve: `text.normal` for a neutral channel, otherwise its data
    /// token.
    pub fn curve(self, tokens: &UiTokens) -> Color32 {
        match self {
            ChannelColor::Neutral => tokens.component.curve.point_stroke.to_egui(),
            other => other.color(tokens),
        }
    }

    /// The light end of the channel's gradient bars: white for a neutral channel, otherwise the
    /// theme's data token for the channel. The dark end is black in every theme (image data).
    pub(super) fn bar(self, tokens: &UiTokens) -> Color32 {
        match self {
            ChannelColor::Neutral => Rgba::WHITE.to_egui(),
            other => other.color(tokens),
        }
    }

    /// The name in story args and dumps.
    pub const fn name(self) -> &'static str {
        match self {
            ChannelColor::Neutral => "neutral",
            ChannelColor::Red => "red",
            ChannelColor::Green => "green",
            ChannelColor::Blue => "blue",
            ChannelColor::Cyan => "cyan",
            ChannelColor::Magenta => "magenta",
            ChannelColor::Yellow => "yellow",
        }
    }
}

/// How tall the bins are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HistogramScale {
    /// The tallest bin (the tallest interior bin of an RGB plot, whose edge bins are clipped
    /// pixels) at 1/1.1 of the height: Photoshop's and PhotoCraft's.
    #[default]
    Peak,
    /// Camera Raw's: the 95th-percentile column at two thirds of the height; taller spikes run
    /// off the top.
    Typical,
}

/// How a [`HistogramData`] is drawn by [`Histogram::paint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HistogramLook {
    /// One channel as a filled area in its colour.
    Tone,
    /// One grey, `histogram.backdrop`, in every channel, to sit behind a curve.
    Backdrop,
    /// Overlapping ribbons: filled bands with exact overlap colours and an outline each.
    Rgb,
}

/// A count a histogram's bin holds.
pub trait BinCount: Copy {
    /// The count as a height before scaling.
    fn height(self) -> f32;
}

impl BinCount for u32 {
    fn height(self) -> f32 {
        self as f32
    }
}

impl BinCount for u64 {
    fn height(self) -> f32 {
        self as f32
    }
}

/// The bins of a histogram and how tall they are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct HistogramData {
    plot: Plot,
    scale: HistogramScale,
}

#[derive(Clone, Debug, PartialEq)]
enum Plot {
    Tone { bins: Vec<f32>, channel: ChannelColor },
    Channels { channels: Vec<(Vec<f32>, ChannelColor)>, blend: Blend },
}

/// How the fills of overlapping channels combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Blend {
    /// Channel-wise sum, saturating: light on the dark scope (red over green is yellow, all three grey).
    Add,
    /// Channel-wise product: inks on paper.
    Multiply,
}

impl HistogramData {
    /// One channel's bins, drawn across the plot width, in `channel`'s colour.
    pub fn tone<B: BinCount>(bins: &[B], channel: ChannelColor) -> Self {
        Self { plot: Plot::Tone { bins: bins.iter().map(|b| b.height()).collect(), channel }, scale: HistogramScale::Peak }
    }

    /// Red, green and blue bins, drawn as ribbons. The three slices should have one length.
    pub fn rgb<B: BinCount>(channels: [&[B]; 3]) -> Self {
        let colors = [ChannelColor::Red, ChannelColor::Green, ChannelColor::Blue];
        let channels = channels.iter().zip(colors).map(|(bins, color)| (bins.iter().map(|b| b.height()).collect(), color)).collect();
        Self { plot: Plot::Channels { channels, blend: Blend::Add }, scale: HistogramScale::Peak }
    }

    /// Any number of channels, each `(bins, color)`, as opaque ribbons whose fills multiply where
    /// they overlap: the inks of a CMYK or spot-colour image. The slices should have one length.
    pub fn channels<B: BinCount>(channels: &[(&[B], ChannelColor)]) -> Self {
        let channels = channels.iter().take(MAX_CHANNELS).map(|(bins, color)| (bins.iter().map(|b| b.height()).collect(), *color)).collect();
        Self { plot: Plot::Channels { channels, blend: Blend::Multiply }, scale: HistogramScale::Peak }
    }

    /// How tall the bins are drawn; [`HistogramScale::Peak`] by default.
    pub fn scale(mut self, scale: HistogramScale) -> Self {
        self.scale = scale;
        self
    }
}

/// A histogram's look, from [`Histogram::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistogramStyle {
    /// Height of the plot: `histogram.height`.
    pub height: f32,
    /// The tone plot's box: `field.fill`.
    pub fill: Color32,
    /// The box's edge: `field.border.rest`.
    pub border: Color32,
    /// The plot behind RGB ribbons: `data.scope`, dark in every theme.
    pub scope: Color32,
    /// The tone look's colour per [`ChannelColor`], in [`ChannelColor::ALL`] order: the data
    /// tokens, `text.muted` for neutral. Read it with [`tone`](HistogramStyle::tone).
    pub tones: [Color32; 7],
    /// The tone look's outline in the neutral channel: `text.normal`, so it stands out from the
    /// `text.muted` neutral fill.
    pub outline_neutral: Color32,
    /// The ribbons' colours, from the tokens passed to [`Histogram::style`]: `data.red`,
    /// `data.green`, `data.blue`. [`Histogram::show`] reads them from the dark mode's tokens, since
    /// the scope is dark in every theme.
    pub ribbons: [Color32; 3],
    /// The colour of each [`ChannelColor`] on the scope, in [`ChannelColor::ALL`] order: its data
    /// token, `data.luma` for neutral. Read it with [`ink`](HistogramStyle::ink).
    pub inks: [Color32; 7],
    /// The tone look behind a curve, in every channel: `histogram.backdrop`.
    pub backdrop: Color32,
    /// The selected range's band: `accent.solid` at `histogram.selection_alpha`.
    pub selection: Color32,
    /// Width of the ribbons' outlines: `histogram.line`.
    pub line: f32,
    /// Width of a marker tick and of the selection's edges: `histogram.marker_width`.
    pub marker_width: f32,
    /// Height of a marker tick, from the bottom of the plot: `histogram.marker_height`.
    pub marker_height: f32,
    /// The stale mark: `status.warn.fg`.
    pub stale: Color32,
}

impl HistogramStyle {
    /// The tone look's colour for `channel`.
    pub fn tone(&self, channel: ChannelColor) -> Color32 {
        self.tones[channel as usize]
    }

    /// The ribbon colour of `channel` on the scope.
    pub fn ink(&self, channel: ChannelColor) -> Color32 {
        self.inks[channel as usize]
    }
}

/// A plot of bins: one tone channel (Levels, the Histogram panel) or overlapping ribbons (Camera
/// Raw's red, green and blue, a print's inks). It senses nothing and takes no focus unless the host
/// asks for [`selection`](Histogram::selection) or [`on_stale_click`](Histogram::on_stale_click).
///
/// ```
/// use craft_ui::composites::{ChannelColor, Histogram};
/// # egui::__run_test_ui(|ui| {
/// let bins = [0u32, 4, 9, 20, 9, 4, 0];
/// Histogram::new("levels", "Histogram").tone(&bins, ChannelColor::Red).value("Red channel").show(ui);
/// # });
/// ```
///
/// - **Bins:** any count, drawn across the plot width, each bin centred in its slot.
/// - **Tone look:** a filled area on `field.fill`, with a `field.border.rest` edge only when
///   [`bordered`](Histogram::bordered), in the channel's data token (`text.muted` for neutral).
/// - **RGB look** ([`rgb`](Histogram::rgb), [`channels`](Histogram::channels)): the bins smoothed
///   for display with the five-bin binomial kernel (the two edge bins pinned), one shared scale
///   and a `histogram.line` outline per channel. Fills are opaque: `rgb` adds the channel colours where fills overlap
///   (saturating, as light), `channels` takes their product (as inks). The plot sits on `data.scope` and draws the dark mode's data
///   colours.
/// - **Outline:** the original histogram as a `histogram.line` line over the plot, on the plot's
///   scale in the tone look (a taller original is clamped to the top of the plot) and on its own
///   scale in the RGB look.
/// - **Marker, markers:** a tick at a level, standing on the bottom of the plot (PhotoCraft's sampler mark); markers add a caption beside the tick, level with its top.
/// - **Selection:** a band over the whole bin slots of a range of levels with 2 pt edges, dragged
///   with the pointer or moved with the arrow keys (Shift moves the start edge) while the plot
///   has focus. A click or Esc clears it. Levels are bin positions, whole numbers.
/// - **Stale:** a warning icon at the top trailing corner with a tooltip; with
///   [`on_stale_click`](Histogram::on_stale_click) it is a button.
/// - **AccessKit:** one `Image` node named by the label, whose value is [`value`](Histogram::value);
///   the stale mark and each marker caption are named nodes too. A selection is two `Slider`
///   nodes beneath the plot's node, its start and its end level, with Increment and Decrement.
#[must_use = "show the histogram with `show`"]
pub struct Histogram {
    id_salt: egui::IdSalt,
    label: String,
    data: Option<HistogramData>,
    value: Option<String>,
    marker: Option<f32>,
    markers: Vec<(f32, String)>,
    outline: Option<(Vec<f32>, ChannelColor)>,
    selection: Option<Option<RangeInclusive<f32>>>,
    selection_labels: [String; 2],
    stale: bool,
    stale_label: String,
    stale_click: Option<String>,
    height: Option<f32>,
    bordered: bool,
}

/// What [`Histogram::show`] returns. It dereferences to the plot's [`egui::Response`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HistogramResponse {
    /// The plot's response.
    pub response: egui::Response,
    /// What the user did to the selection this frame: `None` when [`Histogram::selection`] is off
    /// or nothing changed, `Some(Some(range))` for a range set by dragging, the keys or a screen
    /// reader (in levels), `Some(None)` when a click or Esc cleared it. The host stores the inner
    /// value and passes it back to [`Histogram::selection`]:
    /// `if let Some(new) = shown.selection { stored = new; }`.
    pub selection: Option<Option<RangeInclusive<f32>>>,
    /// The whole level under the pointer while it is over the plot or dragging from it, clamped to
    /// the bins; `None` otherwise.
    pub hovered_level: Option<f32>,
    /// The stale mark was clicked; only with [`Histogram::on_stale_click`].
    pub clicked: bool,
}

impl std::ops::Deref for HistogramResponse {
    type Target = egui::Response;

    fn deref(&self) -> &egui::Response {
        &self.response
    }
}

/// Channels a plot draws; the overlap masks are 32-bit.
const MAX_CHANNELS: usize = 32;

/// The keys a focused plot keeps from focus navigation; Esc only while there is a selection to clear.
const fn selection_keys(clearable: bool) -> EventFilter {
    EventFilter { horizontal_arrows: true, vertical_arrows: false, tab: false, escape: clearable }
}

impl Histogram {
    /// An empty histogram named `label`. `id_salt` is its id.
    pub fn new(id_salt: impl egui::AsIdSalt, label: impl Into<String>) -> Self {
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            label: label.into(),
            data: None,
            value: None,
            marker: None,
            markers: Vec::new(),
            outline: None,
            selection: None,
            selection_labels: ["Selection start".to_owned(), "Selection end".to_owned()],
            stale: false,
            stale_label: "Cached data".to_owned(),
            stale_click: None,
            height: None,
            bordered: false,
        }
    }

    /// One channel in the tone look.
    pub fn tone<B: BinCount>(mut self, bins: &[B], channel: ChannelColor) -> Self {
        self.data = Some(HistogramData::tone(bins, channel));
        self
    }

    /// Three channels in the RGB look.
    pub fn rgb<B: BinCount>(mut self, channels: [&[B]; 3]) -> Self {
        self.data = Some(HistogramData::rgb(channels));
        self
    }

    /// Any number of channels (inks), each `(bins, color)`, in the RGB look with the fills
    /// multiplied where they overlap.
    pub fn channels<B: BinCount>(mut self, channels: &[(&[B], ChannelColor)]) -> Self {
        self.data = Some(HistogramData::channels(channels));
        self
    }

    /// Bins in either look, as a host keeps them.
    pub(super) fn data(mut self, data: HistogramData) -> Self {
        self.data = Some(data);
        self
    }

    /// How tall the bins are drawn; [`HistogramScale::Peak`] by default.
    pub fn scale(mut self, scale: HistogramScale) -> Self {
        self.data = self.data.map(|d| d.scale(scale));
        self
    }

    /// An outline of the original histogram over the current one, in `channel`'s colour. In the
    /// tone look it is drawn on the plot's own scale, so a taller original is clamped to the top of
    /// the plot; in the RGB look it has its own scale.
    pub fn outline<B: BinCount>(mut self, bins: &[B], channel: ChannelColor) -> Self {
        self.outline = Some((bins.iter().map(|b| b.height()).collect(), channel));
        self
    }

    /// A marker at `level`, a position in bins (a bin's centre is its index): the sampled pixel's
    /// tone in PhotoCraft.
    pub fn marker(mut self, level: Option<f32>) -> Self {
        self.marker = level;
        self
    }

    /// Labelled markers, each `(level, caption)`: the black point, white point and mid of a
    /// levels adjustment. The caption sits beside its tick, its bottom level with the tick's top,
    /// on the side with room; the host translates it.
    pub fn markers(mut self, markers: &[(f32, String)]) -> Self {
        self.markers = markers.to_vec();
        self
    }

    /// A selected range of levels (bin positions) that the user changes by dragging over the plot
    /// or with the arrow keys and clears with a click or Esc; `None` is no selection. The change
    /// is [`HistogramResponse::selection`]; the host stores it and passes it back. A selection
    /// is two `Slider` nodes for screen readers, named by
    /// [`selection_labels`](Histogram::selection_labels).
    pub fn selection(mut self, range: Option<RangeInclusive<f32>>) -> Self {
        self.selection = Some(range);
        self
    }

    /// The accessible names of the selection's start and end sliders; "Selection start" and
    /// "Selection end" by default. The host translates them.
    pub fn selection_labels(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.selection_labels = [start.into(), end.into()];
        self
    }

    /// Marks the data as older than the image: a warning icon at the top trailing corner.
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// The stale mark's tooltip and accessible name; "Cached data" by default. The host
    /// translates it.
    pub fn stale_label(mut self, label: impl Into<String>) -> Self {
        self.stale_label = label.into();
        self
    }

    /// Makes the stale mark a button named `label` ("Recalculate"), reported as
    /// [`HistogramResponse::clicked`]. It replaces [`stale_label`](Histogram::stale_label) as the
    /// mark's name and tooltip.
    pub fn on_stale_click(mut self, label: impl Into<String>) -> Self {
        self.stale_click = Some(label.into());
        self
    }

    /// What a screen reader reads after the label, e.g. "Red channel".
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The plot's height in points; `histogram.height` by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Draws the 1 point `field.border.rest` edge round the plot. Off by default, as the Histogram
    /// panel has none; a levels editor turns it on.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// The look in `tokens`. The ribbon colours are those of `tokens`; [`show`](Histogram::show)
    /// uses the dark mode's.
    pub fn style(tokens: &UiTokens) -> HistogramStyle {
        let (h, d) = (&tokens.component.histogram, &tokens.color.data);
        HistogramStyle {
            height: h.height,
            fill: tokens.component.field.fill.rest.to_egui(),
            border: tokens.component.field.border.rest.to_egui(),
            scope: d.scope.to_egui(),
            tones: ChannelColor::ALL.map(|c| if c == ChannelColor::Neutral { tokens.color.text.muted.to_egui() } else { c.color(tokens) }),
            outline_neutral: tokens.color.text.normal.to_egui(),
            ribbons: [d.red.to_egui(), d.green.to_egui(), d.blue.to_egui()],
            inks: ChannelColor::ALL.map(|c| c.color(tokens)),
            backdrop: h.backdrop.to_egui(),
            selection: tokens.color.accent.solid.to_egui().gamma_multiply(h.selection_alpha),
            line: h.line,
            marker_width: h.marker_width,
            marker_height: h.marker_height,
            stale: tokens.color.status.warn.fg.to_egui(),
        }
    }

    /// Draws `data` in `look` into `rect` with `painter`: no box, no layout, no interaction, nothing
    /// in the AccessKit tree. A curve editor paints [`HistogramLook::Backdrop`] behind its curve.
    /// `data` that doesn't fit the look (RGB data in the tone look) and a degenerate `rect` draw
    /// nothing. [`HistogramLook::Rgb`] draws the colours of `tokens`: pass the dark mode's, as
    /// [`show`](Histogram::show) does.
    pub fn paint(painter: &Painter, rect: Rect, data: &HistogramData, look: HistogramLook, tokens: &UiTokens) {
        painter.extend(plot_shapes(rect, data, look, &Histogram::style(tokens)));
    }

    /// Shows the histogram across the available width.
    pub fn show(self, ui: &mut Ui) -> HistogramResponse {
        let tokens = Tokens::of(ui);
        let mut style = Histogram::style(&tokens);
        let rgb = matches!(self.data, Some(HistogramData { plot: Plot::Channels { .. }, .. }));
        if rgb {
            let dark = Histogram::style(&tokens_in_mode(ui.ctx(), Mode::Dark));
            (style.ribbons, style.inks) = (dark.ribbons, dark.inks);
        }
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), self.height.unwrap_or(style.height)), Sense::hover());
        let id = ui.make_persistent_id(self.id_salt);
        let bins = self.data.as_ref().map_or(0, HistogramData::bin_count);
        let selectable = self.selection.is_some() && bins > 0 && ui.is_enabled();
        let response = ui.interact(rect, id, if selectable { Sense::click_and_drag() } else { Sense::hover() });
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::Image, ui.is_enabled(), &self.label);
            info.current_text_value.clone_from(&self.value);
            info
        });
        let selection = if selectable { self.edit_selection(ui, &response, id, rect, bins) } else { None };
        let shown = selection.clone().unwrap_or_else(|| self.selection.clone().flatten());
        let hovered_level = (bins > 0 && (response.hovered() || response.dragged()))
            .then(|| response.interact_pointer_pos().or_else(|| response.hover_pos()))
            .flatten()
            .map(|at| level_at(rect, bins, at.x));
        let mut clicked = false;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_filled(rect, 0.0, if rgb { style.scope } else { style.fill });
            if let Some(data) = &self.data {
                let look = if rgb { HistogramLook::Rgb } else { HistogramLook::Tone };
                painter.extend(plot_shapes(rect, data, look, &style));
                if let Some((bins, channel)) = &self.outline {
                    painter.extend(outline_shapes(rect, data, bins, *channel, &style));
                }
                let solid = if rgb { sum(&style.ribbons, 0b111) } else { tokens.color.text.normal.to_egui() };
                if let Some(range) = &shown {
                    painter.extend(selection_shapes(rect, bins, range, style.marker_width, style.selection));
                }
                if let Some(level) = self.marker {
                    painter.extend(marker_shapes(rect, bins, level, (style.marker_width, style.marker_height), solid));
                }
                for (i, (level, caption)) in self.markers.iter().enumerate() {
                    painter.extend(marker_shapes(rect, bins, *level, (style.marker_width, style.marker_height), solid));
                    Self::caption(
                        ui,
                        id.with(("marker", i)),
                        rect,
                        level_x(rect, bins, *level),
                        (style.marker_width, style.marker_height),
                        caption,
                        solid,
                        &tokens,
                    );
                }
            }
            if self.bordered {
                painter.rect_stroke(rect, 0.0, Stroke::new(craft_theme::Stroke::W1.width(ui.pixels_per_point()), style.border), StrokeKind::Inside);
            }
            if selectable {
                FocusRing::paint_radius(ui, rect, CornerRadius::ZERO, &response);
            }
            if self.stale {
                clicked = self.stale_mark(ui, id, rect, &tokens, style.stale);
            }
        }
        if bins > 0
            && let Some(range) = &shown
        {
            self.selection_nodes(ui, id, rect, bins, range, style.marker_width);
        }
        HistogramResponse { response, selection, hovered_level, clicked }
    }

    /// The selection as two sliders, beneath the plot's node, at the band's edges.
    fn selection_nodes(&self, ui: &mut Ui, id: Id, rect: Rect, bins: usize, range: &RangeInclusive<f32>, width: f32) {
        let Some(band) = selection_band(rect, bins, range) else { return };
        let (start, end) = ordered(range);
        let last = bins as f32 - 1.0;
        let nodes = ui.new_child(UiBuilder::new().max_rect(rect).accessibility_parent(id));
        let sliders = [(start, 0.0..=end), (end, start..=last)];
        for (i, (edge, (value, bounds))) in selection_edges(band, width).into_iter().zip(sliders).enumerate() {
            Interactive::with_id(selection_node(id, i), Role::Slider, self.selection_labels[i].as_str())
                .sense(Sense::hover())
                .enabled(ui.is_enabled())
                .state_layer(false)
                .focus_ring(false)
                .interact_at(&nodes, edge)
                .numeric(f64::from(value), f64::from(*bounds.start())..=f64::from(*bounds.end()), 1.0)
                .finish(&nodes);
        }
    }

    /// What the pointer, the keys or a screen reader did to the selection this frame.
    fn edit_selection(&self, ui: &Ui, response: &egui::Response, id: Id, rect: Rect, bins: usize) -> Option<Option<RangeInclusive<f32>>> {
        let current = self.selection.clone().flatten();
        let level = |x: f32| level_at(rect, bins, x);
        let last = bins as f32 - 1.0;
        let mut changed = None;
        if response.hovered() || response.dragged() {
            ui.set_cursor_icon(CursorIcon::Crosshair);
        }
        if response.dragged()
            && let Some((from, to)) = ui.input(|i| i.pointer.press_origin().zip(i.pointer.interact_pos()))
        {
            let (a, b) = (level(from.x), level(to.x));
            changed = Some(Some(a.min(b)..=a.max(b)));
        } else if response.clicked() && current.is_some() && ui.input(|i| i.pointer.any_released()) {
            // Not a click from Enter or Space, which activate nothing here.
            changed = Some(None);
        }
        if let Some(range) = &current {
            let (start, end) = ordered(range);
            let moved = [(0, start, 0.0..=end), (1, end, start..=last)].map(|(i, value, bounds)| requested(ui, selection_node(id, i), value, bounds));
            if moved.iter().any(Option::is_some) {
                changed = Some(Some(moved[0].unwrap_or(start)..=moved[1].unwrap_or(end)));
            }
        }
        if response.has_focus() {
            ui.memory_mut(|m| m.set_focus_lock_filter(response.id, selection_keys(current.is_some())));
            let (start_step, end_step) = ui.input_mut(|i| {
                let mut step = |modifiers| i8::from(i.consume_key(modifiers, Key::ArrowRight)) - i8::from(i.consume_key(modifiers, Key::ArrowLeft));
                let start = step(Modifiers::SHIFT);
                (start, step(Modifiers::NONE))
            });
            if (start_step, end_step) != (0, 0) {
                let middle = self.marker.unwrap_or((bins as f32 - 1.0) / 2.0).round();
                let range = current.clone().unwrap_or(middle..=middle);
                let start = (range.start() + f32::from(start_step)).clamp(0.0, last);
                let end = (range.end() + f32::from(end_step)).clamp(0.0, last);
                changed = Some(Some(start.min(end)..=start.max(end)));
            }
            if current.is_some() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                changed = Some(None);
            }
        }
        changed
    }

    /// Paints a marker's caption beside its tick, the text's bottom level with the tick's top, and
    /// names it for screen readers. `tick` is the tick's width and height.
    #[allow(clippy::too_many_arguments)]
    fn caption(ui: &Ui, id: egui::Id, rect: Rect, x: f32, tick: (f32, f32), caption: &str, color: Color32, tokens: &UiTokens) {
        if !usable(rect) || !x.is_finite() {
            return;
        }
        let gap = tokens.space.gap.inline;
        let galley = ui.painter().layout_no_wrap(caption.to_owned(), tokens.font(TypeRole::Caption), color);
        let (before, after) = (x - tick.0 / 2.0 - gap, x + tick.0 / 2.0 + gap);
        let left = if after + galley.size().x <= rect.right() { after } else { (before - galley.size().x).max(rect.left()) };
        let at = pos2(left, rect.bottom() - tick.1 - galley.size().y);
        let area = Rect::from_min_size(at, galley.size()).intersect(rect);
        ui.painter().with_clip_rect(rect).galley(at, galley, color);
        let node = ui.interact(area, id, Sense::hover());
        node.widget_info(|| WidgetInfo::labeled(Role::Image, ui.is_enabled(), caption));
    }

    /// The stale mark; true when it was clicked.
    fn stale_mark(&self, ui: &Ui, id: egui::Id, rect: Rect, tokens: &UiTokens, color: Color32) -> bool {
        let side = IconSize::Sm.points(tokens);
        let gap = tokens.space.gap.inline;
        let x = direction::end_x(ui, rect) - direction::forward(ui) * (gap + side / 2.0);
        let mark = Rect::from_center_size(pos2(x, rect.top() + gap + side / 2.0), vec2(side, side));
        icons::paint(ui, mark, Icon::Warning, IconSize::Sm, color);
        let (sense, role, label) = match &self.stale_click {
            Some(label) => (Sense::click(), Role::Button, label),
            None => (Sense::hover(), Role::Image, &self.stale_label),
        };
        let stale = ui.interact(mark, id.with("stale"), sense);
        stale.widget_info(|| WidgetInfo::labeled(role, ui.is_enabled(), label));
        if self.stale_click.is_some() {
            FocusRing::paint(ui, mark, Corner::Control, &stale);
        }
        let clicked = stale.clicked();
        stale.tooltip(label);
        clicked
    }
}

impl HistogramData {
    fn bin_count(&self) -> usize {
        match &self.plot {
            Plot::Tone { bins, .. } => bins.len(),
            Plot::Channels { channels, .. } => channels.first().map_or(0, |(bins, _)| bins.len()),
        }
    }
}

/// The colour of the area under every channel of `mask` (bit `c` for channel `c`): the
/// channel-wise saturating sum of their colours (PhotoCraft's `mix`), black under none. It doesn't
/// depend on drawing order.
pub(super) fn sum(colors: &[Color32], mask: u32) -> Color32 {
    let (mut r, mut g, mut b) = (0u8, 0u8, 0u8);
    for (c, color) in colors.iter().enumerate() {
        if mask & (1 << c) != 0 {
            (r, g, b) = (r.saturating_add(color.r()), g.saturating_add(color.g()), b.saturating_add(color.b()));
        }
    }
    Color32::from_rgb(r, g, b)
}

/// The colour of the area under every channel of `mask`, multiplied channel-wise: white under
/// none.
pub(super) fn product(colors: &[Color32], mask: u32) -> Color32 {
    let mut rgb = [255u16; 3];
    for (c, color) in colors.iter().enumerate() {
        if mask & (1 << c) != 0 {
            for (acc, v) in rgb.iter_mut().zip([color.r(), color.g(), color.b()]) {
                *acc = (*acc * u16::from(v) + 127) / 255;
            }
        }
    }
    Color32::from_rgb(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8)
}

impl Blend {
    fn color(self, colors: &[Color32], mask: u32) -> Color32 {
        match self {
            Blend::Add => sum(colors, mask),
            Blend::Multiply => product(colors, mask),
        }
    }
}

fn usable(rect: Rect) -> bool {
    rect.is_finite() && rect.is_positive() && rect.size().is_finite()
}

/// The shapes of `data` in `look` inside `rect`.
pub(super) fn plot_shapes(rect: Rect, data: &HistogramData, look: HistogramLook, style: &HistogramStyle) -> Vec<Shape> {
    if !usable(rect) {
        return Vec::new();
    }
    match (&data.plot, look) {
        (Plot::Tone { bins, channel }, HistogramLook::Tone | HistogramLook::Backdrop) => {
            let color = if look == HistogramLook::Backdrop { style.backdrop } else { style.tone(*channel) };
            vec![Shape::mesh(tone_mesh(rect, &tone_heights(bins, data.scale), color))]
        }
        (Plot::Channels { channels, blend }, HistogramLook::Rgb) => {
            let colors: Vec<Color32> = channels.iter().map(|(_, channel)| style.ink(*channel)).collect();
            let bins: Vec<Vec<f32>> = channels.iter().map(|(bins, _)| bins.clone()).collect();
            let heights = rgb_heights(&bins, data.scale);
            let mut shapes = vec![Shape::mesh(rgb_mesh(rect, &heights, &colors, *blend))];
            for (values, color) in heights.iter().zip(colors) {
                if values.iter().any(|&v| v > 0.0) {
                    shapes.push(Shape::line(outline(rect, values), Stroke::new(style.line, color)));
                }
            }
            shapes
        }
        _ => Vec::new(),
    }
}

/// The marker tick of `width` by `height` at bin position `level`, standing on the bottom of `rect`,
/// snapped to whole points and clipped to `rect`.
pub(super) fn marker_shapes(rect: Rect, bins: usize, level: f32, (width, height): (f32, f32), color: Color32) -> Vec<Shape> {
    if !usable(rect) || bins == 0 || !level.is_finite() {
        return Vec::new();
    }
    let left = (level_x(rect, bins, level) - width / 2.0).round();
    let band = Rect::from_min_max(pos2(left, rect.bottom() - height), pos2(left + width, rect.bottom())).intersect(rect);
    vec![Shape::rect_filled(band, 0.0, color)]
}

/// The x of bin position `level`, clamped to the plot.
fn level_x(rect: Rect, bins: usize, level: f32) -> f32 {
    rect.left() + ((level + 0.5) / bins.max(1) as f32).clamp(0.0, 1.0) * rect.width()
}

/// The whole bin position under `x`, clamped to the bins. A bin owns the whole slot it is centred in.
fn level_at(rect: Rect, bins: usize, x: f32) -> f32 {
    ((x - rect.left()) / rect.width() * bins as f32 - 0.5).round().clamp(0.0, bins.saturating_sub(1) as f32)
}

/// The range's ends in ascending order.
fn ordered(range: &RangeInclusive<f32>) -> (f32, f32) {
    let (a, b) = (*range.start(), *range.end());
    (a.min(b), a.max(b))
}

/// The whole bin slots of `range`, from the first bin's left edge to the last bin's right edge, so
/// a one-bin range has the bin's width. `None` for a degenerate plot or a range that isn't finite.
fn selection_band(rect: Rect, bins: usize, range: &RangeInclusive<f32>) -> Option<Rect> {
    if !usable(rect) || bins == 0 || !range.start().is_finite() || !range.end().is_finite() {
        return None;
    }
    let slot = |level: f32| rect.left() + (level / bins as f32).clamp(0.0, 1.0) * rect.width();
    let (first, last) = ordered(range);
    Some(Rect::from_min_max(pos2(slot(first), rect.top()), pos2(slot(last + 1.0), rect.bottom())))
}

/// The two edge bands of `band`, each `width` wide inside it and snapped to whole points (a band
/// narrower than two edges is split between them).
fn selection_edges(band: Rect, width: f32) -> [Rect; 2] {
    let (left, right) = (band.left().round(), band.right().round());
    let width = width.min((right - left) / 2.0);
    [
        Rect::from_min_max(pos2(left, band.top()), pos2(left + width, band.bottom())),
        Rect::from_min_max(pos2(right - width, band.top()), pos2(right, band.bottom())),
    ]
}

/// The selected range: the marker colour over the whole slots of the range and a marker band at
/// each edge.
pub(super) fn selection_shapes(rect: Rect, bins: usize, range: &RangeInclusive<f32>, width: f32, color: Color32) -> Vec<Shape> {
    let Some(band) = selection_band(rect, bins, range) else { return Vec::new() };
    let mut shapes = vec![Shape::rect_filled(band, 0.0, color)];
    shapes.extend(selection_edges(band, width).into_iter().map(|edge| Shape::rect_filled(edge.intersect(rect), 0.0, color)));
    shapes
}

/// The id of the selection's start (0) or end (1) slider.
pub(super) fn selection_node(id: Id, edge: usize) -> Id {
    id.with(("selection", edge))
}

/// The value a screen reader asked of the slider `node` this frame, within `bounds`: its
/// Increment and Decrement steps of one level, or the last SetValue. `None` when it changed nothing.
fn requested(ui: &Ui, node: Id, value: f32, bounds: RangeInclusive<f32>) -> Option<f32> {
    if !ui.is_enabled() {
        return None;
    }
    let (steps, set) = ui.input(|i| {
        let steps = i.num_accesskit_action_requests(node, Action::Increment) as f32 - i.num_accesskit_action_requests(node, Action::Decrement) as f32;
        let set = i.accesskit_action_requests(node, Action::SetValue).filter_map(|r| match r.data {
            Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v as f32),
            _ => None,
        });
        (steps, set.last())
    });
    let (lo, hi) = (*bounds.start(), *bounds.end());
    let wanted = set.unwrap_or(value + steps).round().clamp(lo, hi.max(lo));
    (wanted != value).then_some(wanted)
}

/// The outline of `bins` over `data`: in the tone look on the plot's scale, so a taller outline
/// runs off the top; in the RGB look on its own.
pub(super) fn outline_shapes(rect: Rect, data: &HistogramData, bins: &[f32], channel: ChannelColor, style: &HistogramStyle) -> Vec<Shape> {
    if !usable(rect) {
        return Vec::new();
    }
    let (heights, color) = match &data.plot {
        Plot::Tone { bins: current, .. } => (
            normalized(bins, tone_top(current, data.scale)),
            if channel == ChannelColor::Neutral { style.outline_neutral } else { style.tone(channel) },
        ),
        Plot::Channels { .. } => (tone_heights(bins, data.scale), style.ink(channel)),
    };
    vec![Shape::line(outline(rect, &heights), Stroke::new(style.line, color))]
}

/// `v / scale` clamped to 1 for every bin.
fn normalized(bins: &[f32], scale: f32) -> Vec<f32> {
    bins.iter().map(|&v| (v / scale).min(1.0)).collect()
}

/// The 95th-percentile column of the interior columns (every bin but the two edges), whose
/// 1.5 times is the height of the full plot in [`HistogramScale::Typical`].
fn typical(columns: impl Iterator<Item = f32>) -> f32 {
    let mut columns: Vec<f32> = columns.collect();
    columns.sort_unstable_by(f32::total_cmp);
    columns.get(columns.len() * 95 / 100).copied().unwrap_or(0.0) * 1.5
}

fn interior(bins: &[f32]) -> &[f32] {
    bins.get(1..bins.len().saturating_sub(1)).unwrap_or(&[])
}

/// Heights in `0..=1` of one channel's bins.
pub(super) fn tone_heights(bins: &[f32], scale: HistogramScale) -> Vec<f32> {
    normalized(bins, tone_top(bins, scale))
}

/// The count at the top of the plot for one channel's bins.
fn tone_top(bins: &[f32], scale: HistogramScale) -> f32 {
    let tallest = bins.iter().copied().fold(0.0f32, f32::max);
    let mut top = tallest.max(1.0) * 1.1;
    if scale == HistogramScale::Typical {
        let typical = typical(interior(bins).iter().copied());
        if typical > 0.0 {
            top = top.min(typical);
        }
    }
    top
}

/// The display smoothing of an RGB channel: a five-bin binomial kernel (1 4 6 4 1)/16 on the
/// interior bins; the two edge bins are pinned and kept out of their neighbours' averages.
fn smooth(bins: &[f32]) -> Vec<f32> {
    let n = bins.len();
    if n < 3 {
        return bins.to_vec();
    }
    let sample = |j: usize| bins[j.clamp(1, n - 2)];
    (0..n)
        .map(|i| {
            if i == 0 || i == n - 1 {
                bins[i]
            } else {
                (sample(i.saturating_sub(2)) + 4.0 * sample(i - 1) + 6.0 * bins[i] + 4.0 * sample(i + 1) + sample(i + 2)) / 16.0
            }
        })
        .collect()
}

/// Heights in `0..=1` of the three channels, on one scale. Edge bins are clipped pixels: they
/// don't set the scale unless the interior is empty, and are capped at the top.
pub(super) fn rgb_heights(channels: &[Vec<f32>], scale: HistogramScale) -> Vec<Vec<f32>> {
    let curves: Vec<Vec<f32>> = channels.iter().map(|c| smooth(c)).collect();
    let interior_peak = curves.iter().flat_map(|c| interior(c).iter().copied()).fold(0.0f32, f32::max);
    let peak = if interior_peak > 0.0 { interior_peak } else { curves.iter().flatten().copied().fold(0.0f32, f32::max) };
    let mut top = if peak > 0.0 { peak * 1.1 } else { 1.0 };
    if scale == HistogramScale::Typical {
        let n = curves.first().map_or(0, Vec::len);
        let columns = (1..n.saturating_sub(1)).map(|i| curves.iter().map(|c| c.get(i).copied().unwrap_or(0.0)).fold(0.0f32, f32::max));
        let typical = typical(columns);
        if typical > 0.0 {
            top = top.min(typical);
        }
    }
    curves.iter().map(|c| normalized(c, top)).collect()
}

/// The top edge of a plot: flat from the left to the first bin's centre, straight between
/// centres, flat from the last to the right.
pub(super) fn outline(rect: Rect, heights: &[f32]) -> Vec<Pos2> {
    let n = heights.len();
    let y = |h: f32| rect.bottom() - h * rect.height();
    let mut points = Vec::with_capacity(n + 2);
    let (Some(&first), Some(&last)) = (heights.first(), heights.last()) else { return points };
    points.push(pos2(rect.left(), y(first)));
    points.extend(heights.iter().enumerate().map(|(i, &h)| pos2(rect.left() + (i as f32 + 0.5) / n as f32 * rect.width(), y(h))));
    points.push(pos2(rect.right(), y(last)));
    points
}

/// One mesh of vertical strips under `heights`: no anti-aliasing feather, so no seams between bins.
pub(super) fn tone_mesh(rect: Rect, heights: &[f32], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for pair in outline(rect, heights).windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let i = mesh.vertices.len() as u32;
        for p in [pos2(a.x, rect.bottom()), a, b, pos2(b.x, rect.bottom())] {
            mesh.colored_vertex(p, color);
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    mesh
}

fn quad(mesh: &mut Mesh, rect: Rect, x: [f32; 2], bottom: [f32; 2], top: [f32; 2], color: Color32) {
    let i = mesh.vertices.len() as u32;
    for (x, height) in [(x[0], bottom[0]), (x[0], top[0]), (x[1], top[1]), (x[1], bottom[1])] {
        mesh.colored_vertex(pos2(x, rect.bottom() - height * rect.height()), color);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// One span between two bin centres, split where two channels' edges cross so each piece has a
/// fixed channel order: the band under k channels is drawn once, in their overlap colour, so the
/// result doesn't depend on drawing order and has no cracks.
fn span(mesh: &mut Mesh, rect: Rect, x: [f32; 2], a: &[f32], b: &[f32], fill: &dyn Fn(u32) -> Color32) {
    let mut cuts = vec![0.0f32, 1.0];
    for (i, (ai, bi)) in a.iter().zip(b).enumerate() {
        for (aj, bj) in a.iter().zip(b).skip(i + 1) {
            let denominator = (bi - ai) - (bj - aj);
            if denominator.abs() > f32::EPSILON {
                let crossing = (aj - ai) / denominator;
                if crossing > 0.0 && crossing < 1.0 {
                    cuts.push(crossing);
                }
            }
        }
    }
    cuts.sort_unstable_by(f32::total_cmp);
    let all = if a.len() >= 32 { u32::MAX } else { (1u32 << a.len()) - 1 };
    for cut in cuts.windows(2) {
        let (start, end) = (cut[0], cut[1]);
        if end <= start {
            continue;
        }
        let at = |t: f32| -> Vec<f32> { a.iter().zip(b).map(|(a, b)| a + (b - a) * t).collect() };
        let (left, right) = (at(start), at(end));
        let mut order: Vec<usize> = (0..a.len()).collect();
        order.sort_by(|&i, &j| (left[i] + right[i]).total_cmp(&(left[j] + right[j])));
        let x = [x[0] + (x[1] - x[0]) * start, x[0] + (x[1] - x[0]) * end];
        let mut floor = [0.0; 2];
        let mut mask = all;
        for channel in order {
            let top = [left[channel], right[channel]];
            if top[0] > floor[0] || top[1] > floor[1] {
                quad(mesh, rect, x, floor, top, fill(mask));
            }
            floor = top;
            mask &= !(1 << channel);
        }
    }
}

/// The ribbons' fills.
pub(super) fn rgb_mesh(rect: Rect, heights: &[Vec<f32>], colors: &[Color32], blend: Blend) -> Mesh {
    let mut mesh = Mesh::default();
    let n = heights.first().map_or(0, Vec::len);
    if n == 0 || heights.iter().any(|h| h.len() != n) || colors.len() != heights.len() {
        return mesh;
    }
    let fill = |mask: u32| blend.color(colors, mask);
    let at = |i: usize| heights.iter().map(|h| h[i]).collect::<Vec<f32>>();
    let mut previous_x = rect.left();
    let mut previous = at(0);
    for i in 0..n {
        let x = rect.left() + (i as f32 + 0.5) / n as f32 * rect.width();
        let next = at(i);
        span(&mut mesh, rect, [previous_x, x], &previous, &next, &fill);
        previous_x = x;
        previous = next;
    }
    span(&mut mesh, rect, [previous_x, rect.right()], &previous, &previous, &fill);
    mesh
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Histogram"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/histogram/tone",
            Level::Composites,
            "Histogram",
            "Tone",
            ToneArgs { channel: ChannelArg::Red, stale: false, marker: true, width: 256.0 },
            show_tone
        ),
        typed_story!(
            "composites/histogram/rgb",
            Level::Composites,
            "Histogram",
            "RGB",
            RgbArgs { scale: ScaleArg::Peak, marker: false, width: 256.0 },
            show_rgb
        ),
        typed_story!("composites/histogram/inks", Level::Composites, "Histogram", "Inks", WidthArgs::default(), show_inks),
        typed_story!("composites/histogram/outline", Level::Composites, "Histogram", "Outline", WidthArgs::default(), show_outline),
        typed_story!("composites/histogram/markers", Level::Composites, "Histogram", "Markers", WidthArgs::default(), show_markers),
        typed_story!("composites/histogram/selection", Level::Composites, "Histogram", "Selection", WidthArgs::default(), show_selection),
        typed_story!("composites/histogram/stale-click", Level::Composites, "Histogram", "Stale click", WidthArgs::default(), show_stale_click),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
pub(super) enum ChannelArg {
    Neutral,
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
}

impl ChannelArg {
    fn color(self) -> ChannelColor {
        match self {
            ChannelArg::Neutral => ChannelColor::Neutral,
            ChannelArg::Red => ChannelColor::Red,
            ChannelArg::Green => ChannelColor::Green,
            ChannelArg::Blue => ChannelColor::Blue,
            ChannelArg::Cyan => ChannelColor::Cyan,
            ChannelArg::Magenta => ChannelColor::Magenta,
            ChannelArg::Yellow => ChannelColor::Yellow,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
pub(super) enum ScaleArg {
    Peak,
    Typical,
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct ToneArgs {
    pub(super) channel: ChannelArg,
    pub(super) stale: bool,
    pub(super) marker: bool,
    #[arg(range = 80.0..=400.0)]
    pub(super) width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct RgbArgs {
    pub(super) scale: ScaleArg,
    pub(super) marker: bool,
    #[arg(range = 80.0..=400.0)]
    pub(super) width: f32,
}

/// The args of the stories that only pick the column width.
#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct WidthArgs {
    #[arg(range = 80.0..=400.0)]
    width: f32,
}

impl Default for WidthArgs {
    fn default() -> Self {
        Self { width: 256.0 }
    }
}

/// A bell curve of `count` pixels around `centre` with spread `width`, over 256 bins.
fn bell(centre: f32, width: f32, count: f32) -> Vec<u32> {
    (0..256).map(|i| (count * (-((i as f32 - centre) / width).powi(2) / 2.0).exp()) as u32).collect()
}

pub(super) fn show_tone(ui: &mut Ui, args: &mut ToneArgs) {
    let width = args.width.min(ui.available_width());
    let channel = args.channel.color();
    let bins = bell(90.0, 30.0, 9000.0).iter().zip(bell(190.0, 14.0, 6000.0)).map(|(a, b)| a + b).collect::<Vec<_>>();
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let marker = args.marker.then_some(128.0);
        Histogram::new("story-tone", "Histogram")
            .tone(&bins, channel)
            .value(format!("{} channel", channel.name()))
            .marker(marker)
            .stale(args.stale)
            .show(ui);
    });
}

pub(super) fn show_rgb(ui: &mut Ui, args: &mut RgbArgs) {
    let width = args.width.min(ui.available_width());
    let scale = match args.scale {
        ScaleArg::Typical => HistogramScale::Typical,
        ScaleArg::Peak => HistogramScale::Peak,
    };
    let (mut r, mut g, mut b) = (bell(100.0, 36.0, 9000.0), bell(120.0, 40.0, 8000.0), bell(150.0, 30.0, 7000.0));
    // Clipped pixels at both ends, far above the interior.
    for (bins, edges) in [(&mut r, (60_000, 9000)), (&mut g, (30_000, 2000)), (&mut b, (20_000, 40_000))] {
        bins[0] = edges.0;
        bins[255] = edges.1;
    }
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let marker = args.marker.then_some(100.0);
        Histogram::new("story-rgb", "RGB histogram").rgb([&r, &g, &b]).scale(scale).value("Red, green and blue channels").marker(marker).show(ui);
    });
}

/// `story` in a column `width` wide.
fn sized(ui: &mut Ui, args: &WidthArgs, story: impl FnOnce(&mut Ui)) {
    let width = args.width.min(ui.available_width());
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        story(ui);
    });
}

fn story_bins() -> Vec<u32> {
    bell(90.0, 30.0, 9000.0).iter().zip(bell(190.0, 14.0, 6000.0)).map(|(a, b)| a + b).collect()
}

pub(super) fn show_inks(ui: &mut Ui, args: &mut WidthArgs) {
    let (c, m, y, k) = (bell(70.0, 30.0, 9000.0), bell(110.0, 36.0, 8000.0), bell(150.0, 30.0, 7000.0), bell(40.0, 20.0, 6000.0));
    sized(ui, args, |ui| {
        let inks: [(&[u32], ChannelColor); 4] =
            [(&c, ChannelColor::Cyan), (&m, ChannelColor::Magenta), (&y, ChannelColor::Yellow), (&k, ChannelColor::Neutral)];
        Histogram::new("story-inks", "Ink histogram").channels(&inks).value("Cyan, magenta, yellow and black inks").show(ui);
    });
}

pub(super) fn show_outline(ui: &mut Ui, args: &mut WidthArgs) {
    let original = bell(120.0, 50.0, 12_000.0);
    sized(ui, args, |ui| {
        Histogram::new("story-outline", "Histogram")
            .tone(&story_bins(), ChannelColor::Red)
            .outline(&original, ChannelColor::Neutral)
            .value("Red channel with the original outlined")
            .show(ui);
    });
}

pub(super) fn show_markers(ui: &mut Ui, args: &mut WidthArgs) {
    sized(ui, args, |ui| {
        let markers = [(12.0, "0".to_owned()), (128.0, "1.00".to_owned()), (243.0, "255".to_owned())];
        Histogram::new("story-markers", "Histogram").tone(&story_bins(), ChannelColor::Neutral).markers(&markers).value("Levels points").show(ui);
    });
}

pub(super) fn show_selection(ui: &mut Ui, args: &mut WidthArgs) {
    sized(ui, args, |ui| {
        let id = ui.scope_id().with("story-selection");
        let current = ui.data(|d| d.get_temp::<Option<(f32, f32)>>(id)).flatten().map(|(a, b)| a..=b);
        let shown = Histogram::new("story-selection", "Histogram")
            .tone(&story_bins(), ChannelColor::Neutral)
            .selection(current)
            .value("Selected levels")
            .show(ui);
        if let Some(new) = shown.selection {
            ui.data_mut(|d| d.insert_temp(id, new.map(|r| (*r.start(), *r.end()))));
        }
        ui.label(shown.hovered_level.map_or_else(|| "Level: none".to_owned(), |level| format!("Level: {level}")));
    });
}

pub(super) fn show_stale_click(ui: &mut Ui, args: &mut WidthArgs) {
    sized(ui, args, |ui| {
        let id = ui.scope_id().with("story-recalculated");
        let count = ui.data(|d| d.get_temp::<u32>(id)).unwrap_or(0);
        let shown = Histogram::new("story-stale-click", "Histogram")
            .tone(&story_bins(), ChannelColor::Neutral)
            .stale(true)
            .on_stale_click("Recalculate")
            .value(format!("Recalculated {count} times"))
            .show(ui);
        if shown.clicked {
            ui.data_mut(|d| d.insert_temp(id, count + 1));
        }
    });
}
