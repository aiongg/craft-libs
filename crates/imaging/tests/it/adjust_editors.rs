//! The slider-based adjustment editors, driven through the public API as a host would show them: one binding per
//! editor, one gesture per slider drag, the Set of a check box, a preset or a button, and the requests the colour wells
//! file.

use craft_behavior::{EditEvent, Value};
use craft_imaging::ui::adjust::{
    AdjustContext, AdjustField, AdjustSession, AdjustSliders, AdjustTrack, AdjustWords, BlackWhite, BlackWhiteEditor, BrightnessContrast,
    BrightnessContrastEditor, ChannelMixer, ChannelMixerEditor, ColorBalance, ColorBalanceEditor, ColorBalanceTone, HueRangeValue, HueSaturation,
    HueSaturationEditor, PhotoFilter, PhotoFilterEditor, PhotoFilterPreset, SelectiveColor, SelectiveColorEditor, ThresholdEditor,
};
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Role};
use craft_ui::runtime::requests::ColorRequest;
use craft_ui::style::DataColor;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Target, trial};
use egui::Ui;
use egui::vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An editor shown over a value the host keeps, with every event it reported.
struct Rig<T> {
    fixture: Fixture,
    value: Rc<RefCell<T>>,
    events: Rc<RefCell<Vec<EditEvent<T>>>>,
}

fn rig<T: Value>(start: T, show: impl for<'b> Fn(&mut Ui, Bind<'b, T>) -> Option<EditEvent<T>> + 'static) -> Rig<T> {
    let value = Rc::new(RefCell::new(start));
    let events = Rc::new(RefCell::new(Vec::new()));
    let (shared, seen) = (value.clone(), events.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shared.borrow().clone();
        let event = show(ui, Bind::bound(&mut current));
        *shared.borrow_mut() = current;
        seen.borrow_mut().extend(event);
    });
    fixture.room(vec2(480.0, 900.0));
    fixture.settle();
    Rig { fixture, value, events }
}

impl<T: Value> Rig<T> {
    fn value(&self) -> T {
        self.value.borrow().clone()
    }

    fn target(&self, label: &str) -> Target {
        self.fixture.by_label(label).unwrap_or_else(|| panic!("a control named {label}"))
    }

    fn click(&mut self, label: &str) {
        let target = self.target(label);
        self.fixture.click(&target);
        self.fixture.idle(2);
    }

    fn slider(&self, at: usize) -> Target {
        self.fixture.by_role(Role::SLIDER).into_iter().nth(at).expect("the slider")
    }

    fn text(&self, target: &Target) -> Option<String> {
        self.fixture.tree().value_of(target.node).map(str::to_owned)
    }

    /// Whether exactly one event was reported and it was a `Set`.
    fn one_set(&self) -> bool {
        matches!(self.events.borrow().as_slice(), [EditEvent::Set { .. }])
    }
}

/// A double click at `target`: the slider's reset.
fn double_click(fixture: &mut Fixture, target: &Target) {
    let at = target.center();
    fixture.frame(vec![moved(at)]);
    for _ in 0..2 {
        fixture.frame(vec![press(at)]);
        fixture.frame(vec![release(at)]);
    }
    fixture.idle(2);
}

// ---------------------------------------------------------------------------------------------
// Brightness/Contrast, Threshold, Color Balance, Photo Filter

/// Turning Use Legacy off with Contrast at -80 reports one `Set` with Contrast -50 and legacy off.
#[trial]
fn turning_use_legacy_off_clamps_contrast_to_minus_fifty() {
    let mut rig =
        rig(BrightnessContrast::new(10.0, -80.0, true), |ui, bind| BrightnessContrastEditor::new("Brightness/Contrast", bind).show(ui).event);
    rig.click("Use Legacy");
    let want = BrightnessContrast::new(10.0, -50.0, false);
    assert_eq!(rig.value(), want);
    assert!(rig.one_set(), "{:?}", rig.events.borrow());
}

/// The Threshold Level row reports the level a step key or an AccessKit step reaches.
#[trial]
fn the_threshold_row_reports_the_level() {
    let mut rig = rig(64.0_f64, |ui, bind| ThresholdEditor::new("Threshold", bind).show(ui).event);
    let slider = rig.slider(0);
    rig.fixture.access(AccessActionKind::Increment, &slider);
    rig.fixture.idle(2);
    let want = 65.0;
    assert_eq!(rig.value(), want);
}

/// The tone picked survives a value change from outside; a host that resets shows Midtones again; a reset of a row
/// zeroes only that row of the shown tone.
#[trial]
fn the_color_balance_tone_survives_a_value_change_and_a_reset_zeroes_one_row() {
    let tone = Rc::new(Cell::new(None));
    let host_tone = tone.clone();
    let value = ColorBalance::new([10.0, 20.0, 30.0], [-5.0, -6.0, -7.0], [0.0; 3], true);
    let mut rig = rig(value, move |ui, bind| {
        let mut editor = ColorBalanceEditor::new("Color Balance", bind);
        if let Some(tone) = host_tone.get() {
            editor = editor.tone(tone);
        }
        editor.show(ui).edit.event
    });
    let first_field = |rig: &Rig<ColorBalance>| {
        let field = rig.fixture.by_role(Role::SPIN_BUTTON).into_iter().next().expect("the first row's field");
        rig.text(&field)
    };
    assert_eq!(first_field(&rig).as_deref(), Some("-5"), "Midtones at first");
    rig.click("Shadows");
    assert_eq!(first_field(&rig).as_deref(), Some("10"));

    rig.value.borrow_mut().highlights = [1.0, 2.0, 3.0];
    rig.fixture.idle(2);
    assert_eq!(first_field(&rig).as_deref(), Some("10"), "an outside change keeps the tone");

    let slider = rig.slider(0);
    double_click(&mut rig.fixture, &slider);
    let (shadows, midtones) = ([0.0, 20.0, 30.0], [-5.0, -6.0, -7.0]);
    assert_eq!(rig.value().shadows, shadows);
    assert_eq!(rig.value().midtones, midtones);

    tone.set(Some(ColorBalanceTone::Midtones));
    rig.fixture.idle(2);
    assert_eq!(first_field(&rig).as_deref(), Some("-5"), "the host shows Midtones again");
}

/// Preserve Luminosity is one `Set` with it toggled.
#[trial]
fn preserve_luminosity_reports_a_set() {
    let mut rig = rig(ColorBalance::default(), |ui, bind| ColorBalanceEditor::new("Color Balance", bind).show(ui).edit.event);
    rig.click("Preserve Luminosity");
    let want = ColorBalance::new([0.0; 3], [0.0; 3], [0.0; 3], false);
    assert_eq!(rig.value(), want);
    assert!(rig.one_set(), "{:?}", rig.events.borrow());
}

/// Picking Cooling Filter (80) sets its colour; the Color row then enables the well, and a click on it files a colour
/// request keyed `photoFilter.color`.
#[trial]
fn a_photo_filter_preset_sets_its_colour_and_the_well_asks_for_one() {
    let requests = Rc::new(RefCell::new(Vec::<ColorRequest>::new()));
    let filed = requests.clone();
    let start = PhotoFilter::new(PhotoFilterPreset::Warming85.color(), 25.0, true);
    let mut rig = rig(start, move |ui, bind| {
        let out = PhotoFilterEditor::new("Photo Filter", bind).show(ui);
        filed.borrow_mut().extend(out.pick_color);
        out.edit.event
    });
    let select = rig.fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the preset select");
    rig.fixture.click(&select);
    rig.fixture.idle(2);
    rig.click("Cooling Filter (80)");
    let cooling = PhotoFilterPreset::Cooling80.color();
    assert_eq!(rig.value(), PhotoFilter::new(cooling, start.density, start.preserve_luminosity));

    rig.click("Color");
    let well = rig.fixture.by_role(Role::COLOR_WELL).into_iter().next().expect("the colour well");
    rig.fixture.click(&well);
    rig.fixture.idle(2);
    let filed = requests.borrow();
    let [request] = filed.as_slice() else { panic!("one request: {filed:?}") };
    assert_eq!((request.key.as_str(), request.color), ("photoFilter.color", Some(cooling)));
    assert_eq!(request.title.as_ref().map(|title| title.as_str()), Some("Filter Color"));
}

// ---------------------------------------------------------------------------------------------
// Hue/Saturation

/// Colorize on: the range select is off and shows Master, Hue -90 becomes 270 and Saturation 10 becomes 25; off: Hue
/// 270 becomes -90.
#[trial]
fn colorize_forces_master_and_wraps_the_hue() {
    let mut rig = rig(HueSaturation::new(-90.0, 10.0, 0.0, false, std::array::from_fn(HueRangeValue::neutral)), |ui, bind| {
        HueSaturationEditor::new("Hue/Saturation", bind).show(ui).edit.event
    });
    rig.click("Colorize");
    let on = rig.value();
    assert!(on.colorize);
    let want = (270.0, 25.0);
    assert_eq!((on.hue, on.saturation), want);
    assert!(rig.one_set(), "{:?}", rig.events.borrow());

    let shown = Rc::new(Cell::new(None));
    let seen = shown.clone();
    let mut rig = rig_with_range(on, Some(2), seen);
    let select = rig.fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the range select");
    assert!(rig.fixture.tree().is_disabled_key(select.node), "Colorize works on the master only");
    assert_eq!(shown.get(), Some(None), "Colorize shows Master although Yellows were asked for");
    rig.click("Colorize");
    let off = rig.value();
    assert!(!off.colorize);
    let want = -90.0;
    assert_eq!(off.hue, want);
}

/// A rig of Hue/Saturation with the range the host keeps, writing the range shown to `shown`.
fn rig_with_range(start: HueSaturation, range: Option<usize>, shown: Rc<Cell<Option<Option<usize>>>>) -> Rig<HueSaturation> {
    rig(start, move |ui, bind| {
        let out = HueSaturationEditor::new("Hue/Saturation", bind).range(range).show(ui);
        shown.set(Some(out.shown_range));
        out.edit.event
    })
}

/// The Hue slider runs -180 to 180 degrees, and 0 to 360 with Colorize.
#[trial]
fn the_hue_range_follows_colorize() {
    let reached = |colorize: bool| {
        let start = HueSaturation::new(0.0, 0.0, 0.0, colorize, std::array::from_fn(HueRangeValue::neutral));
        let mut rig = rig(start, |ui, bind| HueSaturationEditor::new("Hue/Saturation", bind).show(ui).edit.event);
        let hue = rig.slider(0);
        rig.fixture.access(AccessActionKind::SetValue(300.0), &hue);
        rig.fixture.idle(2);
        rig.value().hue
    };
    let (plain, colorized) = (180.0, 300.0);
    assert_eq!(reached(false), plain);
    assert_eq!(reached(true), colorized);
}

/// Dragging a range thumb across 0 degrees keeps the four bounds increasing.
#[trial]
fn range_thumbs_wrap_through_zero() {
    // The Magentas' end thumb dragged from 345 to 20 degrees.
    let moved = HueRangeValue::move_bound(HueRangeValue::neutral(5).bounds, 3, 20.0);
    assert_eq!(moved, [255.0, 285.0, 315.0, 380.0]);
    // The Reds' start thumb dragged from 315 (-45) to 340 degrees, past the second thumb's room.
    let moved = HueRangeValue::move_bound(HueRangeValue::neutral(0).bounds, 0, 340.0);
    assert_eq!(moved, [-20.0, -15.0, 15.0, 45.0]);
    assert!(moved.windows(2).all(|pair| pair[0] <= pair[1]));
}

/// The bounds are made canonical before a thumb moves.
#[trial]
fn a_range_thumb_move_canonicalises_the_bounds_first() {
    assert_eq!(HueRangeValue::move_bound([0.0, 350.0, 100.0, 200.0], 2, 354.0), [-360.0, -10.0, -6.0, -1.0]);
    assert_eq!(HueRangeValue::move_bound([0.0, 30.0, 60.0, 90.0], 2, 65.0), [0.0, 30.0, 65.0, 90.0]);
    assert_eq!(HueRangeValue::canonical_bounds([f64::NAN, 0.0, 0.0, 0.0]), [-45.0, -15.0, 15.0, 45.0]);
    assert_eq!(HueRangeValue::move_bound([0.0, 30.0, 60.0, 90.0], 4, 10.0), [0.0, 30.0, 60.0, 90.0], "an index past 3 only canonicalises");
}

// ---------------------------------------------------------------------------------------------
// Black & White, Channel Mixer, Selective Color

/// Default sets the six default weights as one `Set` and leaves the tint alone.
#[trial]
fn black_white_default_restores_the_six_weights() {
    let tint_color = Srgba::rgb(0.2, 0.4, 0.6);
    let mut rig = rig(BlackWhite::new([0.0; 6], true, tint_color), |ui, bind| BlackWhiteEditor::new("Black & White", bind).show(ui).edit.event);
    rig.click("Default");
    assert_eq!(rig.value(), BlackWhite::new(BlackWhite::DEFAULT_WEIGHTS, true, tint_color));
    assert!(rig.one_set(), "{:?}", rig.events.borrow());
}

/// A click on the tint well with Tint on files a colour request keyed `blackWhite.tint`; with Tint off the well is off.
#[trial]
fn the_tint_well_asks_for_a_colour_only_while_tint_is_on() {
    let tint_color = Srgba::rgb(0.2, 0.4, 0.6);
    let requested = |tint: bool| {
        let requests = Rc::new(RefCell::new(Vec::<ColorRequest>::new()));
        let filed = requests.clone();
        let mut rig = rig(BlackWhite::new(BlackWhite::DEFAULT_WEIGHTS, tint, tint_color), move |ui, bind| {
            let out = BlackWhiteEditor::new("Black & White", bind).show(ui);
            filed.borrow_mut().extend(out.pick_color);
            out.edit.event
        });
        let well = rig.fixture.by_role(Role::COLOR_WELL).into_iter().next().expect("the tint well");
        let disabled = rig.fixture.tree().is_disabled_key(well.node);
        rig.fixture.click(&well);
        rig.fixture.idle(2);
        let filed = requests.borrow().clone();
        (disabled, filed)
    };
    let (disabled, filed) = requested(true);
    assert!(!disabled);
    assert_eq!(filed.iter().map(|request| (request.key.as_str(), request.color)).collect::<Vec<_>>(), [("blackWhite.tint", Some(tint_color))]);
    let (disabled, filed) = requested(false);
    assert!(disabled);
    assert!(filed.is_empty());
}

/// Monochrome seeds the Gray row and the select shows Gray; off removes the row and the select shows Red again.
#[trial]
fn monochrome_seeds_the_gray_row_and_off_removes_it() {
    let mut rig = rig(ChannelMixer::default(), |ui, bind| ChannelMixerEditor::new("Channel Mixer", bind).show(ui).event);
    let output = |rig: &Rig<ChannelMixer>| {
        let select = rig.fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the output select");
        rig.text(&select)
    };
    assert_eq!(output(&rig).as_deref(), Some("Red"));
    rig.click("Monochrome");
    assert_eq!(rig.value().gray, Some(ChannelMixer::MONOCHROME_START));
    assert_eq!(output(&rig).as_deref(), Some("Gray"));
    rig.click("Monochrome");
    assert_eq!(rig.value().gray, None);
    assert_eq!(output(&rig).as_deref(), Some("Red"));
}

/// The Total shows its sign and percent, and above 100 percent the warning says so.
#[trial]
fn the_mixer_total_warns_above_one_hundred_percent() {
    let shown = |mixer: ChannelMixer| {
        let rig = rig(mixer, |ui, bind| ChannelMixerEditor::new("Channel Mixer", bind).show(ui).event);
        rig.fixture.texts().iter().map(|run| run.text.clone()).collect::<Vec<_>>()
    };
    let over = shown(ChannelMixer::new([60.0, 60.0, 0.0, 0.0], [0.0; 4], [0.0; 4], None));
    assert!(over.iter().any(|text| text == "+120%"), "{over:?}");
    assert!(over.iter().any(|text| text == "+120%, over 100 percent"), "{over:?}");
    let at = shown(ChannelMixer::new([60.0, 40.0, 0.0, 0.0], [0.0; 4], [0.0; 4], None));
    assert!(at.iter().any(|text| text == "+100%"), "{at:?}");
    assert!(!at.iter().any(|text| text.contains("over 100")), "{at:?}");
}

/// Picking Neutrals and stepping Black changes range 7, slot 3 and nothing else.
#[trial]
fn selective_color_edits_the_shown_range_only() {
    let mut rig = rig(SelectiveColor::default(), |ui, bind| SelectiveColorEditor::new("Selective Color", bind).show(ui).edit.event);
    let select = rig.fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the colors select");
    rig.fixture.click(&select);
    rig.fixture.idle(2);
    rig.click("Neutrals");
    let black = rig.slider(3);
    rig.fixture.access(AccessActionKind::Increment, &black);
    rig.fixture.idle(2);
    let mut want = SelectiveColor::default();
    want.ranges[7][3] = 1.0;
    assert_eq!(rig.value(), want);
}

// ---------------------------------------------------------------------------------------------
// The slider table and the session

/// `AdjustField::round` keeps the places of the field's step: Exposure two, Offset four, a range 10 wide or less
/// hundredths, wider ranges whole numbers.
#[trial]
fn fields_round_to_the_places_of_their_step() {
    let words = AdjustWords::default();
    let [exposure, offset, gamma] = <[AdjustField; 3]>::try_from(AdjustField::exposure(&words)).unwrap_or_else(|_| panic!("three rows"));
    assert_eq!(exposure.round(1.23456), 1.23);
    assert_eq!(offset.round(0.123456), 0.1235);
    assert_eq!(gamma.round(2.3456), 2.35);
    assert_eq!(AdjustField::vibrance(&words)[0].round(33.6), 34.0);
}

/// Colour tracks on paint a gradient in place of the plain track; with them off the plain track is drawn.
#[trial]
fn dynamic_tracks_off_draws_plain_tracks() {
    let shapes = |context: AdjustContext| {
        let field = AdjustField::new("cyanRed", "Cyan · Red", -100.0..=100.0, 0.0)
            .track(AdjustTrack::Colors(vec![DataColor::new(egui::Color32::from_rgb(0, 0, 255)), DataColor::new(egui::Color32::from_rgb(255, 0, 0))]));
        let mut fixture = Fixture::new(move |ui| {
            let mut values = vec![0.0];
            AdjustSliders::new("Tracks", std::slice::from_ref(&field), Bind::bound(&mut values)).context(context).show(ui);
        });
        fixture.settle();
        fixture.shape_bounds().len()
    };
    assert_ne!(shapes(AdjustContext::dialog(true)), shapes(AdjustContext::dialog(false)));
}

/// A drag of a slider is one gesture of the whole table: one begin, updates that carry every value, one commit.
#[trial]
fn a_slider_drag_is_one_gesture_of_the_table() {
    let words = AdjustWords::default();
    let fields = AdjustField::exposure(&words);
    let mut rig = rig(vec![0.0, 0.1, 1.0], move |ui, bind| AdjustSliders::new("Exposure", &fields, bind).show(ui).event);
    let slider = rig.slider(0);
    rig.fixture.drag(&slider, vec2(40.0, 0.0));
    rig.fixture.idle(2);
    let events = rig.events.borrow();
    let begins = events.iter().filter(|event| matches!(event, EditEvent::Begin { .. })).count();
    assert_eq!(begins, 1, "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { .. })), "{events:?}");
    let value = rig.value.borrow();
    let before = 0.0;
    assert!(value[0] > before, "the drag moved Exposure: {value:?}");
    let others = (0.1, 1.0);
    assert_eq!((value[1], value[2]), others, "the other rows are as they were");
}

/// Tracked, edited, Reset: the start settings; Reset again: the defaults; a pass without tracking or another target
/// takes the committed settings anew.
#[trial]
fn reset_returns_to_the_values_found_when_edited_else_defaults() {
    let mut session = AdjustSession::new();
    session.track(7, 1, &10);
    session.track(7, 2, &20);
    assert_eq!(session.start(), None, "nothing is offered before an edit");
    session.edited();
    session.track(7, 3, &30);
    assert_eq!(session.start(), Some(&10), "following passes keep the settings found first");
    assert_eq!(session.reset(|| 0), 10, "edited: the start settings");
    assert!(!session.is_edited());
    assert_eq!(session.reset(|| 0), 0, "a second Reset gives the defaults");

    session.edited();
    session.track(7, 9, &90);
    assert!(!session.is_edited());
    session.edited();
    assert_eq!(session.start(), Some(&90));

    session.track(8, 10, &5);
    assert!(!session.is_edited());
    session.edited();
    assert_eq!(session.start(), Some(&5));
}
