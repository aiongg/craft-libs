//! The options-bar brush chip ([`BrushPresetChip`]) that opens the Brush Preset picker.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Painter, Rect, Response, Ui};

use super::labels::BrushPickerLabels;

/// Photoshop's brush chip in the options bar: the current tip's image, its size under it and a
/// chevron, `brush.chip` (44 × 30). A click toggles the [`BrushPresetPicker`](super::BrushPresetPicker);
/// the host passes [`BrushPresetChipResponse::response`] as the picker's anchor and toggle, so a
/// press on the chip never counts as outside the picker.
///
/// - **Look.** The tip image (`brush.chip_tip`, 14) centred at `brush.chip_tip_at` from the
///   leading top corner, the size number (`caption` role, `text.muted`) centred under it 5 pt
///   off the bottom, the chevron at the trailing end; the hover fill while hovered or open.
/// - **Keys.** One Tab stop; Space and Enter toggle, as a button.
/// - **AccessKit.** A `Button` named "Brush Preset picker" with `has_popup` and `expanded` (the
///   `Popover` sets them on its control anchor); the same text as tooltip.
/// - **RTL.** The tip sits at the leading end, the chevron at the trailing end.
#[must_use = "show the chip with `show`"]
pub struct BrushPresetChip {
    id_salt: IdSalt,
    diameter: f64,
    open: bool,
    labels: Option<BrushPickerLabels>,
}

impl BrushPresetChip {
    /// A chip for the current brush.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), diameter: 0.0, open: false, labels: None }
    }

    /// The brush's diameter in pixels, shown rounded to a whole number in the installed number
    /// format (four digits fit).
    pub fn diameter(mut self, diameter: f64) -> Self {
        self.diameter = diameter;
        self
    }

    /// Whether the picker is open: the chip keeps its hover fill.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The words (shared with the picker); the installed language's by default.
    pub fn labels(mut self, labels: BrushPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the chip; `tip` paints the brush's tip image into its rect (white on dark themes,
    /// the theme's icon colour on light ones, as the app renders it).
    pub fn show(self, ui: &mut Ui, tip: impl FnOnce(&Painter, Rect)) -> BrushPresetChipResponse {
        // pc: crates/ui-egui/src/panels/brush_controls.rs:94-119 `brush_preset_chip` (44 × 30, tip 14 at (14, 11), number 5 pt off the bottom, chevron 9 pt in from the end)
        todo!()
    }
}

/// What [`BrushPresetChip::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct BrushPresetChipResponse {
    /// The chip: the picker's anchor (`PopoverAnchor::Control`) and toggle (`response.id`).
    pub response: Response,
    /// A click, Space or Enter on the chip this pass: the host flips the picker's `open`.
    pub clicked: bool,
}
