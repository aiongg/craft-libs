//! The Style, Width and Height cells of a marquee's options bar: [`ConstraintFields`].

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Ui};

use super::Constraint;
use crate::edit::EditEvent;
use crate::localized::units::{LengthUnit, UnitContext};

crate::words! {
    /// The words of [`ConstraintFields`], English by default (Photoshop's).
    #[non_exhaustive]
    pub struct ConstraintFieldsLabels {
        style: "constraintFields.style" = "Style";
        normal: "constraintFields.normal" = "Normal";
        fixed_ratio: "constraintFields.fixedRatio" = "Fixed Ratio";
        fixed_size: "constraintFields.fixedSize" = "Fixed Size";
        width: "constraintFields.width" = "Width";
        height: "constraintFields.height" = "Height";
        swap: "constraintFields.swap" = "Swaps height and width";
    }
}

/// What [`ConstraintFields::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ConstraintFieldsOutput {
    /// The constraint changed: a Style pick or the swap button (`Set`), a typed Width or Height
    /// (`Set` on Enter, Tab or leaving the field), a scrub of a caption (`Begin`, `Update`,
    /// `Commit`). The value is the whole new [`Constraint`]; it is tool settings, not a document
    /// edit, so the host stores the latest value (a scrub's Update too) and makes no undo step.
    pub event: Option<EditEvent<Constraint>>,
    /// A unit picked from a Fixed Size field's units menu: the host stores it with its settings
    /// (PhotoCraft's `marquee_unit`).
    pub unit_picked: Option<LengthUnit>,
}

/// The Style, Width, swap and Height cells of an `OptionsBar` that read and write a marquee's
/// [`Constraint`] (Photoshop's Rectangular Marquee bar, PhotoCraft's, and the Slice tool's).
///
/// ```no_run
/// use craft_ui::canvas::Constraint;
/// use craft_ui::canvas::marquee::fields::ConstraintFields;
/// # fn demo(ui: &mut egui::Ui, style: &mut Constraint, single_row: bool) {
/// let out = ConstraintFields::new("marquee-style", style).enabled(!single_row).show(ui);
/// if let Some(value) = out.event.as_ref().and_then(|e| e.value()) {
///     *style = *value;
/// }
/// # }
/// ```
///
/// - Cells, in the bar's reading order: "Style:" with a `Select` of Normal, Fixed Ratio and Fixed
///   Size; "Width:" with a number field; a swap icon button (`Icon::SwapHorizontal`, tooltip
///   [`ConstraintFieldsLabels::swap`]); "Height:" with a number field. The captions scrub their
///   fields (`OptionsBar::number_cell`, `OptionsBar::unit_cell`).
/// - Normal ([`Constraint::Free`]): Width, swap and Height are greyed and show 1 and 1.
///   Fixed Ratio ([`Constraint::Ratio`]): plain numbers in [`RATIO_RANGE`](Self::RATIO_RANGE).
///   Fixed Size ([`Constraint::Size`], pixels): length cells in the [size unit](Self::size_units)
///   (pixels by default), typed text may carry another unit (`2in`) or arithmetic, a secondary
///   click opens the units menu, values clamp to 1..=[`SIZE_MAX`](Self::SIZE_MAX) pixels, and
///   leaving a field that shows a rounded value is no change.
/// - Picking a style resets the numbers as PhotoCraft does: Fixed Ratio 1 : 1, Fixed Size 64 × 64
///   px. The swap button exchanges Width and Height.
/// - Typing keeps its text in the field until Enter, Tab or leaving the field (the number field's
///   rule); the constraint changes then, once.
/// - [`enabled`](Self::enabled) off greys every cell (Photoshop greys them for the Single Row and
///   Single Column Marquee).
/// - Keyboard: each cell is its own Tab stop in the bar's order. Right to left: the cells follow
///   the bar's reading order; Width stays before Height.
pub struct ConstraintFields<'a> {
    id_salt: IdSalt,
    constraint: &'a Constraint,
    enabled: bool,
    size_units: Option<SizeUnits<'a>>,
    labels: Option<ConstraintFieldsLabels>,
}

/// The unit of the Fixed Size fields and the context that converts it (resolution, and for a
/// percentage the document's width or height).
#[derive(Clone, Copy, Debug)]
struct SizeUnits<'a> {
    unit: LengthUnit,
    units: &'a [LengthUnit],
    width: UnitContext,
    height: UnitContext,
}

impl<'a> ConstraintFields<'a> {
    /// The range of the Fixed Ratio fields (PhotoCraft's `style_range`).
    pub const RATIO_RANGE: RangeInclusive<f64> = 0.001..=999.0;
    /// The largest Fixed Size side in pixels (PhotoCraft's `FIXED_SIZE_MAX_PX`).
    pub const SIZE_MAX: f64 = 300_000.0;
    /// The ratio a switch to Fixed Ratio starts with.
    pub const RATIO_START: (f64, f64) = (1.0, 1.0);
    /// The size in pixels a switch to Fixed Size starts with.
    pub const SIZE_START: (f64, f64) = (64.0, 64.0);

    /// The cells of the host's `constraint`.
    pub fn new(id_salt: impl AsIdSalt, constraint: &'a Constraint) -> Self {
        Self { id_salt: IdSalt::new(id_salt), constraint, enabled: true, size_units: None, labels: None }
    }

    /// Whether the cells take input; true by default.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows Fixed Size in `unit`, with `units` in the units menu (the host's ruler units list),
    /// converting through `width` and `height` (the document's resolution; for a percentage, its
    /// width and height as the extents). Without it Fixed Size is in pixels with no menu.
    pub fn size_units(mut self, unit: LengthUnit, units: &'a [LengthUnit], width: UnitContext, height: UnitContext) -> Self {
        self.size_units = Some(SizeUnits { unit, units, width, height });
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ConstraintFieldsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> ConstraintFieldsOutput {
        // pc: crates/ui-egui/src/panels/options_bar.rs:289-321 `marquee_options` (greyed for a single
        // row or column; the style dropdown resets to 64 × 64 or 1 × 1 at :294-296; Width, the swap
        // button "Swaps height and width" at :309-311, Height; Fixed Size through `fixed_size_field`
        // :758-774, the unit from a right-click); crates/ui-egui/src/tool_memory.rs:203-205
        // `style_range`; crates/ui-egui/src/panels/options_bar.rs:743 `FIXED_SIZE_MAX_PX`.
        // new: the cells are `OptionsBar::cell` with `Select`, `OptionsBar::number_cell`,
        // `OptionsBar::unit_cell` and `IconButton`; ids from `id_salt`.
        todo!()
    }
}
