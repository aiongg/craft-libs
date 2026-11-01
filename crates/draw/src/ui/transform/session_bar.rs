//! [`TransformBar`]: the options bar of an open Free Transform session.

#![expect(dead_code, clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TR-T9)")]

use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::canvas::RulerUnits;
use craft_ui::words;

use super::{BarGroups, ParseContext, TransformEdit, TransformField, TransformFieldLabels, TransformSession, TransformSettings, TransformWarp};

/// The app's parser of a typed field, as [`TransformFields::parser`](super::TransformFields::parser)
/// takes it.
type Parser<'a> = Box<dyn Fn(TransformField, &str, ParseContext) -> Option<f64> + 'a>;

/// A closure that draws a group of the bar.
type Group<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

words! {
    /// The words of [`TransformBar`]. The commit and cancel names carry the phase: "Commit
    /// transform" in the Box phase, "Commit warp" in the Warp phase; their tooltips add the key
    /// (`OptionsBar::commit_cancel`), which gives PhotoCraft's "Commit warp (Enter)". `Default` is
    /// English.
    pub struct TransformBarLabels {
        interpolation: "transformBar.interpolation" = "Interpolation";
        bicubic: "transformBar.bicubic" = "Bicubic";
        bilinear: "transformBar.bilinear" = "Bilinear";
        nearest_neighbor: "transformBar.nearestNeighbor" = "Nearest Neighbor";
        show_reference_point: "transformBar.showReferencePoint" = "Show Reference Point";
        commit_transform: "transformBar.commitTransform" = "Commit transform";
        cancel_transform: "transformBar.cancelTransform" = "Cancel transform";
        commit_warp: "transformBar.commitWarp" = "Commit warp";
        cancel_warp: "transformBar.cancelWarp" = "Cancel warp";
        reset_warp: "transformBar.resetWarp" = "Reset warp";
        switch_warp: "transformBar.switchWarp" = "Switch between free transform and warp modes";
    }
}

/// The options bar of an open [`TransformSession`].
///
/// - Box phase, in Tab order: the [`TransformFields`](super::TransformFields) row (reference point
///   picker, X, Y, W, link, H, angle, H and V skew), the interpolation select, the Show Reference
///   Point check, the app's [`extra`](Self::extra) group, then cancel and commit at the reading
///   end (`OptionsBar::commit_cancel`) and the Free Transform / Warp switch after them (an
///   `IconButton` with the grid icon, pressed in the Warp phase, LC28).
/// - Warp phase: the app's [`warp_fields`](Self::warp_fields) group (imaging's `WarpFields`) in
///   place of the fields, then cancel, commit, Reset warp and the switch.
/// - [`groups`](Self::groups) narrows the row (Content-Aware Scale: position and size only, no
///   switch).
/// - Every control is an ordinary focusable control. The bar never changes the session; with
///   [`settings`](Self::settings) it writes the persisted choices (Show Reference Point, the
///   aspect link, interpolation, the W and H unit) there as well as reporting them.
/// - RTL: the options bar's own layout; commit and cancel stay at the reading end.
pub struct TransformBar<'a, W> {
    id_salt: IdSalt,
    session: &'a TransformSession<W>,
    labels: TransformBarLabels,
    field_labels: TransformFieldLabels,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    parser: Option<Parser<'a>>,
    groups: BarGroups,
    settings: Option<&'a mut TransformSettings>,
    warp_fields: Option<Group<'a>>,
    extra: Option<Group<'a>>,
    mode_switch: bool,
}

/// What the bar reported this pass.
#[derive(Debug)]
pub struct TransformBarOutput<W> {
    /// The edits in the order they happened; the app applies each with
    /// [`TransformSession::apply`].
    pub edits: Vec<TransformEdit<W>>,
    /// Reset warp was pressed (Warp phase): the app answers with its identity warp for the box,
    /// `TransformEdit::Warp(identity)`, one undo step (PhotoCraft `reset_warp`).
    pub reset_warp: bool,
}

impl<'a, W: TransformWarp> TransformBar<'a, W> {
    /// The bar of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a TransformSession<W>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            session,
            labels: TransformBarLabels::default(),
            field_labels: TransformFieldLabels::default(),
            units: None,
            parser: None,
            groups: BarGroups::ALL,
            settings: None,
            warp_fields: None,
            extra: None,
            mode_switch: true,
        }
    }

    pub fn labels(mut self, labels: TransformBarLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The fields' labels; English by default.
    pub fn field_labels(mut self, labels: TransformFieldLabels) -> Self {
        self.field_labels = labels;
        self
    }

    /// The ruler units of X and Y.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The app's parser of typed values (units, arithmetic).
    pub fn parser(mut self, parser: impl Fn(TransformField, &str, ParseContext) -> Option<f64> + 'a) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    /// Which groups of the fields show; all by default.
    pub fn groups(mut self, groups: BarGroups) -> Self {
        self.groups = groups;
        self
    }

    /// The persisted section: the check, the link, the interpolation and the W and H unit are
    /// written here when changed.
    pub fn settings(mut self, settings: &'a mut TransformSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The Warp phase's group (imaging's `WarpFields`), shown in place of the fields.
    pub fn warp_fields(mut self, show: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.warp_fields = Some(Box::new(show));
        self
    }

    /// The app's own group after the check (Content-Aware Scale's Amount and Protect fields).
    pub fn extra(mut self, show: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.extra = Some(Box::new(show));
        self
    }

    /// Whether the Free Transform / Warp switch shows; true by default (Content-Aware Scale
    /// turns it off).
    pub fn mode_switch(mut self, shown: bool) -> Self {
        self.mode_switch = shown;
        self
    }

    /// Draws the bar into the options bar's `ui`.
    pub fn show(self, ui: &mut Ui) -> TransformBarOutput<W> {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:189 (`options_bar`), :241
        // (interpolation), :532 (`transform_actions`: commit, cancel, reset, switch).
        todo!()
    }
}
