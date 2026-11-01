//! [`ScopePanel`]: views, the Channel and Source rows, the statistics grid, markers and the
//! cached-data mark, and the data the host passes ([`ScopeFrame`]).

use std::ops::RangeInclusive;

use craft_ui::composites::ChannelColor;
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::clipping::{ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingWarnings};
use super::labels::ScopePanelLabels;
use super::vectorscope::VectorscopeFrame;
use super::zones::{ScopeZoneEdit, ToneZone};

/// The Channel key of [`HistogramSet::colors`]'s overlay entry, kept in
/// [`ScopeSettings::channel`].
pub const COLORS_CHANNEL: &str = "colors";

/// One channel's counted histogram, as the host counted it (colour management and the count are
/// the host's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeChannel<'a> {
    /// A stable key ("rgb", "red", "cyan", "luminosity" …), kept in [`ScopeSettings::channel`].
    /// The first channel of a [`HistogramSet`] is the composite.
    pub key: &'a str,
    /// The name in the Channel list and the value text, translated by the host ("Red",
    /// "Lightness").
    pub label: &'a str,
    /// The colour the channel draws in when coloured (`Neutral` for the composite and
    /// Luminosity).
    pub ink: ChannelColor,
    /// One count per level, usually 256.
    pub bins: &'a [u32],
    /// One of the colour mode's colour channels: drawn in All Channels View and overlaid by the
    /// Colors entry (no composite, Luminosity, alpha, spot or mask channels).
    pub color_channel: bool,
}

/// How the Colors entry overlays the colour channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChannelOverlay {
    /// Light (RGB): fills add where they overlap, on the dark plot
    /// ([`HistogramData::rgb`](craft_ui::composites::HistogramData::rgb)'s look).
    #[default]
    Light,
    /// Inks (CMYK): fills multiply where they overlap
    /// ([`HistogramData::channels`](craft_ui::composites::HistogramData::channels)).
    Ink,
}

/// The histograms the host counted for the colour mode, and how fresh they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistogramSet<'a> {
    pub(super) channels: &'a [ScopeChannel<'a>],
    pub(super) original: Option<&'a [ScopeChannel<'a>]>,
    pub(super) colors: Option<ChannelOverlay>,
    pub(super) stale: bool,
    pub(super) cache_level: u32,
}

impl<'a> HistogramSet<'a> {
    /// The mode's channels in Channel-list order, the composite first (PhotoCraft's
    /// `channel_options`: RGB, Red, Green, Blue, Luminosity; CMYK, Cyan, Magenta, Yellow, Black,
    /// Luminosity; Lightness, a, b, Luminosity; Gray).
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:123 `channel_options`.
    pub fn new(channels: &'a [ScopeChannel<'a>]) -> Self {
        Self { channels, original: None, colors: None, stale: false, cache_level: 1 }
    }

    /// The histograms before the adjustment that previews now: each channel's original is outlined
    /// over the adjusted one (matched by key; Colors outlines the composite), in `text.normal` at
    /// 0.8, 1 pt.
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:652-654, :695 `outline`.
    pub fn original(mut self, original: Option<&'a [ScopeChannel<'a>]>) -> Self {
        self.original = original;
        self
    }

    /// Offers the Colors entry at the end of the Channel list, overlaying the colour channels as
    /// `overlay` says (RGB and CMYK modes; Lab and Gray have none).
    pub fn colors(mut self, overlay: ChannelOverlay) -> Self {
        self.colors = Some(overlay);
        self
    }

    /// The counts are from a cache or out of date: the plot shows the library histogram's stale
    /// mark (lead call L15), whose click asks for a refresh.
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// The cache level the counts were made at (1 = every pixel): shown as Cache Level; above 1
    /// the stale mark shows too.
    pub fn cache_level(mut self, level: u32) -> Self {
        self.cache_level = level.max(1);
        self
    }
}

/// A sample under the pointer, for the hover readout badge, in the form
/// [`ScopeSettings::lab_readouts`] asks for (the host does the colour management).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScopeReadout {
    /// R, G, B in 0..=1, shown in 0..=255 without decimals.
    Rgb([f32; 3]),
    /// L, a, b, shown with one decimal.
    Lab([f32; 3]),
}

/// Everything the host counted for one pass: the histograms and, for Camera Raw's scope, the
/// vectorscope, the clipping state and the hover sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeFrame<'a> {
    pub(super) histogram: HistogramSet<'a>,
    pub(super) vectorscope: Option<VectorscopeFrame<'a>>,
    pub(super) clipping: Option<ClippingFrame>,
    pub(super) readout: Option<ScopeReadout>,
}

impl<'a> ScopeFrame<'a> {
    /// A frame of histograms only (Photoshop's Histogram panel).
    pub fn histogram(histogram: HistogramSet<'a>) -> Self {
        Self { histogram, vectorscope: None, clipping: None, readout: None }
    }

    /// The vectorscope's counts; shown under the plot while [`ScopeSettings::vectorscope`] is on.
    pub fn vectorscope(mut self, vectorscope: VectorscopeFrame<'a>) -> Self {
        self.vectorscope = Some(vectorscope);
        self
    }

    /// The clipped channels: shows the two clipping warning buttons in the plot's top corners.
    pub fn clipping(mut self, clipping: ClippingFrame) -> Self {
        self.clipping = Some(clipping);
        self
    }

    /// The sample under the pointer: the readout badge over the plot while it is passed.
    pub fn readout(mut self, readout: Option<ScopeReadout>) -> Self {
        self.readout = readout;
        self
    }
}

/// A Source-menu entry (Entire Image, Selected Layer, Adjustment Composite): what the host counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeSource<'a> {
    /// A stable key, kept in [`ScopeSettings::source`].
    pub key: &'a str,
    /// The name, translated by the host.
    pub label: &'a str,
}

/// The panel's view (its menu's Compact, Expanded and All Channels View).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScopeView {
    /// The plot alone (Photoshop's default).
    #[default]
    Compact,
    /// The Channel row, the plot, the Source row and the statistics.
    Expanded,
    /// Expanded plus one plot per colour channel.
    AllChannels,
}

/// The scope panel's persisted section: the app keeps it with its preferences (PhotoCraft's
/// `histogramPanel` and `filter.cameraRaw.scope`). Unknown fields are ignored, missing ones take
/// their defaults.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScopeSettings {
    /// The view.
    pub view: ScopeView,
    /// The Channel key ([`ScopeChannel::key`] or [`COLORS_CHANNEL`]); empty or a key the mode
    /// lacks shows the composite. Kept across views.
    pub channel: String,
    /// Show Channels in Color: single channels in Expanded and All Channels View in their colour
    /// (Compact always colours them; Colors is always in colour).
    pub show_channels_in_color: bool,
    /// Show Statistics (on by default).
    pub show_statistics: bool,
    /// The Source key ([`ScopeSource::key`]); empty is the first source.
    pub source: String,
    /// Show Lab Color Readouts: the host passes [`ScopeReadout::Lab`].
    pub lab_readouts: bool,
    /// Show Vectorscope.
    pub vectorscope: bool,
    /// Show Selected Region: the host counts the vectorscope from the selection.
    pub selected_region: bool,
    /// Show Red at 3 o'clock (else red sits at −100°).
    pub red_right: bool,
    /// Show Skin Tone Indicator (on by default).
    pub skin_tone: bool,
    /// The shadow clipping warning (U).
    pub shadow_warning: bool,
    /// The highlight clipping warning (O).
    pub highlight_warning: bool,
}

impl Default for ScopeSettings {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:89-99 (panel defaults); state/panel_state.rs:98 `CameraRawScopeState`.
    fn default() -> Self {
        Self {
            view: ScopeView::Compact,
            channel: String::new(),
            show_channels_in_color: false,
            show_statistics: true,
            source: String::new(),
            lab_readouts: false,
            vectorscope: false,
            selected_region: false,
            red_right: false,
            skin_tone: true,
            shadow_warning: false,
            highlight_warning: false,
        }
    }
}

impl ScopeSettings {
    /// What the clipping overlay draws: `diagnostic` (an Alt-drag on a tone control, which the
    /// host recognises) wins; else the warnings that are on; else [`ClippingMode::None`].
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn clipping_mode(&self, diagnostic: Option<ClippingDiagnostic>) -> ClippingMode {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:608-614 (`mode`).
        todo!()
    }

    /// The warnings that are on.
    pub fn clipping_warnings(&self) -> ClippingWarnings {
        ClippingWarnings { shadows: self.shadow_warning, highlights: self.highlight_warning }
    }
}

/// What the panel keeps for the app between passes: the persisted settings and the selected
/// level range (not persisted).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScopePanelState {
    /// The persisted section.
    pub settings: ScopeSettings,
    /// The level range dragged on the plot, in levels; `None` when nothing is selected.
    pub range: Option<RangeInclusive<f32>>,
}

/// Photoshop's Histogram panel and Camera Raw's scope in one widget (lead call L10).
///
/// **What the user sees.**
/// - **Compact View**: the plot alone, `histogram.height` high; a single channel in its colour.
/// - **Expanded View**: the Channel row (a `Select` of the set's channels, then Colors when
///   offered; the Uncached Refresh icon button at its end), the plot, the Source row (only with
///   two or more [`sources`](Self::sources)), and the statistics grid when Show Statistics is on.
/// - **All Channels View**: Expanded plus one plot per colour channel (`scopes.all_channels_height`
///   high, labelled with the channel's name above it).
/// - The plot is the library [`Histogram`](craft_ui::composites::Histogram) on `field.fill`:
///   a single channel in its colour (in Expanded and All Channels only with Show Channels in
///   Color, else `text.muted`), Colors as overlaid ribbons ([`ChannelOverlay`]), the original
///   outlined over it, the host's [`markers`](Self::markers) (sampler ticks), the stale mark.
///   A channel the mode lacks shows the composite.
/// - The statistics grid (lead call L13): two label and value pairs per row: Mean, Std Dev,
///   Median, Pixels for the whole histogram (Colors counts the composite), Level, Count,
///   Percentile and Cache Level for the hovered level or the selected range, values at
///   `TypeRole::Mono`, labels `text.muted`, `scopes.statistics_gap` between the pairs
///   ([`histogram_statistics`](crate::scopes::histogram_statistics)). Level shows `a` or `a..b`;
///   Mean and Std Dev two decimals, Percentile two decimals (PhotoCraft's).
/// - With [`ScopeFrame::clipping`]: the two warning buttons in the plot's top corners, shadows at
///   the start side and highlights at the end side. With [`ScopeFrame::vectorscope`] and Show
///   Vectorscope: the [`VectorscopePlot`](super::VectorscopePlot) under the plot. With a
///   [`readout`](ScopeFrame::readout): the badge in the plot's top centre.
/// - With [`zones`](Self::zones): the tone zones over the plot ([`ToneZone`]).
///
/// **Pointer.** Hover reports the level ([`ScopePanelOutput::hovered_level`]) and fills Level,
/// Count and Percentile; a drag selects a level range (the library histogram's selection: arrows
/// move it, Shift moves the start edge, Esc or a click clears it), except with tone zones, where
/// the drag edits the zone instead; a double-click, a click on the stale mark or the refresh
/// button reports [`refresh`](ScopePanelOutput::refresh). Right-click, the Menu key or Shift+F10
/// on a plot opens the scope menu ([`scope_menu`](Self::scope_menu)) when the frame has a
/// vectorscope or clipping (Camera Raw's scope).
///
/// **Keyboard and focus.** The Channel and Source lists, the refresh button, the plot (for its
/// selection and the focused tone zone) and the warning buttons are Tab stops. While the panel
/// has the focus and no text field has, U toggles the shadow warning and O the highlight warning
/// (the panel is the key owner through [`focus::key_owner`](craft_ui::focus::key_owner); S is the
/// host's, lead call L28).
///
/// **AccessKit.** The plot is the histogram's `Image` named [`ScopePanelLabels::histogram`] with
/// the value text [`ScopePanelLabels::histogram_value`] ("Red channel, Entire Image"); a selection
/// adds two `Slider` nodes. The warnings are `CheckBox`es; the zones a `Group` of `Slider`s; the
/// statistics a `Table` of `Row`s and `Cell`s.
///
/// **Direction.** Plots, the vectorscope and the warning buttons never mirror
/// (`Unmirrored::Graph`); the Channel and Source rows follow the interface direction.
///
/// **What the app gets back.** [`ScopePanelOutput`]; settings are edited in the state it was
/// given and reported with `settings_changed`. The app recounts, persists and maps zone edits.
#[must_use = "show the panel with `show`"]
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct ScopePanel<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) state: &'a mut ScopePanelState,
    pub(super) frame: Option<&'a ScopeFrame<'a>>,
    pub(super) sources: &'a [ScopeSource<'a>],
    pub(super) markers: &'a [(f32, String)],
    pub(super) zones: &'a [ToneZone],
    pub(super) zone_values: &'a [String],
    pub(super) has_selection: bool,
    pub(super) labels: Option<ScopePanelLabels>,
    pub(super) enabled: bool,
}

impl<'a> ScopePanel<'a> {
    /// A scope panel over `state`; `id_salt` keeps its focus, menus and drags.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut ScopePanelState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            frame: None,
            sources: &[],
            markers: &[],
            zones: &[],
            zone_values: &[],
            has_selection: false,
            labels: None,
            enabled: true,
        }
    }

    /// This pass's counted data. Without a frame the panel draws nothing (the host shows
    /// "Calculating…" itself).
    pub fn frame(mut self, frame: &'a ScopeFrame<'a>) -> Self {
        self.frame = Some(frame);
        self
    }

    /// The Source list's entries; the row shows only with two or more.
    pub fn sources(mut self, sources: &'a [ScopeSource<'a>]) -> Self {
        self.sources = sources;
        self
    }

    /// Ticks at the foot of the plot, `(level, label)`: the Color Samplers' levels in the shown
    /// channel, computed by the host ([`Histogram::markers`](craft_ui::composites::Histogram::markers)).
    pub fn markers(mut self, markers: &'a [(f32, String)]) -> Self {
        self.markers = markers;
        self
    }

    /// The tone zones over the plot ([`ToneZone::CAMERA_RAW`] or [`ToneZone::LIGHTROOM`]) and each
    /// zone's value text, in the same order (the tooltip and the AccessKit value).
    pub fn zones(mut self, zones: &'a [ToneZone], values: &'a [String]) -> Self {
        self.zones = zones;
        self.zone_values = values;
        self
    }

    /// The document has a selection: the scope menu offers Show Selected Region.
    pub fn has_selection(mut self, has_selection: bool) -> Self {
        self.has_selection = has_selection;
        self
    }

    /// The words; the installed words' [`ScopePanelLabels`] without a call.
    pub fn labels(mut self, labels: ScopePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Whether the panel takes input (default true).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the view the settings ask for and reads this pass's input, in order: the Channel
    /// row, the plot (with zones, warnings, badge), the vectorscope, the Source row, the
    /// statistics, the All Channels plots, then the U and O keys.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn show(self, ui: &mut Ui) -> ScopePanelOutput {
        // pc: crates/ui-egui/src/panels/histogram_panel.rs:735 `show`, :636 `graph`, :706 `statistics`;
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:498 `header`, :380 `graph`, :687 `shortcuts`.
        todo!()
    }
}

/// What [`ScopePanel::show`] returns. The events of one pass, handled in that pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct ScopePanelOutput {
    /// The panel's area.
    pub response: Response,
    /// A setting changed (view, channel, source, a menu check, a warning): persist
    /// [`ScopeSettings`].
    pub settings_changed: bool,
    /// Uncached Refresh: recount at cache level 1 (button, stale mark, double-click, menu row).
    pub refresh: bool,
    /// The Source changed: the host recounts from [`ScopeSettings::source`].
    pub source_changed: bool,
    /// The Channel changed.
    pub channel_changed: bool,
    /// A tone-zone edit; the host maps it to its control.
    pub zone: Option<ScopeZoneEdit>,
    /// The whole level under the pointer on the main plot.
    pub hovered_level: Option<f32>,
    /// The selected level range after this pass (also in [`ScopePanelState::range`]).
    pub range: Option<RangeInclusive<f32>>,
    /// A clipping warning was toggled (click, U, O): the host rebuilds its overlay.
    pub clipping_toggled: bool,
}
