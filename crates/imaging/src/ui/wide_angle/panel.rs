//! [`WideAnglePanel`]: Adaptive Wide Angle's settings column (task WS-WA1).

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Response, Ui, WidgetInfo};

use craft_theme::{ButtonVariant, ControlSize, TypeRole};
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Button, Checkbox, Select};
use craft_ui::edit::{self, EditEvent, GestureId};
use craft_ui::localized::{LocaleNumbers, Unit, Words};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};

use super::WideAngleLabels;
use super::labels::CONSTRAINTS;
use super::session::{ConstraintOrientation, WideAngleConstraintEdit, WideAngleModel, WideAngleSession};

/// Scale's unit.
const PERCENT: &str = "{n} %";
/// Focal Length's unit.
const MILLIMETRES: &str = "{n} mm";

/// What [`WideAnglePanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WideAnglePanelOutput {
    pub response: Response,
    /// The correction values changed (written to the session).
    pub changed: bool,
    pub dragging: bool,
    /// A row's Orientation or Delete.
    pub edit: Option<WideAngleConstraintEdit>,
}

/// One slider row of the correction values.
struct Slider<'a> {
    key: &'static str,
    label: &'a str,
    range: std::ops::RangeInclusive<f64>,
    unit: Option<&'static str>,
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
    labels: &'a WideAngleLabels,
}

impl<'a> WideAnglePanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, session: &'a mut WideAngleSession, preview: &'a mut bool, labels: &'a WideAngleLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, preview, labels }
    }

    pub fn show(self, ui: &mut Ui) -> WideAnglePanelOutput {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:363-437 (Correction, the sliders, the
        // constraint rows, Preview, the hint).
        let Self { id_salt, session, preview, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let ctx = ui.ctx().clone();
        let numbers = LocaleNumbers::of(ui);
        let defaults = WideAngleSession::default();
        let gesture_key = id.with("gesture");
        let mut gesture: Gesture = ctx.data(|d| d.get_temp(gesture_key)).flatten();
        let (mut changed, mut edit) = (false, None);

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
                    changed = true;
                }
                let rows = [
                    Slider {
                        key: "scale",
                        label: labels.scale.as_str(),
                        range: 50.0..=150.0,
                        unit: Some(PERCENT),
                        get: |s| s.scale,
                        set: |s, v| s.scale = v,
                    },
                    Slider {
                        key: "focal",
                        label: labels.focal_length.as_str(),
                        range: 0.0..=200.0,
                        unit: Some(MILLIMETRES),
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
                        row = row.unit(Unit::new(unit));
                    }
                    for event in row.show(ui).edits() {
                        changed |= apply(event, session, &mut gesture, set);
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
                            edit = edit.take().or(Some(WideAngleConstraintEdit::Orient { index, orientation: picked }));
                        }
                        let delete = Button::new(labels.delete.as_str())
                            .variant(ButtonVariant::Secondary)
                            .size(ControlSize::Sm)
                            .id_salt(id.with(("delete", index)));
                        let delete = ui.add(delete);
                        let name = craft_i18n::fmt(&labels.delete_constraint, &[("n", &number)]);
                        delete.widget_info(|| WidgetInfo::labeled(Role::Button, true, name.as_str()));
                        if delete.clicked() {
                            edit = edit.take().or(Some(WideAngleConstraintEdit::Delete(index)));
                        }
                    });
                }
            });
            let _checked = Checkbox::bool(labels.preview.as_str(), preview).id_salt(id.with("preview")).show(ui);
            ui.add(Text::new(labels.hint.as_str()).role(TypeRole::Caption).content(Content::Faint));
        });

        let dragging = gesture.as_ref().is_some_and(|(live, _)| edit::is_live(&ctx, *live));
        ctx.data_mut(|d| d.insert_temp(gesture_key, gesture.filter(|_| dragging)));
        WideAnglePanelOutput { response: stack.response, changed, dragging, edit }
    }
}

/// Writes `event`'s value into `session` through `set`; a Cancel puts back the session the
/// gesture began from. Returns whether the session changed.
fn apply(event: EditEvent<f64>, session: &mut WideAngleSession, gesture: &mut Gesture, set: fn(&mut WideAngleSession, f64)) -> bool {
    match event {
        EditEvent::Begin { id, value, .. } => {
            *gesture = Some((id, session.clone()));
            value.is_some_and(|value| write(session, set, value))
        }
        EditEvent::Update { value, .. } | EditEvent::Set { value, .. } => write(session, set, value),
        EditEvent::Commit { value, .. } => {
            *gesture = None;
            write(session, set, value)
        }
        EditEvent::Cancel { .. } => match gesture.take() {
            Some((_, before)) => std::mem::replace(session, before) != *session,
            None => false,
        },
        _ => false,
    }
}

fn write(session: &mut WideAngleSession, set: fn(&mut WideAngleSession, f64), value: f64) -> bool {
    let before = session.clone();
    set(session, value);
    *session != before
}
