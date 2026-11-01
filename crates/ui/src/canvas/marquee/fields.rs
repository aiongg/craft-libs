//! The Style, Width and Height cells of a marquee's options bar: [`ConstraintFields`].

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Modifiers, Ui};

use super::Constraint;
use crate::controls::{IconButton, NumberField, Select};
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::localized::LocaleNumbers;
use crate::localized::units::{LengthUnit, UnitContext};
use crate::patterns::{CellWidth, OptionsBar};
use craft_theme::ControlSize;

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
    /// The constraint changed: a Style pick or the swap button (`Set`), a typed Width or Height or
    /// a scrub of a caption (the number field's `Begin`, `Update` and `Commit`). The value is the whole new [`Constraint`]; it is tool settings, not a document
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
/// - Typing follows the number field's rule: a plain number applies as it is typed (`Begin`,
///   `Update`), arithmetic and unit text apply on Enter, Tab or leaving the field, and that
///   commit ends the run.
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
        let Self { id_salt, constraint, enabled, size_units, labels } = self;
        let labels = labels.unwrap_or_else(|| ConstraintFieldsLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let mut out = ConstraintFieldsOutput { event: None, unit_picked: None };
        let (w, h) = match *constraint {
            Constraint::Free => (1.0, 1.0),
            Constraint::Ratio(w, h) | Constraint::Size(w, h) => (w, h),
        };
        let free = matches!(constraint, Constraint::Free);
        let fields = Fields { constraint, size_units, numbers: &numbers };
        ui.push_id(id_salt, |ui| {
            OptionsBar::group(ui, enabled, |ui| {
                let mut style = StyleChoice::of(constraint);
                let current = style;
                let pick = OptionsBar::cell(ui, &labels.style, CellWidth::Content, |ui, _| {
                    Select::new("style", labels.style.as_str(), &mut style)
                        .option(StyleChoice::Normal, labels.normal.as_str())
                        .option(StyleChoice::FixedRatio, labels.fixed_ratio.as_str())
                        .option(StyleChoice::FixedSize, labels.fixed_size.as_str())
                        .size(ControlSize::Sm)
                        .show(ui)
                });
                if let Some(event) = pick.event
                    && let Some(&picked) = event.value()
                    && picked != current
                {
                    out.event = Some(EditEvent::Set { value: picked.constraint(), modifiers: Modifiers::NONE });
                }
                OptionsBar::group(ui, !free, |ui| {
                    fields.show(ui, "width", &labels.width, w, h, true, &mut out);
                    let swap = ui.add(IconButton::new(Icon::SwapHorizontal, labels.swap.as_str()).size(ControlSize::Sm));
                    if swap.clicked() {
                        let swapped = match *constraint {
                            Constraint::Free => None,
                            Constraint::Ratio(w, h) => Some(Constraint::Ratio(h, w)),
                            Constraint::Size(w, h) => Some(Constraint::Size(h, w)),
                        };
                        if let Some(value) = swapped {
                            out.event = Some(EditEvent::Set { value, modifiers: Modifiers::NONE });
                        }
                    }
                    fields.show(ui, "height", &labels.height, h, w, false, &mut out);
                });
            });
        });
        out
    }
}

/// The three kinds of [`Constraint`], as the Style select lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StyleChoice {
    Normal,
    FixedRatio,
    FixedSize,
}

impl StyleChoice {
    fn of(constraint: &Constraint) -> Self {
        match constraint {
            Constraint::Free => Self::Normal,
            Constraint::Ratio(..) => Self::FixedRatio,
            Constraint::Size(..) => Self::FixedSize,
        }
    }

    /// The constraint a pick starts with.
    fn constraint(self) -> Constraint {
        match self {
            Self::Normal => Constraint::Free,
            Self::FixedRatio => Constraint::Ratio(ConstraintFields::RATIO_START.0, ConstraintFields::RATIO_START.1),
            Self::FixedSize => Constraint::Size(ConstraintFields::SIZE_START.0, ConstraintFields::SIZE_START.1),
        }
    }
}

/// What the Width and Height cells share.
struct Fields<'a> {
    constraint: &'a Constraint,
    size_units: Option<SizeUnits<'a>>,
    numbers: &'a LocaleNumbers,
}

impl Fields<'_> {
    /// One of Width (`first`) and Height: `value` with the `other` side beside it.
    #[allow(clippy::too_many_arguments)]
    fn show(&self, ui: &mut Ui, name: &str, caption: &str, value: f64, other: f64, first: bool, out: &mut ConstraintFieldsOutput) {
        let pair = |v: f64| if first { (v, other) } else { (other, v) };
        match self.constraint {
            Constraint::Size(..) => {
                let (unit, units, width, height) = match &self.size_units {
                    Some(s) => (s.unit, s.units, s.width, s.height),
                    None => (LengthUnit::Pixels, &[][..], UnitContext::default(), UnitContext::default()),
                };
                let context = if first { width } else { height };
                let cell = OptionsBar::unit_cell(ui, name, caption, value, 1.0..=ConstraintFields::SIZE_MAX, unit, units, context);
                out.unit_picked = out.unit_picked.or(cell.unit_picked);
                if let Some(event) = cell.edit.event {
                    out.event = Some(event.map(|v| {
                        let (w, h) = pair(v);
                        Constraint::Size(w, h)
                    }));
                }
            }
            _ => {
                let field = NumberField::new(name, caption, value, self.numbers).range(ConstraintFields::RATIO_RANGE).decimals(0..=3);
                let edit = OptionsBar::number_cell(ui, caption, CellWidth::Sm, field);
                if let Some(event) = edit.event {
                    out.event = Some(event.map(|v| {
                        let (w, h) = pair(v);
                        Constraint::Ratio(w, h)
                    }));
                }
            }
        }
    }
}
