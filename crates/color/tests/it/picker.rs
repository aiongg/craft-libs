//! `ColorPicker`, its hex pair field, `ColorReadout` and `ColorChoice`: the edit rules the picker owns.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::Mixed;
use craft_color::ui::{ColorPicker, ColorPickerLayout, ColorReadout, ColorWarning, PickedColor};
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

fn picker(setup: Setup, start: PickedColor) -> (Fixture, Rc<RefCell<Host>>) {
    let host = Rc::new(RefCell::new(Host { value: start, web: false, sampled: None, reported: 0 }));
    let shown = host.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let mut value = host.value;
        let mut web = host.web;
        let sampled = host.sampled.take();
        let models: Vec<&dyn ColorModel> =
            if setup.cmyk { vec![&HsbModel, &RgbModel, &CMYK_MODEL, &OklchModel] } else { vec![&HsbModel, &RgbModel, &OklchModel] };
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
        host.reported += usize::from(!out.events.is_empty());
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

/// Two items with different G, B and alpha, shown by a readout the way a multi-selection host does.
fn mixed_readout(items: Rc<RefCell<[Rgba; 2]>>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let mut items = items.borrow_mut();
        let out = ColorReadout::new("Color", Bind::given(Mixed::<Rgba>::Mixed)).show(ui);
        for edit in &out.channel {
            if let Some(Some(byte)) = edit.event.value() {
                for item in items.iter_mut() {
                    item.0[edit.channel.index()] = *byte;
                }
            }
        }
        if let Some(color) = out.events.value() {
            items.fill(*color);
        }
    });
    fixture.room(vec2(480.0, 60.0));
    fixture.settle();
    fixture
}

/// A channel typed while the colour is mixed sets that channel of every item and keeps each item's other channels
/// and alpha; a hex value sets the whole colour of both.
#[trial]
fn a_channel_typed_into_a_mixed_readout_keeps_each_items_other_channels() {
    let items = Rc::new(RefCell::new([Rgba([10, 20, 30, 100]), Rgba([40, 50, 60, 200])]));
    let mut fixture = mixed_readout(items.clone());
    type_into(&mut fixture, "Red", "255");
    assert_eq!(*items.borrow(), [Rgba([255, 20, 30, 100]), Rgba([255, 50, 60, 200])]);
    type_into(&mut fixture, "Hex color", "336699");
    let whole = items.borrow()[0];
    assert_eq!(&whole.0[..3], &[0x33, 0x66, 0x99], "the hex sets the colour");
    assert_eq!(items.borrow()[1], whole, "and sets it for both items");
}
