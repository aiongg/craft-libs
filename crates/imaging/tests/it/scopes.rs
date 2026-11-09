//! The scope panel and its parts: rows per view, the plot's levels and range, the three refresh paths, the clipping
//! buttons, the tone zones and the commands.

use craft_behavior::EditEvent;
use craft_imaging::ui::histogram::ChannelColor;
use craft_imaging::ui::{
    ClippingFrame, HistogramSet, ScopeChannel, ScopeFrame, ScopePanel, ScopePanelState, ScopeSettings, ScopeSource, ScopeView, ScopeWords,
    ScopeZoneEdit, ToneZone, ToneZones, VectorscopeFrame, ZoneChange,
};
use craft_ui::contract::Show as _;
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, pos2, vec2};
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
