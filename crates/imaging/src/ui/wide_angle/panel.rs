//! [`WideAnglePanel`]: Adaptive Wide Angle's settings column (task WS-WA1).

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Modifiers, Response, Ui, WidgetInfo};

use craft_theme::{ButtonVariant, ControlSize, TypeRole};
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Button, Checkbox, Select};
use craft_ui::edit::{self, EditEvent, GestureId};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, UnitKind, Units, Words};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};

use super::WideAngleLabels;
use super::labels::CONSTRAINTS;
use super::session::{ConstraintOrientation, WideAngleConstraintEdit, WideAngleModel, WideAngleSession};

/// What [`WideAnglePanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WideAnglePanelOutput {
    pub response: Response,
    /// The latest edit of the correction values (the lens model and the sliders) this pass, with
    /// the session after it, already written to the session. A slider's gesture reports `Begin`,
    /// `Update`, then `Commit` or `Cancel`.
    pub edit: Option<EditEvent<WideAngleSession>>,
    /// A row's Orientation or Delete; the workspace applies it.
    pub constraint: Option<WideAngleConstraintEdit>,
}

/// One slider row of the correction values.
struct Slider<'a> {
    key: &'static str,
    label: &'a str,
    range: std::ops::RangeInclusive<f64>,
    unit: Option<Unit>,
    get: fn(&WideAngleSession) -> f64,
    set: fn(&mut WideAngleSession, f64),
}

/// The slider gesture in progress and the session it began from, kept in egui memory between
/// passes.
type Gesture = Option<(GestureId, WideAngleSession)>;

/// The settings column (Standard), top to bottom: **Correction** (Auto, Fisheye, Perspective, Full
/// Spherical), Scale (50..=150 %), Focal Length (0..=200 mm), Crop Factor (0.1..=10); **Constraints
/// (N)** (a `Counted` header): one row per constraint with its number, an Orientation select and
/// Delete; Preview (a check box: the corrected preview or the source); the hint "Drag on the image
/// to add a constraint; Shift for horizontal/vertical; right-click a constraint to change or delete
/// it." in the caption role, `text.faint`. The footer (OK, Cancel at the bottom) is the column's;
/// PhotoCraft's full-width footer and its "N ms · straightness" text are gone (lead calls 11, 14).
pub struct WideAnglePanel<'a> {
    id_salt: IdSalt,
    session: &'a mut WideAngleSession,
    preview: &'a mut bool,
    labels: Option<WideAngleLabels>,
}

impl<'a> WideAnglePanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, session: &'a mut WideAngleSession, preview: &'a mut bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, preview, labels: None }
    }

    /// The words; default [`WideAngleLabels::of`] the context's language.
    pub fn labels(mut self, labels: WideAngleLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> WideAnglePanelOutput {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:363-437 (Correction, the sliders, the
        // constraint rows, Preview, the hint).
        let Self { id_salt, session, preview, labels } = self;
        let labels = labels.unwrap_or_else(|| WideAngleLabels::of(ui));
        let units = Units::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let ctx = ui.ctx().clone();
        let numbers = LocaleNumbers::of(ui);
        let defaults = WideAngleSession::default();
        let gesture_key = id.with("gesture");
        let mut gesture: Gesture = ctx.data(|d| d.get_temp(gesture_key)).flatten();
        let (mut edit, mut constraint) = (None, None);

        let stack = Stack::new(Gap::Stack).show(ui, |ui| {
            SectionHeader::new(id.with("correction"), labels.correction.as_str()).collapsible(false).show(ui, |ui| {
                let mut model = session.model;
                let mut select = Select::new(id.with("model"), labels.correction.as_str(), &mut model);
                for (value, text) in [
                    (WideAngleModel::Auto, &labels.auto),
                    (WideAngleModel::Fisheye, &labels.fisheye),
                    (WideAngleModel::Perspective, &labels.perspective),
                    (WideAngleModel::FullSpherical, &labels.full_spherical),
                ] {
                    select = select.option(value, text.as_str());
                }
                if let Some(&picked) = select.show(ui).committed() {
                    session.model = picked;
                    edit = Some(EditEvent::set(session.clone(), Modifiers::NONE));
                }
                let rows = [
                    Slider {
                        key: "scale",
                        label: labels.scale.as_str(),
                        range: 50.0..=150.0,
                        unit: Some(units.unit(UnitKind::Percent)),
                        get: |s| s.scale,
                        set: |s, v| s.scale = v,
                    },
                    Slider {
                        key: "focal",
                        label: labels.focal_length.as_str(),
                        range: 0.0..=200.0,
                        unit: Some(Unit::new(labels.millimeters.as_str())),
                        get: |s| s.focal_length,
                        set: |s, v| s.focal_length = v,
                    },
                    Slider {
                        key: "crop",
                        label: labels.crop_factor.as_str(),
                        range: 0.1..=10.0,
                        unit: None,
                        get: |s| s.crop_factor,
                        set: |s, v| s.crop_factor = v,
                    },
                ];
                for Slider { key, label, range, unit, get, set } in rows {
                    let mut row = SliderRow::new(id.with(key), label, get(session), range, &numbers).default_value(get(&defaults));
                    if let Some(unit) = unit {
                        row = row.unit(unit);
                    }
                    for event in row.show(ui).edits() {
                        apply(event.clone(), session, &mut gesture, set);
                        edit = Some(event.map(|_| session.clone()));
                    }
                }
            });
            let words = Words::of(ui);
            let header = words.counted(&ctx, CONSTRAINTS, &words.get(CONSTRAINTS.word), session.constraints.len() as u64);
            SectionHeader::new(id.with("constraints"), header).collapsible(false).show(ui, |ui| {
                let count = session.constraints.len();
                for index in 0..count {
                    let Some(orientation) = session.constraints.get(index).map(|c| c.orientation) else { continue };
                    let number = numbers.format(index as f64 + 1.0, 0..=0);
                    Row::new(Gap::Inline).show(ui, |ui| {
                        ui.add(Text::new(number.as_str()).role(TypeRole::Label).content(Content::Muted));
                        let mut chosen = orientation;
                        let name = craft_i18n::fmt(&labels.orientation_of, &[("n", &number)]);
                        let mut select = Select::new(id.with(("orientation", index)), name, &mut chosen).fit(true).size(ControlSize::Sm);
                        for (value, text) in [
                            (ConstraintOrientation::Free, &labels.free),
                            (ConstraintOrientation::Horizontal, &labels.horizontal),
                            (ConstraintOrientation::Vertical, &labels.vertical),
                        ] {
                            select = select.option(value, text.as_str());
                        }
                        if let Some(&picked) = select.show(ui).committed() {
                            constraint = constraint.take().or(Some(WideAngleConstraintEdit::Orient { index, orientation: picked }));
                        }
                        let delete = Button::new(labels.delete.as_str())
                            .variant(ButtonVariant::Secondary)
                            .size(ControlSize::Sm)
                            .id_salt(id.with(("delete", index)));
                        let delete = ui.add(delete);
                        let name = craft_i18n::fmt(&labels.delete_constraint, &[("n", &number)]);
                        delete.widget_info(|| WidgetInfo::labeled(Role::Button, true, name.as_str()));
                        if delete.clicked() {
                            constraint = constraint.take().or(Some(WideAngleConstraintEdit::Delete(index)));
                        }
                    });
                }
            });
            let _checked = Checkbox::bool(labels.preview.as_str(), preview).id_salt(id.with("preview")).show(ui);
            ui.add(Text::new(labels.hint.as_str()).role(TypeRole::Caption).content(Content::Faint));
        });

        let gesture = gesture.filter(|(live, _)| edit::is_live(&ctx, *live));
        ctx.data_mut(|d| d.insert_temp(gesture_key, gesture));
        WideAnglePanelOutput { response: stack.response, edit, constraint }
    }
}

/// Writes `event`'s value into `session` through `set`; a Cancel puts back the session the
/// gesture began from.
fn apply(event: EditEvent<f64>, session: &mut WideAngleSession, gesture: &mut Gesture, set: fn(&mut WideAngleSession, f64)) {
    match event {
        EditEvent::Begin { id, value, .. } => {
            *gesture = Some((id, session.clone()));
            if let Some(value) = value {
                set(session, value);
            }
        }
        EditEvent::Update { value, .. } | EditEvent::Set { value, .. } => set(session, value),
        EditEvent::Commit { value, .. } => {
            *gesture = None;
            set(session, value);
        }
        EditEvent::Cancel { .. } => {
            if let Some((_, before)) = gesture.take() {
                *session = before;
            }
        }
        _ => {}
    }
}
