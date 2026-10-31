//! [`PaintChip`] (F04.06): a shape's fill or stroke paint as a chip that opens a popover with No
//! Color, Solid and Gradient, as PhotoCraft's shape options bar and Properties panel; and
//! [`PaintList`], named paints in rows with their chips.
//!
//! The gradient editor is craft-draw's, and craft-draw depends on craft-color, so the chip takes
//! the editor's body from the host ([`PaintChip::gradient_editor`]).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::edit::EditResponse;
use craft_ui::localized::LocaleNumbers;

craft_ui::words! {
    /// The paint popover's tabs, English by default.
    #[non_exhaustive]
    pub struct PaintLabels {
        no_color: "paint.noColor" = "No Color";
        solid: "paint.solid" = "Solid";
        gradient: "paint.gradient" = "Gradient";
    }
}

/// A gradient as the chip draws it: stops at 0–1 along a left-to-right ramp. The gradient itself
/// (angle, style, midpoints) is the host's; it gives this preview with the paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientPreview {
    pub stops: Vec<(f32, Srgba)>,
}

/// A paint: none, a solid colour, or a gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    None,
    Solid(Srgba),
    Gradient(GradientPreview),
}

/// The host's gradient editor body, drawn in the popover's Gradient tab: it edits the host's
/// gradient and returns the new preview when it changed.
type GradientEditor<'a> = dyn FnMut(&mut Ui) -> Option<GradientPreview> + 'a;

/// A paint chip, 30 by 22 points in PhotoCraft (the `swatch` control's Md size here).
///
/// - Look: a solid paint fills the chip over the checkerboard (alpha shows); a gradient is a
///   left-to-right ramp through its stops; No Color is the field fill with a red diagonal
///   (`status.danger`) from bottom left to top right. Edge: `line.control_edge`, the focus ring
///   when focused.
/// - Click, Enter or Space opens a popover: a segmented No Color / Solid / Gradient switch, then
///   the Compact [`ColorPicker`](super::ColorPicker) (Solid, with the screen eyedropper) or the
///   host's gradient editor (Gradient). Switching to Solid starts from the last solid colour (or
///   the fallback), to Gradient from the host's default gradient
///   ([`default_gradient`](Self::default_gradient)).
/// - Edits: a switch is a [`Set`](craft_ui::edit::EditEvent::Set); picker gestures report as the
///   picker does. Alpha is kept through a screen sample.
/// - AccessKit: a `ColorWell` named by `label`, its value the hex or "No Color"/"Gradient".
pub struct PaintChip<'a> {
    id_salt: IdSalt,
    label: String,
    paint: &'a Paint,
    fallback: Srgba,
    numbers: &'a LocaleNumbers,
    none_allowed: bool,
    gradient: Option<(GradientPreview, Box<GradientEditor<'a>>)>,
    enabled: bool,
}

impl<'a> PaintChip<'a> {
    /// A chip for `paint`; `fallback` is the solid colour a switch to Solid starts from when the
    /// paint has none.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paint: &'a Paint, fallback: Srgba, numbers: &'a LocaleNumbers) -> PaintChip<'a> {
        PaintChip { id_salt: IdSalt::new(id_salt), label: label.into(), paint, fallback, numbers, none_allowed: true, gradient: None, enabled: true }
    }

    /// Whether No Color is offered; on by default (a stroke can be none, a text colour cannot).
    pub fn none_allowed(mut self, allowed: bool) -> PaintChip<'a> {
        self.none_allowed = allowed;
        self
    }

    /// The Gradient tab: `default_gradient` is what a switch to Gradient sets, `editor` draws the
    /// host's editor. Without it there is no Gradient tab.
    pub fn gradient_editor(
        mut self,
        default_gradient: GradientPreview,
        editor: impl FnMut(&mut Ui) -> Option<GradientPreview> + 'a,
    ) -> PaintChip<'a> {
        self.gradient = Some((default_gradient, Box::new(editor)));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> PaintChip<'a> {
        self.enabled = enabled;
        self
    }

    /// The gradient a switch to Gradient sets, when the host gave an editor.
    pub fn default_gradient(&self) -> Option<&GradientPreview> {
        self.gradient.as_ref().map(|(g, _)| g)
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 `swatch` (the chip at :72, the ramp at
    // :80, No Color at :103, the popover tabs at :133); built on `ColorPopover`'s popover and
    // `ColorPicker` Compact, `Segmented` for the tabs.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Paint> {
        todo!()
    }
}

/// Named paints in rows: the chip, then the name; a click on a row selects it, a double-click
/// renames it in place. The Paints list of a document (PhotoCraft's shape paints; FigCraft's
/// and VectorCraft's document colours).
pub struct PaintList<'a> {
    id_salt: IdSalt,
    label: String,
    paints: &'a [(String, Paint)],
    selected: Option<usize>,
}

/// What [`PaintList::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct PaintListResponse {
    pub selected: Option<usize>,
    /// A row was renamed: its index and new name.
    pub renamed: Option<(usize, String)>,
}

impl<'a> PaintList<'a> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paints: &'a [(String, Paint)]) -> PaintList<'a> {
        PaintList { id_salt: IdSalt::new(id_salt), label: label.into(), paints, selected: None }
    }

    pub fn selected(mut self, selected: Option<usize>) -> PaintList<'a> {
        self.selected = selected;
        self
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 (the chip per row); rows on
    // `craft_ui::list` with an in-place `TextField` rename.
    pub fn show(self, ui: &mut Ui) -> PaintListResponse {
        todo!()
    }
}
