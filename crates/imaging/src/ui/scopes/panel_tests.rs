//! The scope panel on screen (egui_kittest).

use craft_ui::composites::ChannelColor;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{
    ClippingFrame, HistogramSet, ScopeChannel, ScopeFrame, ScopePanel, ScopePanelLabels, ScopePanelOutput, ScopePanelState, ScopeSource, ScopeView,
    ScopeZoneEdit, ToneZone, ZoneChange,
};
use crate::scopes::ChannelMask;

/// The state, what the last pass reported, and which one-shot reports have come.
struct Host {
    state: ScopePanelState,
    stale: bool,
    sources: &'static [ScopeSource<'static>],
    last: Option<ScopePanelOutput>,
    refreshes: usize,
    clipping: Option<ClippingFrame>,
    toggles: usize,
    zoned: bool,
    zone_edits: Vec<ScopeZoneEdit>,
    labels: Option<ScopePanelLabels>,
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
    let host =
        Host { state, stale, sources, last: None, refreshes: 0, clipping: None, toggles: 0, zoned: false, zone_edits: Vec::new(), labels: None };
    let mut harness =
        Harness::builder().with_size([320.0, 480.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let channels = [
                    ScopeChannel { key: "rgb", label: "RGB", ink: ChannelColor::Neutral, bins: &RAMP, color_channel: false },
                    ScopeChannel { key: "red", label: "Red", ink: ChannelColor::Red, bins: &RAMP, color_channel: true },
                    ScopeChannel { key: "green", label: "Green", ink: ChannelColor::Green, bins: &RAMP, color_channel: true },
                    ScopeChannel { key: "blue", label: "Blue", ink: ChannelColor::Blue, bins: &RAMP, color_channel: true },
                ];
                let mut frame = ScopeFrame::histogram(HistogramSet::new(&channels).stale(host.stale));
                if let Some(clipping) = host.clipping {
                    frame = frame.clipping(clipping);
                }
                let values = ["0", "1", "2", "3", "4"].map(String::from);
                let mut panel = ScopePanel::new("scope", &mut host.state).frame(&frame).sources(host.sources);
                if host.zoned {
                    panel = panel.zones(&ToneZone::CAMERA_RAW, &values);
                }
                if let Some(labels) = host.labels.clone() {
                    panel = panel.labels(labels);
                }
                let out = panel.show(ui);
                host.zone_edits.extend(out.zone.clone());
                host.refreshes += usize::from(out.refresh);
                host.toggles += usize::from(out.clipping_toggled);
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

// A click, U and O toggle the warnings and report `clipping_toggled`; the buttons exist only when
// the frame carries clipping.
#[test]
fn clipping_buttons_toggle_by_click_and_keys() {
    const SHADOWS: &str = "Shadow Clipping Warning (U)";
    const HIGHLIGHTS: &str = "Highlight Clipping Warning (O)";
    let mut harness = rig(ScopeView::Compact, &[], false);
    assert!(harness.query_by_label(SHADOWS).is_none(), "no clipping in the frame, no buttons");

    harness.state_mut().clipping = Some(ClippingFrame { shadows: ChannelMask::R, highlights: ChannelMask::RGB });
    harness.run();
    let warnings = |harness: &Harness<'_, Host>| {
        let settings = &harness.state().state.settings;
        (settings.shadow_warning, settings.highlight_warning)
    };
    assert_eq!(warnings(&harness), (false, false));

    harness.get_by_role_and_label(Role::CheckBox, SHADOWS).click();
    harness.run();
    assert_eq!(warnings(&harness), (true, false), "a click on the shadow button");
    assert_eq!(harness.state().toggles, 1);

    harness.get_by_role_and_label(Role::CheckBox, HIGHLIGHTS).click();
    harness.run();
    assert_eq!(warnings(&harness), (true, true), "a click on the highlight button");
    assert_eq!(harness.state().toggles, 2);

    harness.get_by_role_and_label(Role::CheckBox, HIGHLIGHTS).focus();
    harness.run();
    harness.key_press(Key::U);
    harness.run();
    assert_eq!(warnings(&harness), (false, true), "U toggles the shadow warning");
    harness.key_press(Key::O);
    harness.run();
    assert_eq!(warnings(&harness), (false, false), "O toggles the highlight warning");
    assert_eq!(harness.state().toggles, 4);
}

fn double_click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
    harness.input_mut().events.extend([true, false, true, false].map(|pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }));
    harness.step();
    harness.run();
}

// With tone zones the plot is the one Tab stop, a double-click resets the zone under it and does
// not ask for a refresh, and the arrow keys step the focused zone.
#[test]
fn zones_share_the_plots_tab_stop_and_double_click_resets() {
    let mut harness = rig(ScopeView::Compact, &[], false);
    harness.state_mut().zoned = true;
    harness.run();
    let rect = plot(&harness);
    double_click(&mut harness, pos2(rect.left() + rect.width() * 0.5, rect.center().y));
    let host = harness.state();
    assert!(host.zone_edits.iter().any(|e| e.key == "exposure" && e.change == ZoneChange::Reset), "{:?}", host.zone_edits);
    assert_eq!(host.refreshes, 0, "a double-click on a zoned plot resets, it does not refresh");

    let mut harness = rig(ScopeView::Compact, &[], false);
    harness.state_mut().zoned = true;
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_role_and_label(Role::Image, "Histogram").is_focused(), "the first Tab lands on the plot");
    // egui takes the plot's key filter from the pass after the one that gave it focus.
    harness.key_press(Key::ArrowRight);
    harness.run();
    let steps: Vec<_> = harness.state().zone_edits.iter().map(|e| (e.key, e.change.clone())).collect();
    assert_eq!(steps, [("exposure", ZoneChange::Step(1))]);
    assert_eq!(craft_ui::testing::a11y::tab_order(&mut harness).len(), 1, "the plot is the only stop");
}

// The plot's name and value text come from the labels the host passed and the resolved Source.
#[test]
fn the_plot_reads_the_hosts_labels_and_the_resolved_source() {
    let mut harness = rig(ScopeView::Expanded, &TWO_SOURCES, false);
    let labels =
        ScopePanelLabels { histogram: "Verlauf".to_owned(), histogram_value: "{channel} / {source}".to_owned(), ..ScopePanelLabels::default() };
    harness.state_mut().labels = Some(labels);
    harness.state_mut().state.settings.source = "layer".to_owned();
    harness.run();
    let node = harness.get_by_role_and_label(Role::Image, "Verlauf");
    assert_eq!(node.value().as_deref(), Some("RGB / Selected Layer"));
}

// The scope menu's check rows keep it open, so the rows that depend on Show Vectorscope appear
// in place; Compact View and the other views are radio rows that close it.
#[test]
fn the_menus_check_rows_keep_them_open() {
    let ctx = Context::default();
    let mut state = ScopePanelState::default();
    state.settings.vectorscope = true;
    let scope = ScopePanel::scope_menu(&ctx, "scope", &state, true);
    assert_eq!(scope.len(), 5, "Lab readouts, Vectorscope, Selected Region, Red right, Skin tone");
    assert!(scope.iter().filter_map(|e| e.item()).all(|item| item.keeps_open()));

    let panel = ScopePanel::panel_menu(&ctx, "scope", &state);
    let items: Vec<_> = panel.iter().filter_map(|e| e.item()).collect();
    assert_eq!(items.iter().filter(|item| item.keeps_open()).count(), 2, "Show Statistics and Show Channels in Color");
    assert!(items.iter().take(4).all(|item| !item.keeps_open()), "Refresh and the three views close the menu");
}
