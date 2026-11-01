//! The scope panel on screen (egui_kittest).
#![expect(clippy::todo, reason = "skeleton: ST6 fills the clipping buttons test")]

use craft_ui::composites::ChannelColor;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{HistogramSet, ScopeChannel, ScopeFrame, ScopePanel, ScopePanelOutput, ScopePanelState, ScopeSource, ScopeView};

/// The state, what the last pass reported, and which one-shot reports have come.
struct Host {
    state: ScopePanelState,
    stale: bool,
    sources: &'static [ScopeSource<'static>],
    last: Option<ScopePanelOutput>,
    refreshes: usize,
}

static RAMP: [u32; 256] = {
    let mut bins = [0; 256];
    let mut i = 0;
    while i < 256 {
        bins[i] = 10 + i as u32;
        i += 1;
    }
    bins
};

fn rig(view: ScopeView, sources: &'static [ScopeSource<'static>], stale: bool) -> Harness<'static, Host> {
    let mut state = ScopePanelState::default();
    state.settings.view = view;
    let host = Host { state, stale, sources, last: None, refreshes: 0 };
    let mut harness =
        Harness::builder().with_size([320.0, 480.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let channels = [
                    ScopeChannel { key: "rgb", label: "RGB", ink: ChannelColor::Neutral, bins: &RAMP, color_channel: false },
                    ScopeChannel { key: "red", label: "Red", ink: ChannelColor::Red, bins: &RAMP, color_channel: true },
                    ScopeChannel { key: "green", label: "Green", ink: ChannelColor::Green, bins: &RAMP, color_channel: true },
                    ScopeChannel { key: "blue", label: "Blue", ink: ChannelColor::Blue, bins: &RAMP, color_channel: true },
                ];
                let frame = ScopeFrame::histogram(HistogramSet::new(&channels).stale(host.stale));
                let out = ScopePanel::new("scope", &mut host.state).frame(&frame).sources(host.sources).show(ui);
                host.refreshes += usize::from(out.refresh);
                host.last = Some(out);
            },
            host,
        );
    harness.run();
    harness
}

fn plot(harness: &Harness<'_, Host>) -> egui::Rect {
    harness.get_by(|n| n.role() == Role::Image && n.label().as_deref() == Some("Histogram")).rect()
}

fn mouse(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: Option<bool>) {
    harness.event(Event::PointerMoved(pos));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    harness.run();
}

static TWO_SOURCES: [ScopeSource<'static>; 2] =
    [ScopeSource { key: "image", label: "Entire Image" }, ScopeSource { key: "layer", label: "Selected Layer" }];

// Compact has the plot only; Expanded has the Channel row and statistics; All Channels adds one
// plot per colour channel; the Source row needs two sources.
#[test]
#[ignore = "runs against SC3's plot bodies"]
fn scope_views_show_the_right_rows() {
    let compact = rig(ScopeView::Compact, &[], false);
    assert!(compact.query_by_label("Channel:").is_none());
    assert!(compact.query_by_role_and_label(Role::Table, "Statistics").is_none());

    let expanded = rig(ScopeView::Expanded, &[], false);
    assert!(expanded.query_by_label("Channel:").is_some());
    assert!(expanded.query_by_role_and_label(Role::Table, "Statistics").is_some());
    assert!(expanded.query_by_label("Source:").is_none(), "one source shows no Source row");
    assert!(expanded.query_by_label("Green").is_none());

    let with_sources = rig(ScopeView::Expanded, &TWO_SOURCES, false);
    assert!(with_sources.query_by_label("Source:").is_some());

    let all = rig(ScopeView::AllChannels, &[], false);
    for name in ["Red", "Green", "Blue"] {
        assert!(all.query_by_label(name).is_some(), "{name} has a plot of its own");
    }
}

// Hover reports `hovered_level`; a drag reports `range` and stores it in the state.
#[test]
#[ignore = "runs against SC3's plot bodies"]
fn scope_hover_and_drag_report_levels_and_range() {
    let mut harness = rig(ScopeView::Compact, &[], false);
    let rect = plot(&harness);
    let at = |share: f32| pos2(rect.left() + share * rect.width(), rect.center().y);
    mouse(&mut harness, at(0.5), None);
    let level = harness.state().last.as_ref().and_then(|o| o.hovered_level).expect("hover reports a level");
    assert!((level - 128.0).abs() <= 1.0, "{level}");

    mouse(&mut harness, at(0.25), Some(true));
    mouse(&mut harness, at(0.75), None);
    mouse(&mut harness, at(0.75), Some(false));
    let range = harness.state().state.range.clone().expect("the drag selects a range");
    assert!((*range.start() - 64.0).abs() <= 1.0 && (*range.end() - 192.0).abs() <= 1.0, "{range:?}");
    assert_eq!(harness.state().last.as_ref().and_then(|o| o.range.clone()), Some(range));
}

// The refresh button, a click on the stale mark and a double-click on the plot report `refresh`.
#[test]
#[ignore = "runs against SC3's plot bodies"]
fn scope_refresh_is_requested_three_ways() {
    let mut harness = rig(ScopeView::Expanded, &[], true);
    harness.get_by_role_and_label(Role::Button, "Uncached Refresh").click();
    harness.run();
    assert_eq!(harness.state().refreshes, 1, "the button");

    harness.get_by_role_and_label(Role::Button, "Click to refresh histogram with uncached data").click();
    harness.run();
    assert_eq!(harness.state().refreshes, 2, "the stale mark");

    let rect = plot(&harness);
    let center = rect.center();
    harness.input_mut().events.extend([true, false, true, false].map(|pressed| Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }));
    harness.step();
    harness.run();
    assert!(harness.state().refreshes >= 3, "the double-click");
}

// A click, U and O toggle the warnings and report `clipping_toggled`; the fill shows the clipped
// channels. Needs SC2's panel and SC3's buttons: filled by ST6.
#[test]
#[ignore = "skeleton"]
fn clipping_buttons_toggle_by_click_and_keys() {
    todo!()
}
