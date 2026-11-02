//! Stories of the marquee style cells (task KS1): not registered. Registering them (task RG2)
//! adds `stories()` to the canvas stories, `COMPONENTS` to the component list and
//! [`ConstraintFieldsStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The cells sit in an `OptionsBar` over a fake 2000 × 1500 px document at 300 ppi, so Fixed Size
//! shows its units menu with real conversions. The host keeps the constraint: every event's value
//! is stored (`Begin` and `Update` as typed, `Commit` and `Set` as the value). Every string goes
//! through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/constraint-fields/constraint-fields` | Normal (fields greyed), Fixed Ratio 1 : 1 after the switch, Fixed Size 64 × 64 px, swap, a picked unit (in), the whole group disabled (single row marquee) |

use egui::Ui;

use super::Constraint;
use super::fields::{ConstraintFields, ConstraintFieldsLabels};
use crate::localized::units::{LengthUnit, UnitContext};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::patterns::OptionsBar;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, Surface, harness};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ConstraintFields"];

crate::words! {
    /// The captions of the constraint fields story, English by default.
    #[non_exhaustive]
    pub(crate) struct ConstraintFieldsStoryLabels {
        value: "constraintFieldsStory.value" = "{style}: {width} × {height}";
        pixels: "constraintFieldsStory.pixels" = "{value} px";
        unit: "constraintFieldsStory.unit" = "Unit: {unit}";
        none: "constraintFieldsStory.none" = "–";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/constraint-fields/constraint-fields",
            Level::Patterns,
            "ConstraintFields",
            "Constraint fields",
            FieldsArgs { enabled: true, units_menu: true },
            show_fields
        )
        .on(Surface::Chrome),
    ]
}

/// The fake document: 2000 × 1500 pixels at 300 pixels per inch.
const DOCUMENT_SIZE: [f64; 2] = [2000.0, 1500.0];
const DOCUMENT_PPI: f64 = 300.0;

#[derive(StoryArgs, Debug, PartialEq)]
struct FieldsArgs {
    /// Off: the whole group is disabled, as in a single row marquee.
    enabled: bool,
    #[arg(name = "units menu")]
    units_menu: bool,
}

#[derive(Clone, Copy, Debug)]
struct FieldsState {
    constraint: Constraint,
    unit: LengthUnit,
}

/// The cells of the Marquee options bar. Pick a style, type, swap or pick a unit from the menu of
/// a Fixed Size cell; the line under the bar shows the constraint in pixels.
fn show_fields(ui: &mut Ui, args: &mut FieldsArgs) {
    let shared = harness::shared(ui, "constraint-fields", || FieldsState { constraint: Constraint::Free, unit: LengthUnit::Pixels });
    let mut st = harness::lock(&shared);
    let labels = ConstraintFieldsLabels::of(ui);
    let own = ConstraintFieldsStoryLabels::of(ui);
    let width = UnitContext { pixels_per_inch: DOCUMENT_PPI, extent: Some(DOCUMENT_SIZE[0]), ..UnitContext::default() };
    let height = UnitContext { extent: Some(DOCUMENT_SIZE[1]), ..width };
    let mut out = None;
    OptionsBar::new("constraint-fields-story-bar").show(ui, |bar| {
        bar.body(|ui| {
            let mut fields = ConstraintFields::new("constraint-fields", &st.constraint).enabled(args.enabled);
            if args.units_menu {
                fields = fields.size_units(st.unit, &LengthUnit::ALL, width, height);
            }
            out = Some(fields.show(ui));
        });
    });
    if let Some(out) = out {
        if let Some(value) = out.event.as_ref().and_then(|e| e.value()) {
            st.constraint = *value;
        }
        if let Some(unit) = out.unit_picked {
            st.unit = unit;
        }
    }
    let numbers = LocaleNumbers::of(ui);
    let number = |v: f64| numbers.format(v, 0..=3);
    let pixels = |v: f64| craft_i18n::fmt(&own.pixels, &[("value", &number(v))]);
    let (style, w, h) = match st.constraint {
        Constraint::Free => (labels.normal.as_str(), None, None),
        Constraint::Ratio(w, h) => (labels.fixed_ratio.as_str(), Some(number(w)), Some(number(h))),
        Constraint::Size(w, h) => (labels.fixed_size.as_str(), Some(pixels(w)), Some(pixels(h))),
    };
    let line =
        craft_i18n::fmt(&own.value, &[("style", style), ("width", w.as_deref().unwrap_or(&own.none)), ("height", h.as_deref().unwrap_or(&own.none))]);
    ui.add(Text::new(line).content(Content::Muted));
    ui.add(Text::new(craft_i18n::fmt(&own.unit, &[("unit", st.unit.suffix())])).content(Content::Muted));
}
