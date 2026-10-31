#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`LinkedFields`]: number fields with a link toggle that keeps their values in proportion
//! (width and height) or equal (four sides), and the [`LinkButton`] on its own.

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::ControlSize;
use crate::edit::EditResponse;
use crate::localized::{LocaleNumbers, Unit};
use crate::menu::MenuEntry;

crate::words! {
    /// The link button's words, English by default.
    #[non_exhaustive]
    pub struct LinkedLabels {
        /// [`LinkMode::Ratio`]'s tooltip and accessible name (PhotoCraft's transform bar, Clone
        /// Source and Photoshop's Image Size).
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
/// `toggled`.
pub struct LinkButton<'a> {
    id_salt: IdSalt,
    linked: &'a mut bool,
    mode: LinkMode,
    size: ControlSize,
}

impl<'a> LinkButton<'a> {
    pub fn new(id_salt: impl AsIdSalt, linked: &'a mut bool, mode: LinkMode) -> Self {
        Self { id_salt: IdSalt::new(id_salt), linked, mode, size: ControlSize::Md }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:331 (link button)
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

/// `N` number fields joined by a link, after PhotoCraft's linked W and H (transform options bar,
/// Clone Source, Properties' Canvas) and DesignCraft's linked margins.
///
/// ```ignore
/// let out = LinkedFields::new("size", ["W:", "H:"], [w, h], &mut prefs.size_linked, LinkMode::Ratio, &numbers)
///     .unit(Unit::pixels())
///     .show(ui);
/// if let Some(EditEvent::Commit { value: [w, h], .. }) = out.event { resize(w, h) }
/// ```
///
/// - **Fields.** One [`NumberField`](crate::controls::NumberField) per label (the label before
///   it, as the field's scrub label), sharing the unit, its menu, the decimals and the size, each
///   with its range; a
///   [`LinkButton`] between the first two (row) or after them, centred on their height (column).
/// - **Linked.** An edit of one field moves the others with it, live through the gesture: in
///   [`LinkMode::Ratio`] by its factor ([`follow_ratio`]), kept inside each field's range by
///   taking the smallest factor (PhotoCraft's transform bar); in [`LinkMode::Equal`] to the same
///   value ([`follow_equal`]). Unlinked, a field moves alone.
/// - **Linking.** Turning the link on in [`LinkMode::Equal`] sets every field to the first one's
///   value at once, one `Set` (DesignCraft's `setup_link`); in [`LinkMode::Ratio`] it changes no
///   value: the ratio is the current one.
/// - **Report.** One [`EditResponse`] for the group whose events carry every field's value
///   after the link rule, so the host applies them together and makes one undo step per
///   gesture. The link's own state is `linked`, which the host keeps (settings or the dialog's
///   remembered values).
/// - **Keys.** Each field is its own Tab stop and the link button one between them, in order.
pub struct LinkedFields<'a, const N: usize> {
    id_salt: IdSalt,
    labels: [&'a str; N],
    values: [f64; N],
    linked: &'a mut bool,
    mode: LinkMode,
    numbers: &'a LocaleNumbers,
    ranges: Option<[RangeInclusive<f64>; N]>,
    unit: Option<Unit>,
    unit_menu: Option<(&'a [MenuEntry<usize>], &'a mut Option<usize>)>,
    decimals: Option<RangeInclusive<usize>>,
    layout: LinkedLayout,
    size: ControlSize,
    enabled: bool,
}

impl<'a, const N: usize> LinkedFields<'a, N> {
    /// Fields labelled `labels` (as shown, translated) showing `values`.
    pub fn new(
        id_salt: impl AsIdSalt,
        labels: [&'a str; N],
        values: [f64; N],
        linked: &'a mut bool,
        mode: LinkMode,
        numbers: &'a LocaleNumbers,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            labels,
            values,
            linked,
            mode,
            numbers,
            ranges: None,
            unit: None,
            unit_menu: None,
            decimals: None,
            layout: LinkedLayout::Row,
            size: ControlSize::Md,
            enabled: true,
        }
    }

    /// Every field's range ([`NumberField::range`](crate::controls::NumberField::range)).
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.ranges = Some(std::array::from_fn(|_| range.clone()));
        self
    }

    /// Each field's own range (PhotoCraft's transform W and H: each side's scale limit).
    pub fn ranges(mut self, ranges: [RangeInclusive<f64>; N]) -> Self {
        self.ranges = Some(ranges);
        self
    }

    /// Every field's unit ([`NumberField::unit`](crate::controls::NumberField::unit)).
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// The unit menu every field opens on a secondary click
    /// ([`NumberField::unit_menu`](crate::controls::NumberField::unit_menu)); the row picked goes
    /// to `picked`, whichever field it came from (PhotoCraft's W and H share one unit).
    pub fn unit_menu(mut self, entries: &'a [MenuEntry<usize>], picked: &'a mut Option<usize>) -> Self {
        self.unit_menu = Some((entries, picked));
        self
    }

    /// Every field's decimals ([`NumberField::decimals`](crate::controls::NumberField::decimals)).
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

    // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:328 (W, link, H)
    // designcraft: crates/ui-egui/src/dialogs.rs:2055 setup_link, :2263 setup_link_follow
    pub fn show(self, ui: &mut Ui) -> EditResponse<[f64; N]> {
        todo!()
    }
}

/// The values after field `changed` became `new` under a ratio link: every value scaled by
/// `new / values[changed]`, the factor lowered so no value leaves its field's range in `ranges`
/// (PhotoCraft's `k.min(wlim / sx).min(hlim / sy)`). An old value of zero scales nothing: only
/// `changed` moves.
// pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:346 (linked k)
pub fn follow_ratio<const N: usize>(values: [f64; N], changed: usize, new: f64, ranges: &[RangeInclusive<f64>; N]) -> [f64; N] {
    todo!()
}

/// The values after field `changed` became `new` under an equal link: all `new`.
// designcraft: crates/ui-egui/src/dialogs.rs:2263 setup_link_follow
pub fn follow_equal<const N: usize>(values: [f64; N], changed: usize, new: f64) -> [f64; N] {
    todo!()
}
