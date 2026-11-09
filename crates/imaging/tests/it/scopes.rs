//! The scope panel and its parts: rows per view, the plot's levels and range, the three refresh paths, the clipping
//! buttons, the tone zones and the commands.

use craft_behavior::EditEvent;
use craft_imaging::scopes::ChannelMask;
use craft_imaging::ui::histogram::ChannelColor;
use craft_imaging::ui::{
    ClippingFrame, HistogramSet, ScopeChannel, ScopeCommand, ScopeFrame, ScopePanel, ScopePanelState, ScopeSettings, ScopeSource, ScopeView,
    ScopeWords, ScopeZoneEdit, ToneZone, ToneZones, VectorscopeFrame, ZoneChange, scope_point,
};
use craft_ui::contract::{Role, Show as _};
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Rect, pos2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

static RAMP: [u32; 256] = {
    let mut bins = [0; 256];
    let mut i = 0;
    while i < 256 {
        bins[i] = 10 + i as u32;
        i += 1;
    }
    bins
};

const TWO_SOURCES: [ScopeSource<'static>; 2] = [ScopeSource::new("image", "Entire Image"), ScopeSource::new("layer", "Selected Layer")];

#[derive(Default)]
struct Host {
    state: ScopePanelState,
    stale: bool,
    sources: bool,
    clipping: Option<ClippingFrame>,
    zoned: bool,
    zone_edits: Vec<EditEvent<ScopeZoneEdit>>,
    refreshes: usize,
    toggles: usize,
    hovered: Option<f32>,
}

fn rig(view: ScopeView, sources: bool, stale: bool) -> (Fixture, Rc<RefCell<Host>>) {
    let host = Rc::new(RefCell::new(Host { stale, sources, ..Host::default() }));
    host.borrow_mut().state.settings.view = view;
    let shared = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shared.borrow_mut();
        let channels = [
            ScopeChannel::new("rgb", "RGB", ChannelColor::Neutral, &RAMP, false),
            ScopeChannel::new("red", "Red", ChannelColor::Red, &RAMP, true),
            ScopeChannel::new("green", "Green", ChannelColor::Green, &RAMP, true),
            ScopeChannel::new("blue", "Blue", ChannelColor::Blue, &RAMP, true),
        ];
        let mut frame = ScopeFrame::histogram(HistogramSet::new(&channels).stale(host.stale));
        if let Some(clipping) = host.clipping {
            frame = frame.clipping(clipping);
        }
        let values: Vec<Label> = ["0", "1", "2", "3", "4"].map(Label::from).to_vec();
        let (zoned, sources) = (host.zoned, host.sources);
        let zones = ToneZone::CAMERA_RAW;
        let Host { state, .. } = &mut *host;
        let mut panel = ScopePanel::new(state).frame(&frame);
        if sources {
            panel = panel.sources(&TWO_SOURCES);
        }
        if zoned {
            panel = panel.zones(&zones, &values);
        }
        let out = panel.show(ui);
        host.zone_edits.extend(out.zone);
        host.refreshes += usize::from(out.refresh);
        host.toggles += usize::from(out.clipping_toggled);
        host.hovered = out.hovered_level;
    });
    fixture.settle();
    (fixture, host)
}

fn plot(fixture: &Fixture) -> Rect {
    fixture.by_label("Histogram").map(|target| target.rect).expect("the plot is shown")
}

#[trial]
fn scope_views_show_the_right_rows() {
    let (compact, _) = rig(ScopeView::Compact, false, false);
    assert!(compact.by_label("Channel:").is_none());
    assert!(compact.by_label("Statistics").is_none());

    let (expanded, _) = rig(ScopeView::Expanded, false, false);
    assert!(expanded.by_label("Channel").is_some(), "the Channel list is named by its own word");
    assert!(expanded.by_label("Statistics").is_some());
    assert!(expanded.by_label("Source").is_none(), "one source shows no Source row");

    let (with_sources, _) = rig(ScopeView::Expanded, true, false);
    assert!(with_sources.by_label("Source").is_some());

    let (all, _) = rig(ScopeView::AllChannels, false, false);
    assert!(all.by_role(Role::IMAGE).len() >= 4, "the main plot and one per colour channel");
}

#[trial]
fn scope_hover_and_drag_report_levels_and_range() {
    let (mut fixture, host) = rig(ScopeView::Compact, false, false);
    let rect = plot(&fixture);
    let at = |share: f32| pos2(rect.left() + share * rect.width(), rect.center().y);
    fixture.frame(vec![egui::Event::PointerMoved(at(0.5))]).settle();
    let middle = host.borrow().hovered.expect("hover reports a level");
    fixture.frame(vec![egui::Event::PointerMoved(at(0.25))]).settle();
    let low = host.borrow().hovered.expect("hover reports a level");
    assert!(low < middle, "levels grow to the right: {low} then {middle}");

    let target = fixture.by_label("Histogram").expect("the plot is shown");
    fixture.drag(&target, vec2(rect.width() * 0.5, 0.0)).settle();
    let range = host.borrow().state.range.expect("the drag selects a range");
    assert!(range.end > range.start, "{range:?}");
}

#[trial]
fn scope_refresh_is_requested_three_ways() {
    let (mut fixture, host) = rig(ScopeView::Expanded, false, true);
    let button = fixture.by_label("Uncached Refresh").expect("the refresh button");
    fixture.click(&button).settle();
    assert_eq!(host.borrow().refreshes, 1, "the button");

    let mark = fixture.by_label("Click to refresh histogram with uncached data").expect("the stale mark");
    fixture.click(&mark).settle();
    assert_eq!(host.borrow().refreshes, 2, "the stale mark");

    let center = plot(&fixture).center();
    fixture.click_at(center).click_at(center).settle();
    assert!(host.borrow().refreshes >= 3, "the double click");
}

#[trial]
fn clipping_buttons_toggle_by_click_and_keys() {
    const SHADOWS: &str = "Shadow Clipping Warning (U)";
    const HIGHLIGHTS: &str = "Highlight Clipping Warning (O)";
    let (mut fixture, host) = rig(ScopeView::Compact, false, false);
    assert!(fixture.by_label(SHADOWS).is_none(), "no clipping in the frame, no buttons");

    host.borrow_mut().clipping = Some(ClippingFrame::new(ChannelMask::R, ChannelMask::RGB));
    fixture.settle();
    let warnings = || {
        let host = host.borrow();
        (host.state.settings.shadow_warning, host.state.settings.highlight_warning)
    };
    assert_eq!(warnings(), (false, false));

    let shadows = fixture.by_label(SHADOWS).expect("the shadow button");
    fixture.click(&shadows).settle();
    assert_eq!(warnings(), (true, false), "a click on the shadow button");
    let highlights = fixture.by_label(HIGHLIGHTS).expect("the highlight button");
    fixture.click(&highlights).settle();
    assert_eq!(warnings(), (true, true), "a click on the highlight button");
    assert_eq!(host.borrow().toggles, 2);

    fixture.key(Key::U, Modifiers::NONE).settle();
    assert_eq!(warnings(), (false, true), "U toggles the shadow warning");
    fixture.key(Key::O, Modifiers::NONE).settle();
    assert_eq!(warnings(), (false, false), "O toggles the highlight warning");
}

#[trial]
fn zones_share_the_plots_tab_stop_and_double_click_resets() {
    let (mut fixture, host) = rig(ScopeView::Compact, false, false);
    host.borrow_mut().zoned = true;
    fixture.settle();
    let rect = plot(&fixture);
    let middle = pos2(rect.left() + rect.width() * 0.5, rect.center().y);
    fixture.click_at(middle).click_at(middle).settle();
    let edits = host.borrow().zone_edits.iter().filter_map(|edit| edit.value().copied()).collect::<Vec<_>>();
    assert!(edits.iter().any(|edit| edit.key == "exposure" && edit.change == ZoneChange::Reset), "{edits:?}");
    assert_eq!(host.borrow().refreshes, 0, "a double click on a zoned plot resets, it does not refresh");
}

#[trial]
fn the_menus_check_rows_keep_them_open() {
    let rows = ScopePanel::panel_rows();
    assert!(!rows.is_empty());

    let mut state = ScopePanelState::default();
    let before = state.settings.show_statistics;
    ScopePanel::run_command(&mut state, ScopeCommand::ShowStatistics);
    assert_eq!(state.settings.show_statistics, !before);
    ScopePanel::run_command(&mut state, ScopeCommand::AllChannelsView);
    assert_eq!(state.settings.view, ScopeView::AllChannels);
    state.settings.vectorscope = true;
    let commands = ScopeCommand::context_menu(&state, true);
    assert_eq!(commands.len(), ScopeCommand::context_menu(&state, false).len() + 1, "Selected Region needs a selection");
}

#[trial]
fn scope_settings_survive_a_serde_round_trip() {
    let mut settings = ScopeSettings::default();
    settings.view = ScopeView::AllChannels;
    settings.channel = "red".into();
    settings.show_channels_in_color = true;
    settings.show_statistics = false;
    settings.source = "layer".into();
    settings.lab_readouts = true;
    settings.vectorscope = true;
    settings.selected_region = true;
    settings.red_right = true;
    settings.skin_tone = false;
    settings.shadow_warning = true;
    settings.highlight_warning = true;
    let json = serde_json::to_value(&settings).expect("settings serialize");
    assert_eq!(json["view"], "allChannels");
    assert_eq!(json["showChannelsInColor"], true);
    assert_eq!(serde_json::from_value::<ScopeSettings>(json.clone()).expect("settings read back"), settings);

    let mut extra = json;
    extra["fromANewerVersion"] = serde_json::json!(7);
    assert_eq!(serde_json::from_value::<ScopeSettings>(extra).expect("unknown fields are ignored"), settings);

    let partial: ScopeSettings = serde_json::from_str(r#"{"view":"expanded","redRight":true}"#).expect("missing fields take defaults");
    assert_eq!((partial.view, partial.red_right), (ScopeView::Expanded, true));
    assert!(partial.show_statistics && partial.skin_tone, "the defaults that are on stay on");
}

#[trial]
fn vectorscope_places_red_at_the_chosen_angle() {
    let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(210.0, 120.0));
    let (center, radius) = (rect.center(), 36.0);
    let near = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-3;
    assert!(near(scope_point(rect, 14.0, true, 0.0, 1.0), center + vec2(radius, 0.0)));
    assert!(near(scope_point(rect, 14.0, true, 0.0, 0.5), center + vec2(radius * 0.5, 0.0)));
    let angle = -100.0f32.to_radians();
    assert!(near(scope_point(rect, 14.0, false, 0.0, 1.0), center + vec2(angle.cos(), angle.sin()) * radius));
    assert!(near(scope_point(rect, 14.0, true, 0.0, 0.0), center));
}

#[trial]
fn the_margin_sets_the_rings_radius() {
    let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 100.0));
    assert!((scope_point(rect, 20.0, true, 0.0, 1.0).x - (rect.center().x + 30.0)).abs() < 1e-3);
}

#[trial]
fn the_summary_uses_the_locale_digits() {
    let counts = [0, 0, 0, 5];
    let frame = VectorscopeFrame::new(&counts, 2, 2, None);
    let words = ScopeWords::default();
    let latin = frame.summary(&words, &craft_i18n::Format::from_tag("en")).expect("counted");
    let arabic = frame.summary(&words, &craft_i18n::Format::from_tag("ar-EG")).expect("counted");
    assert!(latin.chars().any(|c| c.is_ascii_digit()));
    assert!(!arabic.chars().any(|c| c.is_ascii_digit()), "{arabic}");
}

/// A fixture of the tone zones alone over a 200 by 100 plot at the origin, and the edits they report.
fn zone_rig() -> (Fixture, Rc<RefCell<Vec<EditEvent<ScopeZoneEdit>>>>) {
    let edits = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&edits);
    let mut fixture = Fixture::new(move |ui| {
        let zones = ToneZone::CAMERA_RAW;
        let texts: Vec<Label> = ["0", "1", "2", "3", "4"].map(Label::from).to_vec();
        let rect = ui.max_rect();
        let out = ToneZones::new("Tone zones", &zones, &texts).show_at(ui, rect);
        seen.borrow_mut().extend(out.event);
    });
    fixture.settle();
    (fixture, edits)
}

#[trial]
fn tone_zone_drag_is_one_gesture() {
    let (mut fixture, edits) = zone_rig();
    let zone = fixture.by_label("Shadows").expect("a slider per zone");
    let by = zone.rect.width() * 0.5;
    fixture.drag(&zone, vec2(by, 0.0)).settle();
    let events = edits.borrow();
    let begins = events.iter().filter(|event| matches!(event, EditEvent::Begin { .. } | EditEvent::Set { .. })).count();
    assert_eq!(begins, 1, "one gesture per drag: {events:?}");
    assert!(events.iter().filter_map(|event| event.value()).all(|edit| edit.key == "shadows"), "the press locks the zone: {events:?}");
}

#[trial]
fn tone_zone_keys_step_the_focused_zone() {
    let (mut fixture, edits) = zone_rig();
    let zones = fixture.by_label("Tone zones").expect("the zones are shown");
    fixture.click(&zones).settle();
    edits.borrow_mut().clear();
    fixture.key(Key::ArrowRight, Modifiers::NONE).settle();
    fixture.key(Key::ArrowLeft, Modifiers::NONE).settle();
    fixture.key(Key::ArrowRight, Modifiers::SHIFT).settle();
    let steps: Vec<_> = edits.borrow().iter().filter_map(|event| event.value().map(|edit| (edit.key, edit.change))).collect();
    assert_eq!(steps, [("exposure", ZoneChange::Step(1)), ("exposure", ZoneChange::Step(-1)), ("exposure", ZoneChange::Step(10))]);
}
