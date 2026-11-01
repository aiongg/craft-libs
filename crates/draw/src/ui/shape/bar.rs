//! [`ShapeOptionsBar`]: the Shape tools' options bar cells (task SH-T6).

use craft_color::ui::{Paint, PaintChip};
use craft_theme::{ControlSize, Srgba};
use egui::{AsIdSalt, IdSalt, Ui, UiBuilder};

use super::panel::{ChipDecor, Field, paint_chip, paint_events, stroke_width_field, style_button, width_events};
use super::{ShapeAppearance, ShapeChange, ShapeLabels, ShapePaint, ShapeToolSettings};
use crate::ui::shape_drag::ShapeKind;
use craft_ui::controls::{Checkbox, TooltipExt as _};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::theme::{Tokens, TokensOf as _};

/// The kind's own cell: its field, width class and the change a value gives.
type KindCell<'a> = (Field<'a>, CellWidth, fn(f64) -> ShapeChange);

/// What [`ShapeOptionsBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app routes each change to the selected layer and to the tool settings"]
pub struct ShapeOptionsOutput {
    /// Each changed cell this pass, as [`ShapePanelOutput::events`](super::ShapePanelOutput::events).
    /// With a shape layer selected the app applies it to the layer and to [`ShapeToolSettings`];
    /// without, to the settings only (PhotoCraft #3347).
    pub events: Vec<EditEvent<ShapeChange>>,
}

/// The cells a Shape tool adds to the options bar, in PhotoCraft's order: "Fill" check box and
/// chip, "Stroke" chip, stroke width (0–288 px), the Stroke style button; then the kind's cell:
/// Rectangle and Rounded Rectangle "Radius" (0–10,000), Polygon and Star "Sides" (3–100), Line
/// "Weight" (1–1,000). Custom Shape's picker is the app's (S6a-presets' `PresetPicker`), added
/// after these cells.
///
/// - The values are the selected shape layer's when one is selected, else the tool settings'
///   (the app passes the right [`ShapeAppearance`]).
/// - Stroke width 0 is no stroke; turning the stroke on at 0 gives
///   [`ShapeChange::ENABLED_STROKE_WIDTH`].
/// - Keyboard: every cell is a Tab stop in the bar's order; captions scrub their fields.
pub struct ShapeOptionsBar<'a> {
    id_salt: IdSalt,
    kind: ShapeKind,
    settings: &'a ShapeToolSettings,
    appearance: &'a ShapeAppearance,
    stroke_open: Option<&'a mut bool>,
    chips: Option<Box<ChipDecor<'a>>>,
    fallbacks: (Srgba, Srgba),
    labels: Option<ShapeLabels>,
}

impl<'a> ShapeOptionsBar<'a> {
    /// The cells for `kind` with the `settings` and `appearance` shown.
    pub fn new(id_salt: impl AsIdSalt, kind: ShapeKind, settings: &'a ShapeToolSettings, appearance: &'a ShapeAppearance) -> Self {
        ShapeOptionsBar {
            id_salt: IdSalt::new(id_salt),
            kind,
            settings,
            appearance,
            stroke_open: None,
            chips: None,
            fallbacks: (Srgba::BLACK, Srgba::WHITE),
            labels: None,
        }
    }

    /// The Stroke style popover's open flag (S7-pen's `StrokeButton`); without it the button is
    /// not shown.
    pub fn stroke_open(mut self, open: &'a mut bool) -> Self {
        self.stroke_open = Some(open);
        self
    }

    /// Decorates each paint chip before it shows (its Gradient, Pattern and Picture editors).
    pub fn chips(mut self, decorate: impl for<'b> FnMut(ShapePaint, PaintChip<'b>) -> PaintChip<'b> + 'a) -> Self {
        self.chips = Some(Box::new(decorate));
        self
    }

    /// The foreground and background colours a switch to Solid starts from.
    pub fn fallbacks(mut self, fill: Srgba, stroke: Srgba) -> Self {
        self.fallbacks = (fill, stroke);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ShapeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> ShapeOptionsOutput {
        // pc: crates/ui-egui/src/tools/shape_options.rs:35-191 `options_bar` (Fill :63, Stroke :69, the
        // width field :71, Radius :100, Sides :113, Weight :121; the edit target :36-52, :173-191).
        // new: `OptionsBar` cells, `PaintChip`, `NumberField`, `craft_draw::ui::stroke::StrokeButton`
        // (its `EditEvent<StrokeStyle>` mapped to `ShapeChange::StrokeStyle`).
        let Self { id_salt, kind, settings, appearance, stroke_open, mut chips, fallbacks, labels } = self;
        let labels = labels.unwrap_or_else(|| ShapeLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let width_box = OptionsBar::style(&Tokens::of(ui), false).fields[0];
        let mut events = Vec::new();
        let builder = UiBuilder::new().id_salt(id_salt).layout(*ui.layout());
        ui.scope_builder(builder, |ui| {
            ui.add_enabled_ui(!appearance.locked, |ui| {
                let mut fill_on = !matches!(appearance.fill, Paint::None);
                let check = Checkbox::bool(labels.fill.as_str(), &mut fill_on).id_salt("fill-on").size(ControlSize::Sm).show(ui);
                events.extend(check.event.map(|e| e.map(ShapeChange::FillOn)));
                let mut edit =
                    paint_chip(ui, chips.as_deref_mut(), ShapePaint::Fill, "fill", &labels.fill_color, &appearance.fill, fallbacks.0, &numbers);
                edit.response = edit.response.tooltip(labels.set_fill_type.as_str());
                paint_events(&mut events, ShapePaint::Fill, edit.event, appearance.stroke_width);

                let edit = OptionsBar::cell(ui, &labels.stroke, CellWidth::Content, |ui, _| {
                    let mut edit = paint_chip(
                        ui,
                        chips.as_deref_mut(),
                        ShapePaint::Stroke,
                        "stroke",
                        &labels.stroke_color,
                        &appearance.stroke,
                        fallbacks.1,
                        &numbers,
                    );
                    edit.response = edit.response.tooltip(labels.set_stroke_type.as_str());
                    edit
                });
                paint_events(&mut events, ShapePaint::Stroke, edit.event, appearance.stroke_width);

                let field = stroke_width_field("stroke-width", &labels, appearance.stroke_width, &numbers);
                let edit = field.size(ControlSize::Sm).width(width_box).show(ui);
                width_events(&mut events, edit.event, appearance.stroke_width);

                if let Some(open) = stroke_open {
                    let corners = kind != ShapeKind::Ellipse;
                    style_button(ui, &mut events, "stroke-style", appearance, corners, &numbers, open);
                }

                let cell: Option<KindCell<'_>> = match kind {
                    ShapeKind::Rectangle | ShapeKind::RoundedRectangle => Some((
                        Field::pixels("radius", &labels.radius, settings.corner_radius, 0.0..=10_000.0),
                        CellWidth::Md,
                        ShapeChange::CornerRadius,
                    )),
                    ShapeKind::Polygon | ShapeKind::Star => {
                        let sides = Field {
                            places: 0..=0,
                            unit: Default::default(),
                            ..Field::pixels("sides", &labels.sides, f64::from(settings.sides), 3.0..=100.0)
                        };
                        Some((sides, CellWidth::Sm, |n| ShapeChange::Sides(n.round().clamp(3.0, 100.0) as u32)))
                    }
                    ShapeKind::Line => {
                        Some((Field::pixels("weight", &labels.weight, settings.line_weight, 1.0..=1_000.0), CellWidth::Md, ShapeChange::Weight))
                    }
                    ShapeKind::Ellipse | ShapeKind::Triangle | ShapeKind::Custom => None,
                };
                if let Some((field, width, change)) = cell {
                    OptionsBar::divider(ui);
                    let edit = OptionsBar::number_cell(ui, field.caption, width, field.number(&numbers));
                    events.extend(edit.event.map(|e| e.map(change)));
                }
            });
        });
        ShapeOptionsOutput { events }
    }
}
