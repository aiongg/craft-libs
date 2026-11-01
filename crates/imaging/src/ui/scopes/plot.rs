//! The histogram plot of one channel, of Colors, and the All Channels plots.

use craft_theme::TypeRole;
use craft_ui::composites::{ChannelColor, Histogram, HistogramData, HistogramResponse};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::{RichText, Ui};

use super::labels::ScopePanelLabels;
use super::panel::{COLORS_CHANNEL, ChannelOverlay, HistogramSet, ScopeChannel};

/// The channel a key shows: the set's channel with that key, else the composite (the first
/// channel); [`COLORS_CHANNEL`](super::COLORS_CHANNEL) only when the set offers Colors, else the
/// composite as well. `None` for an empty set.
pub(super) fn resolve_channel<'s>(set: &HistogramSet<'s>, key: &str) -> Option<Shown<'s>> {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:151 `resolve_channel`.
    if key == COLORS_CHANNEL
        && let Some(overlay) = set.colors
        && !set.channels.is_empty()
    {
        return Some(Shown::Colors(overlay));
    }
    set.channels.iter().find(|channel| channel.key == key).or_else(|| set.channels.first()).copied().map(Shown::Channel)
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
    let colors = || set.channels.iter().filter(|channel| channel.color_channel);
    match overlay {
        ChannelOverlay::Light => {
            let mut bins = colors().map(|channel| channel.bins);
            let mut next = || bins.next().unwrap_or(&[]);
            HistogramData::rgb([next(), next(), next()])
        }
        ChannelOverlay::Ink => {
            let inks: Vec<(&[u32], ChannelColor)> = colors().map(|channel| (channel.bins, channel.ink)).collect();
            HistogramData::channels(&inks)
        }
    }
}

/// Draws the main plot with the library [`Histogram`](craft_ui::composites::Histogram): `shown`
/// in its colour when `colored` (else `ChannelColor::Neutral`), the original outlined, the
/// markers, the stale mark (with `on_stale_click`), the selection (`range`) and the value text
/// (`labels.histogram_value` over the channel's name and `source`, the resolved Source's label).
/// With `zoned` the plot takes the pointer and the keys for the tone zones
/// ([`Histogram::interactive`]) and has no selection (`range` is ignored). Returns the histogram's response (hovered level, selection
/// change, stale click).
#[expect(clippy::too_many_arguments, reason = "the panel passes the plot's inputs once per pass")]
pub(super) fn main_plot(
    ui: &mut Ui,
    id: egui::Id,
    labels: &ScopePanelLabels,
    source: &str,
    set: &HistogramSet<'_>,
    shown: Shown<'_>,
    colored: bool,
    height: f32,
    markers: &[(f32, String)],
    range: Option<std::ops::RangeInclusive<f32>>,
    zoned: bool,
) -> HistogramResponse {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:636 `graph` (band, outline, sampler ticks, cached mark, double-click).
    let (data, name, original) = match shown {
        Shown::Channel(channel) => {
            let ink = if colored { channel.ink } else { ChannelColor::Neutral };
            (HistogramData::tone(channel.bins, ink), channel.label, set.original.and_then(|o| o.iter().find(|c| c.key == channel.key)))
        }
        Shown::Colors(overlay) => (colors_data(set, overlay), labels.colors.as_str(), set.original.and_then(|o| o.first())),
    };
    let value = craft_i18n::fmt(&labels.histogram_value, &[("channel", name), ("source", source)]);
    let mut histogram = Histogram::new(id, labels.histogram.clone())
        .data(data)
        .markers(markers)
        .stale(set.stale || set.cache_level > 1)
        .on_stale_click(labels.refresh_hint.clone())
        .value(value)
        .height(height)
        .interactive(zoned);
    // The selection and the zones both take the plot's keys and drags: with zones there is none.
    if !zoned {
        histogram = histogram.selection(range);
    }
    if let Some(original) = original {
        histogram = histogram.outline(original.bins, ChannelColor::Neutral);
    }
    histogram.show(ui)
}

/// All Channels View's extra plots: for each colour channel its label (`TypeRole::Caption`, the
/// channel's name) and a hover-only plot `height` high, in its colour when `colored`, named and
/// valued from `labels` and `source` as the main plot is.
pub(super) fn all_channels(ui: &mut Ui, id: egui::Id, labels: &ScopePanelLabels, source: &str, set: &HistogramSet<'_>, colored: bool, height: f32) {
    // pc: crates/ui-egui/src/panels/histogram_panel.rs:792-801.
    let tokens = Tokens::of(ui);
    let caption = tokens.font(TypeRole::Caption);
    let text = tokens.color.text.normal.to_egui();
    for channel in set.channels.iter().filter(|channel| channel.color_channel) {
        ui.label(RichText::new(channel.label).font(caption.clone()).color(text));
        let ink = if colored { channel.ink } else { ChannelColor::Neutral };
        let value = craft_i18n::fmt(&labels.histogram_value, &[("channel", channel.label), ("source", source)]);
        Histogram::new((id, channel.key), labels.histogram.clone()).tone(channel.bins, ink).value(value).height(height).show(ui);
    }
}
