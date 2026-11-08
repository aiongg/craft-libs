//! `ColorPicker`, its hex pair field, `ColorReadout` and `ColorChoice`: the edit rules the picker owns.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::Codec as _;
use craft_color::ui::{
    ChosenColor, ColorChoice, ColorPicker, ColorPickerLayout, ColorReadout, ColorWarning, NamedColor, PairCodec, PickedColor,
};
use craft_theme::{Channel, ColorModel, DefaultOn, HsbModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, Rgba, Srgba};
use craft_ui::contract::Bind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};

/// A naive CMYK in percent, a host's model.
struct NaiveCmyk;

const CMYK: [Channel; 4] = [
    Channel { name: "Cyan", qualified: "CMYK cyan", label: "C", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Magenta", qualified: "CMYK magenta", label: "M", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Yellow", qualified: "CMYK yellow", label: "Y", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Black", qualified: "CMYK black", label: "K", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
];

impl ColorModel for NaiveCmyk {
    fn id(&self) -> &'static str {
        "test-cmyk"
    }

    fn name(&self) -> &'static str {
        "CMYK"
    }

    fn channels(&self) -> &[Channel] {
        &CMYK
    }

    fn from_srgb(&self, c: Srgba) -> ModelValues {
        let k = 1.0 - c.r.max(c.g).max(c.b);
        let ink = |v: f32| if k >= 1.0 { 0.0 } else { 100.0 * (1.0 - v - k) / (1.0 - k) };
        ModelValues::new("test-cmyk", &[ink(c.r), ink(c.g), ink(c.b), 100.0 * k])
    }

    fn to_srgb(&self, v: &[f32; MAX_CHANNELS]) -> Srgba {
        let k = 1.0 - v[3] / 100.0;
        Srgba::rgb((1.0 - v[0] / 100.0) * k, (1.0 - v[1] / 100.0) * k, (1.0 - v[2] / 100.0) * k)
    }

    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: true, compact: false }
    }
}

static CMYK_MODEL: NaiveCmyk = NaiveCmyk;

/// What the host holds and sees.
struct Host {
    value: PickedColor,
    web: bool,
    sampled: Option<Srgba>,
    reported: usize,
}

/// How a fixture's picker is set up.
#[derive(Clone, Copy)]
struct Setup {
    layout: ColorPickerLayout,
    cmyk: bool,
    alpha: bool,
    focus_on_open: bool,
    compare: Option<Srgba>,
    warning: Option<Srgba>,
}

const COMPACT: Setup = Setup { layout: ColorPickerLayout::Compact, cmyk: false, alpha: false, focus_on_open: true, compare: None, warning: None };
const FULL: Setup = Setup { layout: ColorPickerLayout::Full, ..COMPACT };

fn picker(setup: Setup, start: PickedColor) -> (Fixture, Rc<RefCell<Host>>) {
    let host = Rc::new(RefCell::new(Host { value: start, web: false, sampled: None, reported: 0 }));
    let shown = host.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let mut value = host.value;
        let mut web = host.web;
        let sampled = host.sampled.take();
        let models: Vec<&dyn ColorModel> = if setup.cmyk {
            vec![&HsbModel, &RgbModel, &CMYK_MODEL, &OklchModel]
        } else {
            vec![&HsbModel, &RgbModel, &OklchModel]
        };
        let mut picker = ColorPicker::new("Foreground color", Bind::bound(&mut value))
            .layout(setup.layout)
            .alpha(setup.alpha)
            .focus_on_open(setup.focus_on_open)
            .models(models)
            .web_colors(Bind::bound(&mut web));
        if let Some(sampled) = sampled {
            picker = picker.sampled(sampled);
        }
        if let Some(current) = setup.compare {
            picker = picker.compare(current);
        }
        if let Some(nearest) = setup.warning {
            picker = picker.warnings([ColorWarning::new("Out of gamut for printing", nearest)]);
        }
        let out = picker.show(ui);
        host.reported += usize::from(out.edit.event.is_some());
        host.value = value;
        host.web = web;
    });
    fixture.room(vec2(720.0, 640.0));
    fixture.settle();
    (fixture, host)
}

fn type_into(fixture: &mut Fixture, label: &str, text: &str) {
    let field = fixture.by_label(label).unwrap_or_else(|| panic!("the picker shows a {label} field"));
    fixture.click(&field);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
}

fn rgb8(color: Srgba) -> [u8; 3] {
    let rgba = color.to_rgba();
    [rgba.0[0], rgba.0[1], rgba.0[2]]
}

/// A hex colour typed into the hex field is the picker's colour, with no model values.
#[trial]
fn typed_hex_sets_the_colour_without_model_values() {
    let (mut fixture, host) = picker(COMPACT, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    type_into(&mut fixture, "Hex color", "336699");
    let value = host.borrow().value;
    assert_eq!(rgb8(value.color), [0x33, 0x66, 0x99]);
    assert_eq!(value.model, None, "a colour set as such keeps no model values");
}

/// A value typed into a host model's field stays as typed in `PickedColor::model`, and the other channels keep the
/// colour's.
#[trial]
fn a_typed_channel_keeps_the_values_of_its_model() {
    let (mut fixture, host) = picker(FULL, PickedColor::cmyk(Srgba::rgb(0.5, 0.5, 0.5), [0.0, 0.0, 0.0, 50.0]));
    type_into(&mut fixture, "Magenta", "40");
    let value = host.borrow().value;
    let model = value.model.expect("the edited model's values go out with the colour");
    assert_eq!(model.model, "test-cmyk");
    assert_eq!(model.values[1], 40.0);
    assert_eq!(model.values[3], 50.0, "the black ink stays as it was");
}

/// A colour the host gives with model values shows and edits those values, not a conversion of the sRGB.
#[trial]
fn host_model_values_survive_in_the_grid() {
    let (mut fixture, host) = picker(FULL, PickedColor::cmyk(Srgba::rgb(0.5, 0.5, 0.5), [0.0, 0.0, 0.0, 50.0]));
    type_into(&mut fixture, "Cyan", "10");
    let model = host.borrow().value.model.expect("model values");
    assert_eq!([model.values[0], model.values[3]], [10.0, 50.0]);
}

/// A sample from the host replaces the colour but keeps the picker's alpha.
#[trial]
fn a_sample_keeps_the_alpha() {
    let (mut fixture, host) = picker(Setup { alpha: true, ..COMPACT }, PickedColor::rgb(Srgba::new(0.2, 0.5, 0.8, 0.5)));
    host.borrow_mut().sampled = Some(Srgba::rgb(1.0, 0.0, 0.0));
    fixture.settle();
    let value = host.borrow().value;
    assert_eq!(rgb8(value.color), [255, 0, 0]);
    assert!((value.color.a - 0.5).abs() < 0.01, "the sample takes the picker's alpha");
}

/// A sample that arrives while a hex run is open is applied, not lost, and the run's text goes.
#[trial]
fn a_sample_during_a_typing_run_applies() {
    let (mut fixture, host) = picker(COMPACT, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    let field = fixture.by_label("Hex color").expect("hex field");
    fixture.click(&field);
    fixture.type_text("12");
    host.borrow_mut().sampled = Some(Srgba::rgb(0.0, 1.0, 0.0));
    fixture.settle();
    assert_eq!(rgb8(host.borrow().value.color), [0, 255, 0]);
}

/// The compare swatch's current half and a warning chip set their colours.
#[trial]
fn compare_and_warnings_set_their_colours() {
    let current = Srgba::rgb(0.9, 0.2, 0.1);
    let nearest = Srgba::rgb(0.1, 0.9, 0.1);
    let setup = Setup { compare: Some(current), warning: Some(nearest), ..FULL };
    let (mut fixture, host) = picker(setup, PickedColor::rgb(Srgba::rgb(1.0, 0.0, 1.0)));
    let restore = fixture.by_label("Restore current color").expect("the compare swatch's current half");
    fixture.click(&restore);
    fixture.settle();
    assert_eq!(rgb8(host.borrow().value.color), rgb8(current));
    let chip = fixture.by_label("Out of gamut for printing").expect("the warning chip");
    fixture.click(&chip);
    fixture.settle();
    assert_eq!(rgb8(host.borrow().value.color), rgb8(nearest));
}

/// Two models with a channel of one name show the qualified names; the plain name goes.
#[trial]
fn channels_whose_names_clash_are_qualified() {
    let (fixture, _) = picker(FULL, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    assert!(fixture.by_label("OKLCH hue").is_some(), "OKLCH's hue is qualified beside HSB's");
    assert!(fixture.by_label("HSB hue").is_some(), "HSB's hue is qualified beside OKLCH's");
    assert!(fixture.by_label("Hue").is_none(), "no field is named by the clashing plain name");
}

/// The Full layout puts HSB and RGB in the first column, the host's models in the second and OKLCH in the third.
#[trial]
fn the_full_grid_puts_the_models_in_columns() {
    let (fixture, _) = picker(Setup { cmyk: true, ..FULL }, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    let x = |label: &str| fixture.by_label(label).unwrap_or_else(|| panic!("{label} shown")).rect.left();
    assert_eq!(x("HSB hue"), x("Red"), "HSB and RGB share the first column");
    assert!(x("Cyan") > x("Red"), "the host's model is in the next column");
    assert!(x("OKLCH hue") > x("Cyan"), "OKLCH is in the last column");
}

/// Only Web Colors snaps the colour when it turns on, and every colour after it.
#[trial]
fn only_web_colors_snaps_when_it_turns_on() {
    let (mut fixture, host) = picker(FULL, PickedColor::rgb(Srgba::rgb(0.23, 0.47, 0.81)));
    let check = fixture.by_label("Only Web Colors").expect("the check box");
    fixture.click(&check);
    fixture.settle();
    assert!(host.borrow().web, "the host's flag is on");
    for channel in rgb8(host.borrow().value.color) {
        assert_eq!(channel % 0x33, 0, "red, green and blue are on multiples of 0x33");
    }
    type_into(&mut fixture, "Hex color", "123456");
    for channel in rgb8(host.borrow().value.color) {
        assert_eq!(channel % 0x33, 0, "a typed colour snaps too");
    }
}

/// The Full layout opens with the hex text selected: typing replaces it.
#[trial]
fn the_full_picker_opens_with_the_hex_text_selected() {
    let (mut fixture, host) = picker(FULL, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    fixture.type_text("00ff00");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
    assert_eq!(rgb8(host.borrow().value.color), [0, 255, 0]);
}

/// A picker that does not focus on open leaves the focus where it was.
#[trial]
fn a_picker_without_focus_on_open_leaves_the_focus() {
    let (fixture, _) = picker(Setup { focus_on_open: false, ..FULL }, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    assert!(fixture.focused().is_none(), "nothing took the focus");
}

/// The model switch changes the rows of the Compact layout.
#[trial]
fn the_model_switch_switches_the_rows() {
    let (mut fixture, _) = picker(COMPACT, PickedColor::rgb(Srgba::rgb(0.2, 0.5, 0.8)));
    assert!(fixture.by_label("Saturation").is_some() || fixture.by_label("HSB saturation").is_some(), "HSB rows show first");
    let switch = fixture.by_label("Color model").expect("the model switch");
    fixture.click(&switch);
    fixture.settle();
    let rgb = fixture.by_label("RGB").expect("the RGB row in the menu");
    fixture.click(&rgb);
    fixture.settle();
    assert!(fixture.by_label("Red").is_some(), "RGB rows show after the switch");
}

/// A byte as two hex digits: exactly two digits apply; anything else is not a value.
#[trial]
fn a_hex_pair_applies_two_digits_and_marks_anything_else_invalid() {
    let codec = PairCodec;
    assert_eq!(codec.parse("A5", &0), Ok(0xA5));
    assert_eq!(codec.parse("a5", &0), Ok(0xA5), "either case reads");
    assert_eq!(codec.format(&0x0A), "0A");
    for text in ["", "A", "A5F", "+5", "-5", "G1", "٣٣"] {
        assert!(codec.parse(text, &0).is_err(), "{text:?} is not a byte");
    }
}

/// A channel typed into the readout reports the whole colour.
#[trial]
fn a_typed_readout_channel_reports_the_whole_colour() {
    let color = Rc::new(RefCell::new(Rgba::rgb(0x33, 0x66, 0x99)));
    let shown = color.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut value = *shown.borrow();
        let _ = ColorReadout::new("Color", Bind::bound(&mut value)).show(ui);
        *shown.borrow_mut() = value;
    });
    fixture.room(vec2(480.0, 60.0));
    fixture.settle();
    type_into(&mut fixture, "Green", "200");
    assert_eq!(*color.borrow(), Rgba::rgb(0x33, 200, 0x99), "the other channels stay");
}

fn choice_fixture(start: ChosenColor, custom: Option<Srgba>) -> (Fixture, Rc<RefCell<(ChosenColor, usize)>>) {
    let state = Rc::new(RefCell::new((start, 0usize)));
    let shown = state.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut state = shown.borrow_mut();
        let named = [NamedColor::new("default", "Default", None), NamedColor::new("black", "Black", Some(Srgba::rgb(0.0, 0.0, 0.0)))];
        let out = ColorChoice::new("Pasteboard color", &state.0).named(named).custom(custom).show(ui);
        if let Some(chosen) = out.chosen {
            state.0 = chosen;
        }
        state.1 += usize::from(out.pick_color.as_ref().is_some_and(|request| request.key == ColorChoice::CUSTOM_KEY));
    });
    fixture.room(vec2(320.0, 320.0));
    fixture.settle();
    (fixture, state)
}

/// A named row chosen in the list is the choice; the custom row appears only with a custom colour.
#[trial]
fn choosing_a_row_sets_the_choice() {
    let (mut fixture, state) = choice_fixture(ChosenColor::Named("default".into()), None);
    let select = fixture.by_label("Pasteboard color").expect("the select");
    fixture.click(&select);
    fixture.settle();
    assert!(fixture.by_label("Custom").is_none(), "no custom colour, no custom row");
    let black = fixture.by_label("Black").expect("a named row");
    fixture.click(&black);
    fixture.settle();
    assert_eq!(state.borrow().0, ChosenColor::Named("black".into()));
}

/// "Select Custom Color…" asks the host for its picker and does not change the choice.
#[trial]
fn select_custom_color_requests_the_picker() {
    let (mut fixture, state) = choice_fixture(ChosenColor::Named("black".into()), Some(Srgba::rgb(0.8, 0.3, 0.1)));
    let select = fixture.by_label("Pasteboard color").expect("the select");
    fixture.click(&select);
    fixture.settle();
    assert!(fixture.by_label("Custom").is_some(), "the custom row shows with a custom colour");
    let row = fixture.by_label("Select Custom Color…").expect("the picker row");
    fixture.click(&row);
    fixture.settle();
    let (chosen, requests) = state.borrow().clone();
    assert_eq!(requests, 1, "the host gets one request");
    assert_eq!(chosen, ChosenColor::Named("black".into()), "the choice stays until the host answers");
}
