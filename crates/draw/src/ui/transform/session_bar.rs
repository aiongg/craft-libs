//! [`TransformBar`]: the options bar of an open Free Transform session.

use egui::{AsIdSalt, IdSalt, Ui};

use craft_theme::ControlSize;
use craft_ui::canvas::{RulerUnits, Ticks};
use craft_ui::controls::{Checkbox, IconButton};
use craft_ui::icons::Icon;
use craft_ui::localized::units::{LengthUnit, LengthUnitLabels, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::patterns::OptionsBar;
use craft_ui::words;

use super::{
    BarGroups, Interpolation, ParseContext, SizeUnit, TransformEdit, TransformField, TransformFieldLabels, TransformFields, TransformPhase,
    TransformSession, TransformSettings, TransformWarp,
};

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
    size_unit: Option<LengthUnit>,
    unit_context: Option<UnitContext>,
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
    /// The unit picked in the W and H menu this pass (also written to
    /// [`TransformSettings::size_unit`] with [`settings`](TransformBar::settings)); the app keeps
    /// it and passes it back with [`size_unit`](TransformBar::size_unit) when it has no settings.
    pub size_unit: Option<LengthUnit>,
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
            size_unit: None,
            unit_context: None,
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

    /// The unit of W and H, when the app keeps it apart from the [`settings`](Self::settings)
    /// (which give it otherwise; percent when neither does).
    pub fn size_unit(mut self, unit: LengthUnit) -> Self {
        self.size_unit = Some(unit);
        self
    }

    /// The resolution and point size that turn pixels into the W and H unit. With it a secondary
    /// click on W or H opens the unit menu (the one the rulers use) and a pick is reported in
    /// [`TransformBarOutput::size_unit`]; without it W and H show the unit but have no menu.
    pub fn unit_context(mut self, context: UnitContext) -> Self {
        self.unit_context = Some(context);
        self
    }

    /// Draws the bar into the options bar's `ui`.
    pub fn show(self, ui: &mut Ui) -> TransformBarOutput<W> {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:189 (`options_bar`), :241
        // (interpolation), :532 (`transform_actions`: commit, cancel, reset, switch).
        let Self {
            id_salt,
            session,
            labels,
            field_labels,
            units,
            parser,
            groups,
            mut settings,
            warp_fields,
            extra,
            mode_switch,
            size_unit,
            unit_context,
        } = self;
        let id = ui.make_persistent_id(id_salt);
        let warping = session.phase() == TransformPhase::Warp;
        let mut out = TransformBarOutput { edits: Vec::new(), reset_warp: false, size_unit: None };
        let small = ControlSize::Sm;

        if warping {
            if let Some(show) = warp_fields {
                show(ui);
            }
        } else {
            let mut linked = settings.as_ref().map(|s| s.aspect_linked);
            let mut interpolation = session.interpolation().id().to_owned();
            let options: Vec<(String, String)> = Interpolation::ALL
                .into_iter()
                .map(|choice| {
                    let text = match choice {
                        Interpolation::Bicubic => &labels.bicubic,
                        Interpolation::Bilinear => &labels.bilinear,
                        Interpolation::NearestNeighbor => &labels.nearest_neighbor,
                    };
                    (choice.id().to_owned(), text.clone())
                })
                .collect();
            let unit = size_unit.or_else(|| settings.as_ref().and_then(|s| s.size_unit)).unwrap_or(LengthUnit::Percent);
            let (entries, mut picked) = (unit_context.map(|_| unit_entries(ui, unit)), None);
            let mut fields = TransformFields::new((id, "fields"), session.state(), session.rect(), field_labels)
                .skew_v(session.skew_v())
                .groups(groups)
                .interpolation(labels.interpolation.clone(), &mut interpolation, &options);
            if let Some((x, y)) = units {
                fields = fields.units(x, y);
            }
            fields = fields.size_unit(size_unit_of(ui, unit, unit_context.unwrap_or_default()));
            if let Some(entries) = &entries {
                fields = fields.size_menu(entries, &mut picked);
            }
            if let Some(linked) = linked.as_mut() {
                fields = fields.linked(linked);
            }
            if let Some(parser) = &parser {
                fields = fields.parser(|field, text, context| parser(field, text, context));
            }
            if let Some(event) = fields.show_bar(ui).event {
                out.edits.push(TransformEdit::Fields(event));
            }
            if let Some(new) = picked.and_then(|i| LengthUnit::ALL.get(i)).copied().filter(|new| *new != unit) {
                out.size_unit = Some(new);
                if let Some(settings) = settings.as_mut() {
                    settings.size_unit = Some(new);
                }
            }
            if let (Some(settings), Some(linked)) = (settings.as_mut(), linked) {
                settings.aspect_linked = linked;
            }
            if let Some(choice) = Interpolation::from_id(&interpolation).filter(|choice| *choice != session.interpolation()) {
                out.edits.push(TransformEdit::Interpolation(choice));
                if let Some(settings) = settings.as_mut() {
                    settings.interpolation = choice;
                }
            }
            if groups.reference {
                let mut shown = session.reference_point();
                if Checkbox::bool(labels.show_reference_point.clone(), &mut shown).size(small).show(ui).response.changed() {
                    out.edits.push(TransformEdit::ReferencePoint(shown));
                    if let Some(settings) = settings.as_mut() {
                        settings.show_reference_point = shown;
                    }
                }
            }
            if let Some(show) = extra {
                show(ui);
            }
        }

        OptionsBar::divider(ui);
        let (commit, cancel) =
            if warping { (&labels.commit_warp, &labels.cancel_warp) } else { (&labels.commit_transform, &labels.cancel_transform) };
        let pressed = OptionsBar::commit_cancel(ui, commit, cancel);
        if pressed.cancel {
            out.edits.push(TransformEdit::Cancel);
        }
        if pressed.commit {
            out.edits.push(TransformEdit::Commit);
        }
        if warping && ui.add(IconButton::new(Icon::Undo, labels.reset_warp.clone()).size(small)).clicked() {
            out.reset_warp = true;
        }
        if mode_switch {
            OptionsBar::divider(ui);
            if ui.add(IconButton::new(Icon::Grid, labels.switch_warp.clone()).size(small).selected(warping)).clicked() {
                out.edits.push(if warping { TransformEdit::LeaveWarp } else { TransformEdit::WarpRequested });
            }
        }
        out
    }
}

/// The W and H menu: every unit in the rulers' order, `current` marked. A row's command is the
/// unit's index in [`LengthUnit::ALL`].
fn unit_entries(ui: &Ui, current: LengthUnit) -> Vec<MenuEntry<usize>> {
    let labels = LengthUnitLabels::of(ui);
    LengthUnit::ALL.iter().enumerate().map(|(i, &unit)| MenuItem::new(unit.label(&labels), i).radio(unit == current).into()).collect()
}

/// How W and H show in `unit`: percent of the original side, or a length of the document's
/// pixels at `context`'s resolution.
fn size_unit_of(ui: &Ui, unit: LengthUnit, context: UnitContext) -> SizeUnit {
    if unit == LengthUnit::Percent {
        return SizeUnit::default();
    }
    let per_doc = context.from_pixels(1.0, unit).unwrap_or(1.0);
    let axis = RulerUnits {
        per_doc,
        ticks: Ticks::Decimal { whole: unit == LengthUnit::Pixels },
        numbers: LocaleNumbers::of(ui).as_ref().clone(),
        unit: Unit::new(&format!("{{n}} {}", unit.suffix())),
        decimals: unit.decimals(),
    };
    SizeUnit::Length { x: axis.clone(), y: axis }
}
