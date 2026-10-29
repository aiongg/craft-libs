//! Stories of the Full colour picker: the layout with a host's CMYK model, and a dialog inside an
//! [`edit::scope`].

use craft_theme::{Channel, ColorModel, DefaultOn, HsbModel, LabModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, Srgba};
use egui::Ui;

use super::{ColorPicker, ColorPickerLayout, ColorWarning, web_color};
use crate::behavior::FieldTab;
use crate::controls::{Button, ButtonVariant, Swatch};
use crate::edit::{self, EditEvent, OuterGesture};
use crate::localized::LocaleNumbers;
use crate::primitives::{Content, Gap, Row, Stack, Text};
use crate::story::{Args, Level, Story};

pub(super) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "composites/color/full",
            level: Level::Composites,
            component: "ColorPicker",
            name: "Full",
            args: || {
                Args::new()
                    .with_bool("CMYK (host model)", true)
                    .with_bool("OKLCH", false)
                    .with_bool("compare", true)
                    .with_bool("warnings", true)
                    .with_bool("eyedropper", true)
                    .with_bool("alpha", false)
                    .with_bool("disabled", false)
            },
            show: show_full,
        },
        Story {
            id: "composites/color/dialog",
            level: Level::Composites,
            component: "ColorPicker",
            name: "Dialog (edit scope)",
            args: || Args::new().with_bool("CMYK (host model)", true),
            show: show_dialog,
        },
    ]
}

/// A naive CMYK in percent, without a working space: a host's model for the stories. A real host
/// converts through its CMYK profile.
struct NaiveCmyk;

const CMYK: [Channel; 4] = [
    Channel { name: "Cyan", qualified: "CMYK cyan", label: "C", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Magenta", qualified: "CMYK magenta", label: "M", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Yellow", qualified: "CMYK yellow", label: "Y", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Black", qualified: "CMYK black", label: "K", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
];

impl ColorModel for NaiveCmyk {
    fn id(&self) -> &'static str {
        "story-cmyk"
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
        ModelValues::new("story-cmyk", &[ink(c.r), ink(c.g), ink(c.b), 100.0 * k])
    }
    fn to_srgb(&self, v: &[f32; MAX_CHANNELS]) -> Srgba {
        let k = 1.0 - v[3] / 100.0;
        Srgba::rgb((1.0 - v[0] / 100.0) * k, (1.0 - v[1] / 100.0) * k, (1.0 - v[2] / 100.0) * k)
    }
    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: true, compact: false }
    }
}

/// The models of a story picker: the Full defaults, with the host's CMYK and OKLCH when asked.
fn models(cmyk: bool, oklch: bool) -> Vec<&'static dyn ColorModel> {
    let mut models: Vec<&'static dyn ColorModel> = vec![&HsbModel, &RgbModel, &LabModel];
    if cmyk {
        models.push(&NaiveCmyk);
    }
    if oklch {
        models.push(&OklchModel);
    }
    models
}

/// The story's warnings: not a web colour, and a stand-in gamut check that takes saturation over
/// 85 % as out of gamut.
fn warnings(color: Srgba) -> Vec<ColorWarning> {
    let mut out = Vec::new();
    let mut hsb = color.to_hsb();
    if hsb.s > 0.85 {
        hsb.s = 0.85;
        out.push(ColorWarning::new("Out of gamut for printing", Srgba::from_hsb(hsb).with_alpha(color.a)));
    }
    let web = web_color(color);
    if !web.same_rgb8(color) {
        out.push(ColorWarning::new("Not a web colour", web));
    }
    out
}

/// A Full picker editing one colour as a panel does, with the colour it started from in the
/// compare swatch; the eyedropper "samples" orange at once.
fn show_full(ui: &mut Ui, args: &mut Args) {
    let numbers = LocaleNumbers::from_tag("en");
    let start = Srgba::rgb(0.2, 0.55, 0.8);
    crate::controls::slider::with_values(ui, "color-full", [(start, start, String::new(), false)], |ui, state| {
        let (color, before, last, web) = &mut state[0];
        let models = models(args.bool("CMYK (host model)"), args.bool("OKLCH"));
        let mut picker = ColorPicker::new("color-full", "Foreground colour", *color, &numbers)
            .layout(ColorPickerLayout::Full)
            .models(models)
            .alpha(args.bool("alpha"))
            .web_colors(web)
            .enabled(!args.bool("disabled"));
        if args.bool("compare") {
            picker = picker.compare(start);
        }
        if args.bool("warnings") {
            for warning in warnings(*color) {
                picker = picker.warning(warning);
            }
        }
        if args.bool("eyedropper") {
            picker = picker.eyedropper("Pick screen color");
        }
        let picked = picker.show(ui);
        if picked.eyedropper_requested {
            ColorPicker::sampled(ui.ctx(), picked.edit.response.id, Srgba::rgb(1.0, 0.6, 0.0));
        }
        if let Some(event) = &picked.edit.event {
            match event {
                EditEvent::Begin { .. } => *before = *color,
                EditEvent::Cancel { .. } => *color = *before,
                _ => {}
            }
            if let Some(p) = event.value() {
                *color = p.color;
            }
            *last = edit::describe(&event.clone().map(|p| (p.color.to_rgba().to_hex(), p.model.map(|m| m.model))));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}

/// What the dialog story keeps: the document's colour and its undo steps; while the dialog is
/// open, its colour and gesture.
#[derive(Clone, Debug)]
struct Dialog {
    applied: Srgba,
    open: Option<(Srgba, OuterGesture)>,
    steps: usize,
    updates: usize,
    last: String,
}

/// A colour opened in a Full picker dialog: inside an [`edit::scope`] the picker reports only
/// updates, and the dialog's [`OuterGesture`] makes OK one undo step and Cancel none.
fn show_dialog(ui: &mut Ui, args: &mut Args) {
    let numbers = LocaleNumbers::from_tag("en");
    let initial = Dialog { applied: Srgba::rgb(0.8, 0.3, 0.2), open: None, steps: 0, updates: 0, last: String::new() };
    crate::controls::slider::with_values(ui, "color-dialog", [initial], |ui, state| {
        let dialog = &mut state[0];
        Stack::new(Gap::Stack).show(ui, |ui| {
            Row::new(Gap::Inline).show(ui, |ui| {
                let clicked = ui.add(Swatch::new("color-dialog-swatch", "Fill colour", dialog.applied)).clicked();
                ui.add(Text::new(format!("Fill {}  Undo steps: {}", dialog.applied.to_rgba().to_hex().to_uppercase(), dialog.steps)));
                if clicked && dialog.open.is_none() {
                    dialog.open = Some((dialog.applied, OuterGesture::begin(ui.ctx())));
                    dialog.updates = 0;
                }
            });
            let Some((mut color, outer)) = dialog.open else {
                ui.add(Text::new("Click the swatch to open the dialog.").content(Content::Muted));
                return;
            };
            outer.keep(ui);
            let (mut ok, mut cancel) = (false, false);
            FieldTab::new("color-dialog").show(ui, |ui| {
                Row::new(Gap::Section).show(ui, |ui| {
                    let picked = edit::scope(ui, |ui| {
                        ColorPicker::new("color-dialog-picker", "Fill colour", color, &numbers)
                            .layout(ColorPickerLayout::Full)
                            .models(models(args.bool("CMYK (host model)"), false))
                            .compare(dialog.applied)
                            .show(ui)
                    })
                    .inner;
                    if let Some(event) = &picked.edit.event {
                        dialog.updates += usize::from(matches!(event, EditEvent::Update { .. }));
                        dialog.last = edit::describe(&event.clone().map(|p| p.color.to_rgba().to_hex()));
                        if let Some(p) = event.value() {
                            color = p.color;
                        }
                    }
                    Stack::new(Gap::Stack).show(ui, |ui| {
                        ok = ui.add(Button::new("OK").variant(ButtonVariant::Primary)).clicked();
                        cancel = ui.add(Button::new("Cancel")).clicked();
                    });
                });
            });
            ui.add(Text::new(format!("Updates: {}  Last event: {}", dialog.updates, dialog.last)).content(Content::Muted));
            dialog.open = Some((color, outer));
            if ok {
                if !color.same_rgb8(dialog.applied) {
                    dialog.applied = color;
                    dialog.steps += 1;
                }
                dialog.open = None;
            } else if cancel {
                dialog.open = None;
            }
        });
    });
}
