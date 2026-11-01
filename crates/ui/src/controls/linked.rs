#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`LinkedFields`]: number fields with a link toggle that keeps their values in proportion
//! (width and height) or equal (four sides), and the [`LinkButton`] on its own.

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::ControlSize;
use super::expr::ExprUnits;
use crate::edit::EditResponse;
use crate::localized::LocaleNumbers;

crate::words! {
    /// The link button's words, English by default.
    #[non_exhaustive]
    pub struct LinkedLabels {
        /// [`LinkMode::Ratio`]'s tooltip and accessible name (PhotoCraft's Clone Source,
        /// craft-draw's transform bar and Photoshop's Image Size).
        keep_ratio: "linked.keepRatio" = "Maintain aspect ratio";
        /// [`LinkMode::Equal`]'s (DesignCraft's margins, InDesign's).
        same_values: "linked.sameValues" = "Make all settings the same";
    }
}

/// What the link keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinkMode {
    /// A value typed into one field scales the others by the same factor: width and height.
    #[default]
    Ratio,
    /// A value typed into one field is copied to the others: four sides, X and Y offsets.
    Equal,
}

/// The chain toggle: [`Icon::Link`](crate::icons::Icon::Link) while linked,
/// [`Icon::Unlink`](crate::icons::Icon::Unlink) otherwise, as a toggle
/// [`IconButton`](crate::controls::IconButton) (pressed while linked) with the mode's tooltip.
/// A click or Space flips `linked`; AccessKit reads a toggle button named by the tooltip, with
/// `toggled`. The response is `changed()` in the pass the link flipped.
pub struct LinkButton<'a> {
    id_salt: IdSalt,
    linked: &'a mut bool,
    mode: LinkMode,
    size: ControlSize,
    labels: Option<LinkedLabels>,
}

impl<'a> LinkButton<'a> {
    pub fn new(id_salt: impl AsIdSalt, linked: &'a mut bool, mode: LinkMode) -> Self {
        Self { id_salt: IdSalt::new(id_salt), linked, mode, size: ControlSize::Md, labels: None }
    }

    /// Words other than the installed ones (PhotoCraft's Properties' Canvas says "Link width
    /// and height").
    pub fn labels(mut self, labels: LinkedLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    // pc: crates/ui-egui/src/panels/preset_panels/panels.rs:603 (Clone Source link button)
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}

/// How the fields sit: a row with the link between the first two (PhotoCraft's W ⛓ H), or a
/// column with the link beside them (DesignCraft's margins, PhotoCraft's Canvas section).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinkedLayout {
    #[default]
    Row,
    Column,
}

/// `N` number fields joined by a link, after PhotoCraft's linked W and H (Clone Source,
/// Properties' Canvas) and DesignCraft's linked margins. The transform bar's W and H are
/// craft-draw's `TransformFields::linked`, which works on the transform box itself.
///
/// ```ignore
/// let out = LinkedFields::new("size", ["W:", "H:"], [w, h], &mut prefs.size_linked, LinkMode::Ratio, &numbers)
///     .units(&ExprUnits { unit: Some(LengthUnit::Pixels), suffix: "px", context })
///     .show(ui);
/// if let Some(EditEvent::Commit { value: [w, h], .. }) = out.event { resize(w, h) }
/// ```
///
/// - **Fields.** One [`NumberField`](crate::controls::NumberField) per caption (the caption
///   before it, as the field's scrub label), sharing the units, the decimals and the size, each
///   with its range. With [`units`](Self::units) each field is a
///   [`unit_number_field`](crate::patterns::unit_number_field) (the unit after the number, typed
///   units and arithmetic, the unit's decimals), so its value with its unit is what screen
///   readers read; without, a plain field. A [`LinkButton`] sits between the first two (row) or
///   after them, centred on their height (column).
/// - **Linked.** An edit of one field moves the others with it, live through the gesture: in
///   [`LinkMode::Ratio`] by its factor ([`follow_ratio`]), kept inside each field's range by
///   taking the smallest factor (PhotoCraft's `k.min(wlim / sx).min(hlim / sy)`); in
///   [`LinkMode::Equal`] to the same value ([`follow_equal`]). Every value a gesture reports is
///   built from the values at its `Begin`, never from the previous update, so a scrub or a
///   value typed digit by digit keeps the starting ratio (lead call L24). Unlinked, a field
///   moves alone.
/// - **Linking.** Turning the link on in [`LinkMode::Equal`] sets every field to the first one's
///   value at once, one `Set` (DesignCraft's `setup_link`); in [`LinkMode::Ratio`] it changes no
///   value: the ratio is the current one.
/// - **Report.** One [`EditResponse`] for the group whose events carry every field's value
///   after the link rule, so the host applies them together and makes one undo step per
///   gesture. The link's own state is `linked`, which the host keeps (settings or the dialog's
///   remembered values).
/// - **Keys.** Each field is its own Tab stop and the link button one between them, in order.
///   Esc in a field cancels the gesture for every field (one `Cancel`).
pub struct LinkedFields<'a, const N: usize> {
    id_salt: IdSalt,
    captions: [&'a str; N],
    values: [f64; N],
    linked: &'a mut bool,
    mode: LinkMode,
    numbers: &'a LocaleNumbers,
    ranges: Option<[RangeInclusive<f64>; N]>,
    units: Option<ExprUnits<'a>>,
    decimals: Option<RangeInclusive<usize>>,
    layout: LinkedLayout,
    size: ControlSize,
    enabled: bool,
    labels: Option<LinkedLabels>,
}

impl<'a, const N: usize> LinkedFields<'a, N> {
    /// Fields captioned `captions` (as shown, translated; each also its field's accessible
    /// name) showing `values`.
    pub fn new(
        id_salt: impl AsIdSalt,
        captions: [&'a str; N],
        values: [f64; N],
        linked: &'a mut bool,
        mode: LinkMode,
        numbers: &'a LocaleNumbers,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            captions,
            values,
            linked,
            mode,
            numbers,
            ranges: None,
            units: None,
            decimals: None,
            layout: LinkedLayout::Row,
            size: ControlSize::Md,
            enabled: true,
            labels: None,
        }
    }

    /// Every field's range ([`NumberField::range`](crate::controls::NumberField::range)).
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.ranges = Some(std::array::from_fn(|_| range.clone()));
        self
    }

    /// Each field's own range (PhotoCraft's Canvas: each side's own limit).
    pub fn ranges(mut self, ranges: [RangeInclusive<f64>; N]) -> Self {
        self.ranges = Some(ranges);
        self
    }

    /// Every field's units: each field is built by
    /// [`unit_number_field`](crate::patterns::unit_number_field) with them.
    pub fn units(mut self, units: &ExprUnits<'a>) -> Self {
        self.units = Some(*units);
        self
    }

    /// Every field's decimals ([`NumberField::decimals`](crate::controls::NumberField::decimals));
    /// over the units' own decimals when both are given.
    pub fn decimals(mut self, decimals: RangeInclusive<usize>) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn layout(mut self, layout: LinkedLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Words other than the installed ones, for the link button.
    pub fn labels(mut self, labels: LinkedLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pc: crates/ui-egui/src/panels/preset_panels/panels.rs:597 (Clone Source W, link, H)
    // pc: crates/ui-egui/src/panels/doc_props_ui.rs:149 (Properties' Canvas, link beside W and H)
    // designcraft: crates/ui-egui/src/dialogs.rs:2055 setup_link, :2263 setup_link_follow
    pub fn show(self, ui: &mut Ui) -> EditResponse<[f64; N]> {
        todo!()
    }
}

/// The values after field `changed` became `new` under a ratio link: every value scaled by
/// `new / values[changed]`, the factor lowered so no value leaves its field's range in `ranges`
/// (PhotoCraft's `k.min(wlim / sx).min(hlim / sy)`). An old value of zero scales nothing: only
/// `changed` moves. A gesture passes the values at its `Begin`, so the ratio never drifts.
// pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:346 (linked k)
pub fn follow_ratio<const N: usize>(values: [f64; N], changed: usize, new: f64, ranges: &[RangeInclusive<f64>; N]) -> [f64; N] {
    todo!()
}

/// The values after field `changed` became `new` under an equal link: all `new`.
// designcraft: crates/ui-egui/src/dialogs.rs:2263 setup_link_follow
pub fn follow_equal<const N: usize>(values: [f64; N], changed: usize, new: f64) -> [f64; N] {
    todo!()
}
