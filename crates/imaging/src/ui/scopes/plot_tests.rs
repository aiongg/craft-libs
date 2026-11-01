//! The plot's data choices.

use craft_ui::composites::{ChannelColor, HistogramData};

use super::panel::{COLORS_CHANNEL, ChannelOverlay, HistogramSet, ScopeChannel};
use super::plot::{Shown, colors_data, resolve_channel};

const COMPOSITE: [u32; 4] = [1, 2, 3, 4];
const RED: [u32; 4] = [4, 0, 0, 1];
const GREEN: [u32; 4] = [0, 5, 0, 0];
const BLUE: [u32; 4] = [0, 0, 6, 2];

fn channel<'a>(key: &'a str, ink: ChannelColor, bins: &'a [u32], color_channel: bool) -> ScopeChannel<'a> {
    ScopeChannel { key, label: key, ink, bins, color_channel }
}

fn rgb() -> [ScopeChannel<'static>; 4] {
    [
        channel("rgb", ChannelColor::Neutral, &COMPOSITE, false),
        channel("red", ChannelColor::Red, &RED, true),
        channel("green", ChannelColor::Green, &GREEN, true),
        channel("blue", ChannelColor::Blue, &BLUE, true),
    ]
}

// Colors in an RGB set is `HistogramData::rgb` (additive); in a CMYK set `HistogramData::channels`
// of the four inks (multiplied).
#[test]
fn scope_colors_channel_overlays_light_and_ink() {
    let channels = rgb();
    let set = HistogramSet::new(&channels).colors(ChannelOverlay::Light);
    assert_eq!(colors_data(&set, ChannelOverlay::Light), HistogramData::rgb([&RED[..], &GREEN[..], &BLUE[..]]));

    let inks = [
        channel("cyan", ChannelColor::Cyan, &RED, true),
        channel("magenta", ChannelColor::Magenta, &GREEN, true),
        channel("black", ChannelColor::Neutral, &BLUE, true),
    ];
    let set = HistogramSet::new(&inks).colors(ChannelOverlay::Ink);
    assert_eq!(
        colors_data(&set, ChannelOverlay::Ink),
        HistogramData::channels(&[(&RED[..], ChannelColor::Cyan), (&GREEN[..], ChannelColor::Magenta), (&BLUE[..], ChannelColor::Neutral)])
    );
}

// A channel key the mode lacks resolves to the composite; Colors without `colors` does too.
#[test]
fn scope_channel_list_follows_the_mode_and_falls_back_to_composite() {
    let channels = rgb();
    let plain = HistogramSet::new(&channels);
    let key = |shown: Option<Shown<'_>>| match shown {
        Some(Shown::Channel(channel)) => channel.key.to_owned(),
        Some(Shown::Colors(_)) => "colors".to_owned(),
        None => "none".to_owned(),
    };
    assert_eq!(key(resolve_channel(&plain, "green")), "green");
    assert_eq!(key(resolve_channel(&plain, "cyan")), "rgb");
    assert_eq!(key(resolve_channel(&plain, "")), "rgb");
    assert_eq!(key(resolve_channel(&plain, COLORS_CHANNEL)), "rgb");
    let with_colors = HistogramSet::new(&channels).colors(ChannelOverlay::Light);
    assert_eq!(key(resolve_channel(&with_colors, COLORS_CHANNEL)), "colors");
    assert_eq!(key(resolve_channel(&HistogramSet::new(&[]), "red")), "none");
}
