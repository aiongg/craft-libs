//! [`ScopePanel`]: views, the Channel and Source rows, the statistics grid, markers and the cached-data mark, and the
//! data the host passes ([`ScopeFrame`]).

use super::clipping::{ClipWarning, ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingWarnings};
use super::menu::{ScopeCommand, ScopeMenuAction};
use super::plot::{Shown, plot_data, resolve_channel};
use super::vectorscope::{VectorscopeFrame, VectorscopePlot};
use super::words::ScopeWords;
use super::zones::{ScopeZoneEdit, ToneZone, ToneZones};
use crate::scopes::{HistogramStatistics, histogram_statistics};
use crate::ui::histogram::{ChannelColor, Histogram, HistogramData, LevelRange, PlotHeight, StaleMark};
use craft_behavior::{ColumnWidth, EditEvent, IconId, Proposal};
use craft_theme::{ControlSize, Mode};
use craft_ui::buttons::IconButton;
use craft_ui::contract::{
    Bind, BodyCx, Entry, FieldLabelRole, Gap, NoValue, Outcome, Pattern, Props as _, Show as _, ShowContainer as _, SideLabel, ThemeScope, clicked,
};
use craft_ui::contract::{CommandSet as _, Commands as _};
use craft_ui::display::{Badge, Shade, Text};
use craft_ui::form::Field;
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::pickers::Select;
use craft_ui::runtime::commands::FirePhase;
use craft_ui::runtime::env::{Env, Label};
use craft_ui::style::TypeRole;
use egui::{Rect, Response, Ui, pos2};

/// The Channel key of [`HistogramSet::colors`]'s overlay entry, kept in
/// [`ScopeSettings::channel`].
pub const COLORS_CHANNEL: &str = "colors";

/// One channel's counted histogram, as the host counted it (colour management and the count are
/// the host's).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
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
#[non_exhaustive]
pub enum ChannelOverlay {
    /// Light (RGB): fills add where they overlap, on the dark plot
    /// (the RGB look of [`HistogramData`]).
    #[default]
    Light,
    /// Inks (CMYK): fills multiply where they overlap
    /// (the inks look of [`HistogramData`]).
    Ink,
}

/// The histograms the host counted for the colour mode, and how fresh they are.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistogramSet<'a> {
    pub(super) channels: &'a [ScopeChannel<'a>],
    pub(super) original: Option<&'a [ScopeChannel<'a>]>,
    pub(super) colors: Option<ChannelOverlay>,
    pub(super) stale: bool,
    pub(super) cache_level: u32,
}

impl<'a> ScopeChannel<'a> {
    /// A channel `key` named `label` with the counts `bins`, drawn in `ink`.
    pub const fn new(key: &'a str, label: &'a str, ink: ChannelColor, bins: &'a [u32], color_channel: bool) -> Self {
        ScopeChannel { key, label, ink, bins, color_channel }
    }
}

impl<'a> ScopeSource<'a> {
    /// A source `key` named `label`.
    pub const fn new(key: &'a str, label: &'a str) -> Self {
        ScopeSource { key, label }
    }
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
    /// mark, whose click asks for a refresh.
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
#[non_exhaustive]
pub enum ScopeReadout {
    /// R, G, B in 0..=1, shown in 0..=255 without decimals.
    Rgb([f32; 3]),
    /// L, a, b, shown with one decimal.
    Lab([f32; 3]),
}

/// Everything the host counted for one pass: the histograms and, for Camera Raw's scope, the
/// vectorscope, the clipping state and the hover sample.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ScopeSource<'a> {
    /// A stable key, kept in [`ScopeSettings::source`].
    pub key: &'a str,
    /// The name, translated by the host.
    pub label: &'a str,
}

/// The panel's view (its menu's Compact, Expanded and All Channels View).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ScopePanelState {
    /// The persisted section.
    pub settings: ScopeSettings,
    /// The level range dragged on the plot, in levels; `None` when nothing is selected.
    pub range: Option<LevelRange>,
}

/// The columns of the statistics grid: label and value, twice.
static STATISTICS_COLUMNS: [ColumnWidth; 4] = [ColumnWidth::Content; 4];

/// Runs `content` in a block that resolves the dark mode when `dark`, else in the flow.
fn themed<R>(ui: &mut Ui, dark: bool, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    if !dark {
        return Some(content(ui));
    }
    let mut content = Some(content);
    Stack::new().theme(ThemeScope::mode(Mode::Dark)).show(ui, |ui, _| content.take().map(|content| content(ui))).inner.flatten()
}

/// Photoshop's Histogram panel and Camera Raw's scope in one widget.
///
/// **What the user sees.**
/// - **Compact View**: the plot alone; a single channel in its colour.
/// - **Expanded View**: the Channel row (a `Select` of the set's channels, then Colors when offered; the Uncached
///   Refresh icon button at its end), the plot, the Source row (only with two or more [`sources`](Self::sources)), and
///   the statistics grid when Show Statistics is on.
/// - **All Channels View**: Expanded plus one short plot per colour channel, captioned with the channel's name.
/// - The plot is the [`Histogram`]: a single channel in its colour (in Expanded and All Channels only with Show
///   Channels in Color, else neutral), Colors as overlaid ribbons ([`ChannelOverlay`]) on the dark scope, the original
///   outlined over it, the host's [`markers`](Self::markers) and the stale mark. A channel the mode lacks shows the
///   composite.
/// - The statistics: a grid of two label and value pairs per row: Mean, Std Dev, Median, Pixels for the whole
///   histogram (Colors counts the composite), Level, Count, Percentile and Cache Level for the hovered level or the
///   selected range.
/// - With [`ScopeFrame::clipping`]: the two [`ClipWarning`] buttons in the plot's top corners, shadows at the left and
///   highlights at the right. With [`ScopeFrame::vectorscope`] and Show Vectorscope: the [`VectorscopePlot`] under the
///   plot. With a [`readout`](ScopeFrame::readout): the badge in the plot's top centre.
/// - With [`zones`](Self::zones): the [`ToneZones`] over the plot.
///
/// **Pointer.** Hover reports the level and fills Level, Count and Percentile; a drag selects a level range (arrows
/// move it, Esc or a click clears it), except with tone zones, where the drag edits the zone; a double click, a click
/// on the stale mark or the refresh button reports [`refresh`](ScopePanelOutput::refresh). Right click, the Menu key
/// or Shift+F10 on a plot opens the scope menu when the frame has a vectorscope or clipping.
///
/// **Keys.** The panel body is the instance of the `panel.scopes` command scope: while focus is in it, U toggles the
/// shadow warning and O the highlight warning ([`ScopeCommand`]); S is the host's.
///
/// **What the app gets back.** [`ScopePanelOutput`]; settings are edited in the state it was given and reported with
/// `settings_changed`. The app recounts, persists and maps zone edits.
#[must_use = "show the panel with `show`"]
#[derive(Pattern, craft_ui::contract::Props)]
pub struct ScopePanel<'a> {
    state: &'a mut ScopePanelState,
    /// This pass's counted data. Without a frame the panel draws nothing (the host shows "Calculating..." itself).
    #[prop]
    frame: Option<&'a ScopeFrame<'a>>,
    /// The Source list's entries; the row shows only with two or more.
    #[prop]
    sources: &'a [ScopeSource<'a>],
    /// Ticks at the foot of the plot, `(level, label)`: the Color Samplers' levels in the shown channel.
    #[prop]
    markers: &'a [(f32, Label)],
    zones: &'a [ToneZone],
    zone_values: &'a [Label],
    /// The document has a selection: the scope menu offers Show Selected Region.
    #[prop]
    has_selection: bool,
}

impl<'a> ScopePanel<'a> {
    /// A scope panel over `state`.
    pub fn new(state: &'a mut ScopePanelState) -> Self {
        ScopePanel { state, frame: None, sources: &[], markers: &[], zones: &[], zone_values: &[], has_selection: false }
    }

    /// The tone zones over the plot ([`ToneZone::CAMERA_RAW`] or [`ToneZone::LIGHTROOM`]) and each zone's value text,
    /// in the same order (the tooltip and the slider's value).
    pub fn zones(mut self, zones: &'a [ToneZone], values: &'a [Label]) -> Self {
        self.zones = zones;
        self.zone_values = values;
        self
    }

    /// Draws the view the settings ask for and reads this pass's input, in order: the commands fired at the panel, the
    /// Channel row, the plot (with zones, warnings, badge), the vectorscope, the Source row, the statistics and the
    /// All Channels plots.
    pub fn show(self, ui: &mut Ui) -> ScopePanelOutput {
        let env = Env::of(ui);
        let words = env.words::<ScopeWords>();
        let mut pass = Pass { panel: self, words, env, report: Report::default() };
        let label = Label::from(pass.words.histogram.clone());
        let shown = Stack::new().label(label).command_scope(ScopeCommand::SCOPE).show(ui, |ui, cx| pass.rows(ui, cx));
        let Report { settings_changed, refresh, source_changed, channel_changed, zone, hovered_level, clipping_toggled } = pass.report;
        ScopePanelOutput {
            response: shown.output,
            settings_changed,
            refresh,
            source_changed,
            channel_changed,
            zone,
            hovered_level,
            range: pass.panel.state.range,
            clipping_toggled,
        }
    }
}

/// What a pass found out, filled in as the rows run.
#[derive(Default)]
struct Report {
    settings_changed: bool,
    refresh: bool,
    source_changed: bool,
    channel_changed: bool,
    zone: Option<EditEvent<ScopeZoneEdit>>,
    hovered_level: Option<f32>,
    clipping_toggled: bool,
}

/// The panel and what its rows share in one pass.
struct Pass<'a> {
    panel: ScopePanel<'a>,
    words: std::sync::Arc<ScopeWords>,
    env: Env,
    report: Report,
}

impl Pass<'_> {
    fn rows(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let has_selection = self.panel.has_selection;
        for (command, state) in ScopeCommand::states(self.panel.state, has_selection) {
            cx.publish(command, state);
        }
        let clipping = self.panel.frame.is_some_and(|frame| frame.clipping.is_some());
        while let Some((command, fired)) = cx.take_command::<ScopeCommand>() {
            let warning = matches!(command, ScopeCommand::ShadowWarning | ScopeCommand::HighlightWarning);
            if fired.phase == FirePhase::Release || (warning && !clipping) {
                continue;
            }
            match ScopePanel::run_command(self.panel.state, command) {
                Some(ScopeMenuAction::SettingsChanged) => {
                    self.report.settings_changed = true;
                    self.report.clipping_toggled |= warning;
                }
                Some(ScopeMenuAction::Refresh) => self.report.refresh = true,
                None => {}
            }
        }
        let Some(frame) = self.panel.frame else { return };
        let set = &frame.histogram;
        let Some(composite) = set.channels.first() else { return };
        let words = self.words.clone();
        let view = self.panel.state.settings.view;
        let shown = resolve_channel(set, &self.panel.state.settings.channel).unwrap_or(Shown::Channel(*composite));
        let shown_key = match shown {
            Shown::Channel(channel) => channel.key,
            Shown::Colors(_) => COLORS_CHANNEL,
        };
        let expanded = view != ScopeView::Compact;
        let colored = !expanded || self.panel.state.settings.show_channels_in_color;

        if expanded {
            self.channel_row(ui, set, shown_key);
        }
        let shown = resolve_channel(set, &self.panel.state.settings.channel).unwrap_or(shown);

        let source = self
            .panel
            .sources
            .iter()
            .find(|source| source.key == self.panel.state.settings.source)
            .or(self.panel.sources.first())
            .map_or(words.entire_image.as_str(), |source| source.label);
        let plot = plot_data(set, shown, colored, words.colors.as_str());
        let value = Label::from(words.histogram_value.fill(&[("channel", &plot.name), ("source", source)]));
        let zoned = !self.panel.zones.is_empty();
        let stale = (set.stale || set.cache_level > 1).then(|| StaleMark::Button(Label::from(words.refresh_hint.clone())));
        let dark = matches!(shown, Shown::Colors(_));
        let plot_label = Label::from(words.histogram.clone());
        let (markers, state) = (self.panel.markers, &mut *self.panel.state);
        let histogram = themed(ui, dark, |ui| {
            let mut histogram = Histogram::new(plot_label.clone(), &plot.data).value(value).markers(markers).height(PlotHeight::Standard);
            if let Some(outline) = &plot.outline {
                histogram = histogram.outline(outline);
            }
            if let Some(stale) = stale {
                histogram = histogram.stale(stale);
            }
            // The selection and the zones both take the plot's drags and keys: with zones there is no selection.
            if zoned { histogram.show(ui) } else { histogram.selection(Bind::bound(&mut state.range)).show(ui) }
        });
        let Some(histogram) = histogram else { return };
        let rect = histogram.response.rect;
        self.report.hovered_level = histogram.hovered_level;
        self.report.refresh |= histogram.refresh;
        let mut openers = vec![histogram.response.clone()];
        if zoned {
            let edit = ToneZones::new(words.tone_zones.clone(), self.panel.zones, self.panel.zone_values).show_at(ui, rect);
            self.report.zone = edit.event;
        }
        if let Some(clipping) = frame.clipping {
            self.warnings(ui, rect, clipping);
        }
        if let Some(readout) = frame.readout {
            self.readout(ui, rect, readout);
        }

        let settings = &self.panel.state.settings;
        if settings.vectorscope
            && let Some(vectorscope) = frame.vectorscope.as_ref()
        {
            let label = Label::from(words.vectorscope.clone());
            let plot = themed(ui, true, |ui| {
                VectorscopePlot::new(label.clone(), vectorscope).red_right(settings.red_right).skin_tone(settings.skin_tone).show(ui)
            });
            openers.extend(plot);
        }

        if expanded && self.panel.sources.len() >= 2 {
            self.source_row(ui);
        }

        if expanded && self.panel.state.settings.show_statistics {
            let levels = match self.panel.state.range {
                Some(range) => Some([level_of(range.start), level_of(range.end)]),
                None => self.report.hovered_level.map(|level| [level_of(level); 2]),
            };
            if let Some(stats) = histogram_statistics(shown_bins(set, shown), levels) {
                self.statistics(ui, &stats, set.cache_level);
            }
        }

        if view == ScopeView::AllChannels {
            for channel in set.channels.iter().filter(|channel| channel.color_channel) {
                Text::new(channel.label.to_owned()).role(TypeRole::Caption).show(ui);
                let ink = if colored { channel.ink } else { ChannelColor::Neutral };
                let data = HistogramData::tone(channel.bins, ink);
                let value = Label::from(words.histogram_value.fill(&[("channel", channel.label), ("source", source)]));
                Histogram::new(words.histogram.clone(), &data).id_salt(channel.key).value(value).height(PlotHeight::Short).show(ui);
            }
        }

        if frame.vectorscope.is_some() || frame.clipping.is_some() {
            let commands = ScopeCommand::context_menu(self.panel.state, has_selection);
            for opener in &openers {
                Menu::context(words.scope_menu.clone(), opener).show(ui, |ui, _| {
                    for command in &commands {
                        MenuItem::command(*command).keep_open(true).show(ui);
                    }
                });
            }
        }
    }

    /// The Channel row: the select of the set's channels, then the Uncached Refresh button at the end.
    fn channel_row(&mut self, ui: &mut Ui, set: &HistogramSet<'_>, shown_key: &str) {
        let words = self.words.clone();
        let mut entries: Vec<Entry<String>> =
            set.channels.iter().map(|channel| Entry::new(channel.key.to_owned(), channel.label.to_owned())).collect();
        if set.colors.is_some() {
            entries.push(Entry::new(COLORS_CHANNEL.to_owned(), words.colors.clone()));
        }
        let mut picked = None;
        let mut refresh = false;
        Row::new().show(ui, |ui, _| {
            let field = Field::new(words.channel.clone())
                .side(SideLabel::new().role(FieldLabelRole::Describes))
                .show(ui, |ui, _| Select::new(words.channel_name.clone(), Bind::given(shown_key.to_owned()), &entries).show(ui));
            picked = field.inner.and_then(|select| select.value().cloned());
            let button = IconButton::new(IconId::new("rotate-cw")).label(words.uncached_refresh.clone()).size(ControlSize::Sm).show(ui);
            refresh = clicked(&button);
        });
        self.report.refresh |= refresh;
        if let Some(picked) = picked.filter(|picked| picked != shown_key) {
            self.panel.state.settings.channel = picked;
            self.report.channel_changed = true;
            self.report.settings_changed = true;
        }
    }

    /// The Source row.
    fn source_row(&mut self, ui: &mut Ui) {
        let words = self.words.clone();
        let sources = self.panel.sources;
        let current = sources.iter().find(|source| source.key == self.panel.state.settings.source).unwrap_or(&sources[0]).key;
        let entries: Vec<Entry<String>> = sources.iter().map(|source| Entry::new(source.key.to_owned(), source.label.to_owned())).collect();
        let mut picked = None;
        Field::new(words.source.clone()).side(SideLabel::new().role(FieldLabelRole::Describes)).show(ui, |ui, _| {
            picked = Select::new(words.source_name.clone(), Bind::given(current.to_owned()), &entries).show(ui).value().cloned();
        });
        if let Some(picked) = picked.filter(|picked| picked != current) {
            self.panel.state.settings.source = picked;
            self.report.source_changed = true;
            self.report.settings_changed = true;
        }
    }

    /// The two clipping warning buttons in the plot's top corners.
    fn warnings(&mut self, ui: &mut Ui, plot: Rect, clipping: ClippingFrame) {
        let words = self.words.clone();
        let settings = &mut self.panel.state.settings;
        let mut shadows = ClipWarning::new(words.shadow_warning.clone(), Bind::bound(&mut settings.shadow_warning), clipping.shadows);
        let side = shadows.measure_in(ui, Proposal::Ideal);
        let toggled_shadows = shadows.show_at(ui, Rect::from_min_size(plot.min, side)).event.is_some();
        let highlights = ClipWarning::new(words.highlight_warning.clone(), Bind::bound(&mut settings.highlight_warning), clipping.highlights);
        let toggled_highlights = highlights.show_at(ui, Rect::from_min_size(pos2(plot.right() - side.x, plot.top()), side)).event.is_some();
        if toggled_shadows || toggled_highlights {
            self.report.clipping_toggled = true;
            self.report.settings_changed = true;
        }
    }

    /// The hover readout badge in the plot's top centre, between the two warning buttons.
    fn readout(&self, ui: &mut Ui, plot: Rect, readout: ScopeReadout) {
        let words = &self.words;
        let number = |value: f32, decimals: usize, scale: f64| craft_i18n::ltr(&self.env.format().number(f64::from(value) * scale, decimals));
        let text = match readout {
            ScopeReadout::Rgb([r, g, b]) => {
                words.readout_rgb.fill(&[("r", &number(r, 0, 255.0)), ("g", &number(g, 0, 255.0)), ("b", &number(b, 0, 255.0))])
            }
            ScopeReadout::Lab([l, a, b]) => {
                words.readout_lab.fill(&[("l", &number(l, 1, 1.0)), ("a", &number(a, 1, 1.0)), ("b", &number(b, 1, 1.0))])
            }
        };
        let mut badge = Badge::new(Label::from(text));
        let size = badge.measure_in(ui, Proposal::Ideal);
        badge.show_at(ui, Rect::from_center_size(pos2(plot.center().x, plot.top() + size.y / 2.0), size));
    }

    /// The statistics grid: two label and value pairs per row (Mean, Std Dev, Median, Pixels | Level, Count,
    /// Percentile, Cache Level).
    fn statistics(&self, ui: &mut Ui, stats: &HistogramStatistics, cache_level: u32) {
        let words = &self.words;
        // Numbers read left to right in every language.
        let number = |value: f64, decimals: usize| craft_i18n::ltr(&self.env.format().number(value, decimals));
        let level = match stats.level {
            Some([a, b]) if a == b => number(f64::from(a), 0),
            Some([a, b]) => craft_i18n::ltr(&format!("{}..{}", number(f64::from(a), 0), number(f64::from(b), 0))),
            None => String::new(),
        };
        let rows = [
            (&words.mean, number(stats.mean, 2), &words.level, level),
            (
                &words.standard_deviation,
                number(stats.standard_deviation, 2),
                &words.count,
                stats.count.map(|c| number(c as f64, 0)).unwrap_or_default(),
            ),
            (&words.median, number(f64::from(stats.median), 0), &words.percentile, stats.percentile.map(|p| number(p, 2)).unwrap_or_default()),
            (&words.pixels, number(stats.pixels as f64, 0), &words.cache_level, number(f64::from(cache_level), 0)),
        ];
        Grid::new(&STATISTICS_COLUMNS).gap(Gap::Tight).label(words.statistics.clone()).show(ui, |ui, _| {
            for (first, first_value, second, second_value) in &rows {
                Text::new(Label::clone(first)).shade(Shade::Muted).show(ui);
                Text::new(first_value.clone()).role(TypeRole::Mono).show(ui);
                Text::new(Label::clone(second)).shade(Shade::Muted).show(ui);
                Text::new(second_value.clone()).role(TypeRole::Mono).show(ui);
            }
        });
    }
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

/// What [`ScopePanel::show`] returns. The events of one pass, handled in that pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct ScopePanelOutput {
    /// The panel's area.
    pub response: Response,
    /// A setting changed (view, channel, source, a menu check, a warning): persist [`ScopeSettings`].
    pub settings_changed: bool,
    /// Uncached Refresh: recount at cache level 1 (button, stale mark, double click, menu row).
    pub refresh: bool,
    /// The Source changed: the host recounts from [`ScopeSettings::source`].
    pub source_changed: bool,
    /// The Channel changed.
    pub channel_changed: bool,
    /// A tone-zone edit, with its gesture; the host maps it to its control.
    pub zone: Option<EditEvent<ScopeZoneEdit>>,
    /// The whole level under the pointer on the main plot.
    pub hovered_level: Option<f32>,
    /// The selected level range after this pass (also in [`ScopePanelState::range`]).
    pub range: Option<LevelRange>,
    /// A clipping warning was toggled (click, U, O): the host rebuilds its overlay.
    pub clipping_toggled: bool,
}

impl Outcome for ScopePanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}
