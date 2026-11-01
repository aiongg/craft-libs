//! The histogram plot of one channel, of Colors, and the All Channels plots.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::composites::{HistogramData, HistogramResponse};
use egui::Ui;

use super::panel::{ChannelOverlay, HistogramSet, ScopeChannel};

/// The channel a key shows: the set's channel with that key, else the composite (the first
/// channel); [`COLORS_CHANNEL`](super::COLORS_CHANNEL) only when the set offers Colors, else the
/// composite as well. `None` for an empty set.
pub(super) fn resolve_channel<'s>(set: &HistogramSet<'s>, key: &str) -> Option<Shown<'s>> {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:151 `resolve_channel`.
    todo!()
}

/// What the main plot shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Shown<'s> {
    /// One channel.
    Channel(ScopeChannel<'s>),
    /// The colour channels overlaid.
    Colors(ChannelOverlay),
}

/// The data the plot draws for the Colors entry: `Light` is [`HistogramData::rgb`] of the set's
/// three colour channels (fills add); `Ink` is [`HistogramData::channels`] of its colour channels
/// with their inks (fills multiply). Colors is always in colour.
pub(super) fn colors_data(set: &HistogramSet<'_>, overlay: ChannelOverlay) -> HistogramData {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:626-629 (`draw_overlaid`, multiply for CMYK), :537 `mix`.
    todo!()
}

/// Draws the main plot with the library [`Histogram`](craft_ui::composites::Histogram): `shown`
/// in its colour when `colored` (else `ChannelColor::Neutral`), the original outlined, the
/// markers, the stale mark (with `on_stale_click`), the selection (`range`) and the value text.
/// Returns the histogram's response (hovered level, selection change, stale click).
#[expect(clippy::too_many_arguments, reason = "skeleton: the fill may fold these into a context struct")]
pub(super) fn main_plot(
    ui: &mut Ui,
    id: egui::Id,
    set: &HistogramSet<'_>,
    shown: Shown<'_>,
    colored: bool,
    height: f32,
    markers: &[(f32, String)],
    range: Option<std::ops::RangeInclusive<f32>>,
) -> HistogramResponse {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:636 `graph` (band, outline, sampler ticks, cached mark, double-click).
    todo!()
}

/// All Channels View's extra plots: for each colour channel its label (`TypeRole::Caption`, the
/// channel's name) and a hover-only plot `height` high, in its colour when `colored`.
pub(super) fn all_channels(ui: &mut Ui, id: egui::Id, set: &HistogramSet<'_>, colored: bool, height: f32) {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:792-801.
    todo!()
}
