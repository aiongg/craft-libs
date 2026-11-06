//! The "Control:" row of a brush dynamic ([`DynamicControlRow`]).

use craft_ui::controls::{IconButton, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::labels::BrushSettingsLabels;
use super::style::BrushSettingsStyle;

/// Fade's step count: whole steps, 1 to 9999.
const FADE_STEPS: (u32, u32) = (1, 9999);

/// One entry of a dynamic's Control menu: the app's value (`C`, its control enum) and its name,
/// and what the row shows beside it when it is current.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicControl<C> {
    /// The app's value.
    pub value: C,
    /// The name in the menu.
    pub label: String,
    /// This is Fade: the row shows the Fade steps field while it is current.
    pub fade: bool,
    /// This control has a 0..1 input its response curve shapes (Fade, Pen Pressure, Pen Tilt,
    /// Stylus Wheel, Rotation, outside Angle Jitter): the row shows the curve toggle while it is
    /// current.
    pub curve: bool,
}

impl<C> DynamicControl<C> {
    /// An entry with neither the Fade field nor the curve toggle (Off, the direction controls).
    pub fn new(value: C, label: impl Into<String>) -> Self {
        Self { value, label: label.into(), fade: false, curve: false }
    }

    /// Marks the entry as Fade.
    pub fn fade(mut self) -> Self {
        self.fade = true;
        self
    }

    /// Marks the entry as one a response curve shapes.
    pub fn curve(mut self) -> Self {
        self.curve = true;
        self
    }
}

/// Photoshop's `Control: [Off ▾] [25] [~]` row under a jitter slider: the control
/// [`Select`](craft_ui::controls::Select), the Fade steps field (1 to 9999, only while the current
/// control is [`fade`](DynamicControl::fade)) and the response curve toggle (only while it takes a
/// [`curve`](DynamicControl::curve); lit while the curve is open or not linear). The app lists the
/// entries it offers (the direction controls only for Angle Jitter) and shows the
/// [`CurveEditor`](craft_ui::composites::CurveEditor) in its own slot under the row when
/// [`DynamicControlRowResponse::curve_toggled`] opens it. The Jitter and Minimum rows around it are
/// `SliderRow`s.
///
/// - **Look.** The label in `text.muted`, the select as wide as the style's `control_select`, the
///   field the number field's default width, the toggle an icon button; RTL mirrors.
/// - **Keys.** Three Tab stops (select, field, toggle).
/// - **AccessKit.** The select named "Control:", the field a `SpinButton` "Fade steps", the toggle
///   a `Button` "Response curve" with `toggled` while lit.
#[must_use = "show the row with `show`"]
pub struct DynamicControlRow<'a, C> {
    id_salt: IdSalt,
    controls: &'a [DynamicControl<C>],
    current: &'a C,
    fade_steps: u32,
    curve_open: bool,
    curve_shaped: bool,
    labels: Option<BrushSettingsLabels>,
}

impl<'a, C: Clone + PartialEq + Send + Sync + 'static> DynamicControlRow<'a, C> {
    /// The row for the `current` control among `controls`, in menu order.
    pub fn new(id_salt: impl AsIdSalt, controls: &'a [DynamicControl<C>], current: &'a C) -> Self {
        Self { id_salt: IdSalt::new(id_salt), controls, current, fade_steps: 25, curve_open: false, curve_shaped: false, labels: None }
    }

    /// The Fade step count shown while Fade is current; 25 until set.
    pub fn fade_steps(mut self, steps: u32) -> Self {
        self.fade_steps = steps;
        self
    }

    /// Whether the app shows the curve editor (`open`) and whether the curve is not linear
    /// (`shaped`): either lights the toggle.
    pub fn curve(mut self, open: bool, shaped: bool) -> Self {
        self.curve_open = open;
        self.curve_shaped = shaped;
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the row.
    pub fn show(self, ui: &mut Ui) -> DynamicControlRowResponse<C> {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:85-110 `control_row` (select 130 wide, fade field 1..=9999 56 wide, curve toggle 22 lit when open or not linear)
        // pc: crates/ui-egui/src/panels/brush_sections.rs:73 `takes_curve` (Fade, Pen Pressure, Pen Tilt, Stylus Wheel, Rotation; never for Angle Jitter)
        let labels = self.labels.unwrap_or_else(|| BrushSettingsLabels::of(ui));
        let style = BrushSettingsStyle::of(&Tokens::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let entry = self.controls.iter().find(|entry| entry.value == *self.current);
        let (fade, curve) = entry.map_or((false, false), |entry| (entry.fade, entry.curve));
        let mut control = self.current.clone();
        let mut fade_steps = None;
        let mut curve_toggled = false;
        let response = ui
            .horizontal(|ui| {
                ui.add(Text::new(labels.control.as_str()).content(Content::Muted));
                let mut select = Select::new((&self.id_salt, "control"), labels.control.as_str(), &mut control).width(style.control_select);
                for entry in self.controls {
                    select = select.option(entry.value.clone(), entry.label.as_str());
                }
                let _ = select.show(ui);
                if fade {
                    let shown = f64::from(self.fade_steps.clamp(FADE_STEPS.0, FADE_STEPS.1));
                    fade_steps = NumberField::new((&self.id_salt, "fade"), labels.fade_steps.as_str(), shown, &numbers)
                        .range(f64::from(FADE_STEPS.0)..=f64::from(FADE_STEPS.1))
                        .step(1.0)
                        .decimals(0..=0)
                        .snap_typed(true)
                        .show(ui)
                        .event;
                }
                if curve {
                    let lit = self.curve_open || self.curve_shaped;
                    let toggle = IconButton::new(Icon::Spline, labels.response_curve.as_str())
                        .id_salt((&self.id_salt, "curve"))
                        .selected(lit)
                        .show_tooltip(true);
                    curve_toggled = ui.add(toggle).clicked();
                }
            })
            .response;
        DynamicControlRowResponse { response, control: (control != *self.current).then_some(control), fade_steps, curve_toggled }
    }
}

/// What [`DynamicControlRow::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct DynamicControlRowResponse<C> {
    /// The row.
    pub response: Response,
    /// The control picked this pass.
    pub control: Option<C>,
    /// The Fade steps field: whole steps, 1 to 9999.
    pub fade_steps: Option<EditEvent<f64>>,
    /// The curve toggle was pressed: the app opens or closes its curve editor.
    pub curve_toggled: bool,
}
