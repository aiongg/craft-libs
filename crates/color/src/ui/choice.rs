//! [`ColorChoice`] (F04.05): a choice among named colours with a custom colour, as Photoshop's
//! pasteboard colour (Preferences › Interface and the pasteboard's right-click menu), the Crop
//! shield colour and the Note colour.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::controls::ControlSize;
use craft_ui::edit::EditResponse;

craft_ui::words! {
    /// The choice's own rows, English by default.
    #[non_exhaustive]
    pub struct ColorChoiceLabels {
        /// The row of the custom colour last picked.
        custom: "colorChoice.custom" = "Custom";
        /// The row that opens the Color Picker.
        select_custom: "colorChoice.selectCustom" = "Select Custom Color…";
    }
}

/// One named colour of a [`ColorChoice`]: its id (what the app stores), its label (translated by
/// the app) and the colour its swatch shows, `None` for one whose colour the theme decides
/// (PhotoCraft's "Default" pasteboard).
#[derive(Clone, Debug, PartialEq)]
pub struct NamedColor {
    pub id: String,
    pub label: String,
    pub color: Option<Srgba>,
}

impl NamedColor {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color: Option<Srgba>) -> NamedColor {
        NamedColor { id: id.into(), label: label.into(), color }
    }
}

/// What a [`ColorChoice`] holds: one of the named colours by id, or the custom colour.
#[derive(Clone, Debug, PartialEq)]
pub enum ChosenColor {
    Named(String),
    Custom,
}

/// What [`ColorChoice::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the choice and opens the picker"]
pub struct ColorChoiceResponse {
    /// A row was chosen: a [`Set`](craft_ui::edit::EditEvent::Set) of the new choice.
    pub edit: EditResponse<ChosenColor>,
    /// "Select Custom Color…" was chosen: the app opens a
    /// [`ColorPickerDialog`](super::ColorPickerDialog) on the custom colour and, on OK, stores it
    /// and sets the choice to [`ChosenColor::Custom`].
    pub pick_custom: bool,
}

/// A [`Select`](craft_ui::controls::Select) of named colours, each row with a swatch before its
/// label, then "Custom" (the custom colour, shown once there is one), a separator and "Select
/// Custom Color…". The closed control shows the chosen row's swatch and label.
///
/// Keyboard and AccessKit as `Select`; each row's swatch is decoration (the label names it).
pub struct ColorChoice<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a ChosenColor,
    named: Vec<NamedColor>,
    custom: Option<Srgba>,
    size: ControlSize,
    enabled: bool,
}

impl<'a> ColorChoice<'a> {
    /// A choice of `value`, named `label` for screen readers.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a ChosenColor) -> ColorChoice<'a> {
        ColorChoice {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            named: Vec::new(),
            custom: None,
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// The named colours, in order.
    pub fn named(mut self, named: impl IntoIterator<Item = NamedColor>) -> ColorChoice<'a> {
        self.named = named.into_iter().collect();
        self
    }

    /// The custom colour; without it the "Custom" row is not shown.
    pub fn custom(mut self, color: Option<Srgba>) -> ColorChoice<'a> {
        self.custom = color;
        self
    }

    pub fn size(mut self, size: ControlSize) -> ColorChoice<'a> {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> ColorChoice<'a> {
        self.enabled = enabled;
        self
    }

    // pc: crates/ui-egui/src/canvas/canvas_tool_menu.rs:98 `PASTEBOARD_MENU` (Default, Black, Dark
    // Gray, Medium Gray, Light Gray, Custom, separator, Select Custom Color…); built on `Select`
    // with `row_content` for the swatch and `separator` before the picker row.
    pub fn show(self, ui: &mut Ui) -> ColorChoiceResponse {
        todo!()
    }
}
