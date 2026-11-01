//! [`ScopePanel`]: views, the Channel and Source rows, the statistics grid, markers and the
//! cached-data mark, and the data the host passes ([`ScopeFrame`]).

use std::ops::RangeInclusive;

use craft_theme::{ControlSize, TypeRole};
use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::composites::ChannelColor;
use craft_ui::controls::{IconButton, Select};
use craft_ui::direction;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::focus::key_owner::{self, KeyOwner, OwnerGate};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::menu::{ContextMenu, MenuEntry};
use craft_ui::primitives::{Content, Corner, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, Id, IdSalt, Key, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use super::clipping::{ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingWarnings, warning_button};
use super::labels::ScopePanelLabels;
use super::menu::ScopeMenuAction;
use super::plot::{Shown, all_channels, main_plot, resolve_channel};
use super::style::ScopePanelStyle;
use super::vectorscope::{VectorscopeFrame, VectorscopePlot};
use super::zones::{ScopeZoneEdit, ToneZone, zones};
use crate::scopes::{HistogramStatistics, histogram_statistics};

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
    pub fn clipping_mode(&self, diagnostic: Option<ClippingDiagnostic>) -> ClippingMode {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:608-614 (`mode`).
        match diagnostic {
            Some(ClippingDiagnostic::Shadows) => ClippingMode::ShadowChannels,
            Some(ClippingDiagnostic::Highlights) => ClippingMode::HighlightChannels,
            None if self.shadow_warning || self.highlight_warning => ClippingMode::Warnings(self.clipping_warnings()),
            None => ClippingMode::None,
        }
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
    pub fn show(mut self, ui: &mut Ui) -> ScopePanelOutput {
        // pc: crates/ui-egui/src/panels/histogram_panel.rs:735 `show`, :636 `graph`, :706 `statistics`;
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:498 `header`, :380 `graph`, :687 `shortcuts`.
        let tokens = Tokens::of(ui);
        let style = ScopePanel::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| ScopePanelLabels::of(ui));
        let base = ui.make_persistent_id(self.id_salt);
        let mut out = ScopePanelOutput {
            response: ui.allocate_response(Vec2::ZERO, Sense::hover()),
            settings_changed: false,
            refresh: false,
            source_changed: false,
            channel_changed: false,
            zone: None,
            hovered_level: None,
            range: None,
            clipping_toggled: false,
        };
        let Some(frame) = self.frame else {
            out.range = self.state.range.clone();
            return out;
        };
        let set = &frame.histogram;
        let Some(composite) = set.channels.first() else {
            out.range = self.state.range.clone();
            return out;
        };
        let numbers = LocaleNumbers::of(ui);
        let menu_entries =
            (frame.vectorscope.is_some() || frame.clipping.is_some()).then(|| ScopePanel::scope_menu(ui.ctx(), base, self.state, self.has_selection));
        let scope = ui.scope(|ui| {
            ui.add_enabled_ui(self.enabled, |ui| {
                self.rows(
                    ui,
                    &Context {
                        base,
                        tokens: &tokens,
                        style: &style,
                        labels: &labels,
                        numbers: &numbers,
                        composite,
                        menu_entries: menu_entries.as_deref(),
                    },
                    &mut out,
                );
            });
        });
        out.response = scope.response;
        if self.enabled && frame.clipping.is_some() && panel_keys(ui, out.response.rect, &mut self.state.settings) {
            out.clipping_toggled = true;
            out.settings_changed = true;
        }
        out.range = self.state.range.clone();
        out
    }

    fn rows(&mut self, ui: &mut Ui, cx: &Context<'_>, out: &mut ScopePanelOutput) {
        let Some(frame) = self.frame else { return };
        let set = &frame.histogram;
        let state = &mut *self.state;
        let (labels, style) = (cx.labels, cx.style);
        let view = state.settings.view;
        let shown = resolve_channel(set, &state.settings.channel).unwrap_or(Shown::Channel(*cx.composite));
        let shown_key = match shown {
            Shown::Channel(channel) => channel.key,
            Shown::Colors(_) => COLORS_CHANNEL,
        };
        let expanded = view != ScopeView::Compact;
        let colored = !expanded || state.settings.show_channels_in_color;
        let mut menu_widgets: Vec<Response> = Vec::new();

        if expanded {
            let mut picked = shown_key.to_owned();
            direction::horizontal(ui, |ui| {
                ui.add(Text::new(labels.channel.as_str()).role(TypeRole::Label).content(Content::Muted));
                let mut select = Select::new((cx.base, "channel"), labels.channel.trim_end_matches(':'), &mut picked);
                for channel in set.channels {
                    select = select.option(channel.key.to_owned(), channel.label);
                }
                if set.colors.is_some() {
                    select = select.option(COLORS_CHANNEL.to_owned(), labels.colors.as_str());
                }
                let _ = select.show(ui);
                ui.with_layout(direction::trailing(ui), |ui| {
                    if ui.add(IconButton::new(Icon::RotateCw, labels.uncached_refresh.as_str()).size(ControlSize::Sm)).clicked() {
                        out.refresh = true;
                    }
                });
            });
            if picked != shown_key {
                state.settings.channel = picked;
                out.channel_changed = true;
                out.settings_changed = true;
            }
        }
        let shown = resolve_channel(set, &state.settings.channel).unwrap_or(shown);

        let source =
            self.sources.iter().find(|s| s.key == state.settings.source).or(self.sources.first()).map_or(labels.entire_image.as_str(), |s| s.label);
        let zoned = !self.zones.is_empty();
        let plot =
            main_plot(ui, cx.base.with("plot"), labels, source, set, shown, colored, style.plot_height, self.markers, state.range.clone(), zoned);
        let rect = plot.response.rect;
        if zoned {
            out.zone = zones(ui, &plot.response, rect, self.zones, self.zone_values, labels, style);
        } else if let Some(selection) = plot.selection.clone() {
            state.range = selection;
        }
        // With zones a double-click resets the zone under it instead.
        if plot.clicked || !zoned && plot.response.double_clicked() {
            out.refresh = true;
        }
        out.hovered_level = plot.hovered_level;
        menu_widgets.push(plot.response.clone());

        if let Some(clipping) = frame.clipping {
            let side = style.warning_size;
            let settings = &mut state.settings;
            let shadows = Rect::from_min_size(rect.min, vec2(side, side));
            let highlights = Rect::from_min_size(pos2(rect.right() - side, rect.top()), vec2(side, side));
            let toggled_shadows =
                warning_button(ui, shadows, style.warning_inset, clipping.shadows, &mut settings.shadow_warning, &labels.shadow_warning);
            let toggled_highlights =
                warning_button(ui, highlights, style.warning_inset, clipping.highlights, &mut settings.highlight_warning, &labels.highlight_warning);
            if toggled_shadows || toggled_highlights {
                out.clipping_toggled = true;
                out.settings_changed = true;
            }
        }
        if let Some(readout) = frame.readout {
            readout_badge(ui, cx, rect, readout);
        }

        if state.settings.vectorscope
            && let Some(vectorscope) = frame.vectorscope.as_ref()
        {
            let response = VectorscopePlot::new((cx.base, "vectorscope"), vectorscope)
                .red_right(state.settings.red_right)
                .skin_tone(state.settings.skin_tone)
                .labels(labels.clone())
                .show(ui);
            menu_widgets.push(response);
        }

        if expanded && self.sources.len() >= 2 {
            let current = self.sources.iter().find(|s| s.key == state.settings.source).unwrap_or(&self.sources[0]).key;
            let mut picked = current.to_owned();
            direction::horizontal(ui, |ui| {
                ui.add(Text::new(labels.source.as_str()).role(TypeRole::Label).content(Content::Muted));
                let mut select = Select::new((cx.base, "source"), labels.source.trim_end_matches(':'), &mut picked);
                for source in self.sources {
                    select = select.option(source.key.to_owned(), source.label);
                }
                let _ = select.show(ui);
            });
            if picked != current {
                state.settings.source = picked;
                out.source_changed = true;
                out.settings_changed = true;
            }
        }

        if expanded && state.settings.show_statistics {
            let levels = match &state.range {
                Some(range) => Some([level_of(*range.start()), level_of(*range.end())]),
                None => out.hovered_level.map(|level| [level_of(level); 2]),
            };
            if let Some(stats) = histogram_statistics(shown_bins(set, shown), levels) {
                statistics(ui, cx, &stats, set.cache_level);
            }
        }

        if view == ScopeView::AllChannels {
            all_channels(ui, cx.base.with("channels"), labels, source, set, colored, style.all_channels_height);
        }

        if let Some(entries) = cx.menu_entries {
            for widget in &menu_widgets {
                let chosen = ContextMenu::new(entries).show(ui, widget).chosen;
                match chosen.and_then(|command| ScopePanel::run_command(ui.ctx(), cx.base, state, &command)) {
                    Some(ScopeMenuAction::SettingsChanged) => out.settings_changed = true,
                    Some(ScopeMenuAction::Refresh) => out.refresh = true,
                    None => {}
                }
            }
        }
    }
}

/// What the rows of one pass share.
struct Context<'a> {
    base: Id,
    tokens: &'a UiTokens,
    style: &'a ScopePanelStyle,
    labels: &'a ScopePanelLabels,
    numbers: &'a LocaleNumbers,
    composite: &'a ScopeChannel<'a>,
    menu_entries: Option<&'a [MenuEntry<PanelCommand>]>,
}

/// The counts of what the main plot shows: Colors counts the composite.
fn shown_bins<'s>(set: &HistogramSet<'s>, shown: Shown<'s>) -> &'s [u32] {
    match shown {
        Shown::Channel(channel) => channel.bins,
        Shown::Colors(_) => set.channels.first().map_or(&[], |channel| channel.bins),
    }
}

/// The level a plot position names, as the statistics take it.
fn level_of(level: f32) -> u8 {
    level.round().clamp(0.0, 255.0) as u8
}

/// The statistics grid: two label and value pairs per row (Mean, Std Dev, Median, Pixels | Level,
/// Count, Percentile, Cache Level), as a `Table` of `Row`s and `Cell`s.
fn statistics(ui: &mut Ui, cx: &Context<'_>, stats: &HistogramStatistics, cache_level: u32) {
    let labels = cx.labels;
    // Numbers read left to right in every language (`ltr`).
    let number = |value: f64, decimals: usize| craft_i18n::ltr(&cx.numbers.format(value, decimals..=decimals));
    let level = match stats.level {
        Some([a, b]) if a == b => number(f64::from(a), 0),
        Some([a, b]) => craft_i18n::ltr(&format!("{}..{}", number(f64::from(a), 0), number(f64::from(b), 0))),
        None => String::new(),
    };
    let rows = [
        (&labels.mean, number(stats.mean, 2), &labels.level, level),
        (
            &labels.standard_deviation,
            number(stats.standard_deviation, 2),
            &labels.count,
            stats.count.map(|c| number(c as f64, 0)).unwrap_or_default(),
        ),
        (&labels.median, number(f64::from(stats.median), 0), &labels.percentile, stats.percentile.map(|p| number(p, 2)).unwrap_or_default()),
        (&labels.pixels, number(stats.pixels as f64, 0), &labels.cache_level, number(f64::from(cache_level), 0)),
    ];
    let label_font = role_format(cx.tokens, TypeRole::Label, Color32::PLACEHOLDER).font_id;
    let value_font = role_format(cx.tokens, TypeRole::Mono, Color32::PLACEHOLDER).font_id;
    let (label_width, line) = ui.fonts_mut(|f| {
        let widest = rows
            .iter()
            .flat_map(|(a, _, b, _)| [a, b])
            .map(|label| f.layout_no_wrap(label.to_string(), label_font.clone(), Color32::PLACEHOLDER).size().x)
            .fold(0.0, f32::max);
        (widest.ceil(), f.row_height(&label_font).max(f.row_height(&value_font)))
    });
    let (gap, inline) = (cx.style.statistics_gap, cx.tokens.space.gap.inline);
    let width = ui.available_width();
    let pair = ((width - gap) / 2.0).max(0.0);
    let value_width = (pair - label_width - inline).max(0.0);
    let rtl = direction::is_rtl(ui);
    let builder = UiBuilder::new().accessibility_role(Role::Table).accessibility_label(labels.statistics.clone());
    ui.scope_builder(builder, |ui| {
        for (row_at, (first, first_value, second, second_value)) in rows.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(vec2(width, line), Sense::hover());
            let slot = |from: f32, wide: f32| {
                let (min, max) =
                    if rtl { (rect.right() - from - wide, rect.right() - from) } else { (rect.left() + from, rect.left() + from + wide) };
                Rect::from_min_max(pos2(min, rect.top()), pos2(max, rect.bottom()))
            };
            ui.scope_builder(UiBuilder::new().id_salt(("row", row_at)).max_rect(rect).accessibility_role(Role::Row), |ui| {
                for (column, (label, value)) in [(first, first_value), (second, second_value)].into_iter().enumerate() {
                    let from = column as f32 * (pair + gap);
                    ui.scope_builder(
                        UiBuilder::new().max_rect(slot(from, label_width)).layout(direction::trailing(ui)).accessibility_role(Role::Cell),
                        |ui| {
                            ui.add(Text::new(label.as_str()).role(TypeRole::Label).content(Content::Muted));
                        },
                    );
                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(slot(from + label_width + inline, value_width))
                            .layout(direction::row(ui))
                            .accessibility_role(Role::Cell),
                        |ui| {
                            ui.add(Text::new(value.as_str()).role(TypeRole::Mono));
                        },
                    );
                }
            });
        }
    });
}

/// The hover readout badge in the plot's top centre: `R r  G g  B b` in 0..=255, or the Lab form
/// with one decimal, between the two warning buttons.
fn readout_badge(ui: &Ui, cx: &Context<'_>, plot: Rect, readout: ScopeReadout) {
    let labels = cx.labels;
    let text = match readout {
        ScopeReadout::Rgb([r, g, b]) => {
            let [r, g, b] = [r, g, b].map(|v| craft_i18n::ltr(&cx.numbers.format(f64::from(v) * 255.0, 0..=0)));
            craft_i18n::fmt(&labels.readout_rgb, &[("r", &r), ("g", &g), ("b", &b)])
        }
        ScopeReadout::Lab([l, a, b]) => {
            let [l, a, b] = [l, a, b].map(|v| craft_i18n::ltr(&cx.numbers.format(f64::from(v), 1..=1)));
            craft_i18n::fmt(&labels.readout_lab, &[("l", &l), ("a", &a), ("b", &b)])
        }
    };
    let tokens = cx.tokens;
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(text, tokens.font(TypeRole::Mono), tokens.color.text.normal.to_egui());
    let pad = tokens.space.gap.inline;
    let room = (plot.width() - 2.0 * cx.style.warning_size - 2.0 * pad).max(1.0);
    let width = (galley.size().x + 2.0 * pad).min(room);
    let badge = Rect::from_center_size(
        pos2(plot.center().x, plot.top() + cx.style.hover_badge_height / 2.0 + pad / 2.0),
        vec2(width, cx.style.hover_badge_height),
    );
    let radius = Corner::Control.radius(tokens);
    painter.rect_filled(badge, radius, tokens.color.surface.raised.to_egui());
    painter.rect_stroke(
        badge,
        radius,
        Stroke::new(craft_theme::Stroke::W1.width(ui.pixels_per_point()), tokens.component.field.border.rest.to_egui()),
        StrokeKind::Inside,
    );
    let text_at = badge.center() - galley.size() / 2.0;
    painter.with_clip_rect(badge.shrink(pad / 2.0)).galley(text_at, galley, tokens.color.text.normal.to_egui());
}

/// U and O toggle the warnings while the panel has the keyboard focus and no text field, menu or
/// dialog has the keys (the panel is the key owner through [`key_owner`]). The focused control
/// claims both keys so the app's shortcuts leave them alone.
fn panel_keys(ui: &Ui, panel: Rect, settings: &mut ScopeSettings) -> bool {
    let ctx = ui.ctx();
    let Some(focused) = ctx.memory(|m| m.focused()) else { return false };
    if key_owner::owner(ctx, &OwnerGate::default()) != KeyOwner::Widget || !ctx.read_response(focused).is_some_and(|r| panel.contains_rect(r.rect)) {
        return false;
    }
    KeyClaim::claim(ctx, focused, &[ClaimedKey::exact(Modifiers::NONE, Key::U), ClaimedKey::exact(Modifiers::NONE, Key::O)]);
    let (shadows, highlights) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::U), i.consume_key(Modifiers::NONE, Key::O)));
    settings.shadow_warning ^= shadows;
    settings.highlight_warning ^= highlights;
    shadows || highlights
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
