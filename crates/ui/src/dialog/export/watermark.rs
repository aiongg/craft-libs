//! [`WatermarkEditor`]: the Watermark group of an export dialog, after Lightroom's watermark
//! editor and LightCraft's port of it.

use craft_host::{Host, PickId, PickRequest, Picked};
use egui::{Modifiers, Ui};
use serde::{Deserialize, Serialize};

use super::super::sizing::{Anchor, AnchorGrid};
use crate::composites::{FieldControl as _, FormGrid};
use crate::controls::{Button, Checkbox, NumberField, Select, Slider, TextField};
use crate::edit::{EditEvent, EditResponse};
use crate::localized::{LocaleNumbers, SharedLabels as _, Unit};

crate::words! {
    /// The watermark group's words, English by default.
    #[non_exhaustive]
    pub struct WatermarkLabels {
        style: "watermark.style" = "Style";
        text_style: "watermark.textStyle" = "Text";
        graphic_style: "watermark.graphicStyle" = "Graphic";
        text: "watermark.text" = "Text";
        image: "watermark.image" = "Image";
        choose: "watermark.choose" = "Choose…";
        orientation: "watermark.orientation" = "Orientation";
        horizontal: "watermark.horizontal" = "Horizontal";
        vertical: "watermark.vertical" = "Vertical";
        position: "watermark.position" = "Position";
        rotation: "watermark.rotation" = "Rotation";
        size: "watermark.size" = "Size";
        opacity: "watermark.opacity" = "Opacity";
        inset_x: "watermark.insetX" = "Inset X";
        inset_y: "watermark.insetY" = "Inset Y";
        shadow: "watermark.shadow" = "Shadow";
    }
}

/// Text or an image.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WatermarkStyle {
    Text(String),
    /// The image file's path.
    Graphic(String),
}

/// A watermark. Sizes are percent of the image's short edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Watermark {
    pub style: WatermarkStyle,
    pub vertical: bool,
    /// One of the nine anchors.
    pub position: Anchor,
    /// 0, 90, 180 or 270 degrees.
    pub rotation: u16,
    /// 1–15 %.
    pub size: f32,
    /// 5–100 %.
    pub opacity: f32,
    /// 0–40 % each.
    pub inset: [f32; 2],
    pub shadow: bool,
}

impl Default for Watermark {
    /// LightCraft's defaults: size 3.5 %, opacity 70 %, inset 2.5 % on both axes.
    // lightcraft: crates/engine/src/export/watermark.rs:55 Watermark::default
    fn default() -> Self {
        Self {
            style: WatermarkStyle::Text(String::new()),
            vertical: false,
            position: Anchor::BottomRight,
            rotation: 0,
            size: 3.5,
            opacity: 70.0,
            inset: [2.5, 2.5],
            shadow: true,
        }
    }
}

/// The watermark rows inside a form: Style (Text, Graphic), the text field or the image path
/// with Choose… (a picker through `host.files()`, purpose `"watermark"`), Orientation, Position
/// as the 3×3 [`AnchorGrid`](crate::dialog::AnchorGrid), Rotation, Size, Opacity, Inset X and Y as
/// slider rows, Shadow. The preview stays the dialog's: LightCraft draws it on a job.
///
/// ```ignore
/// let r = WatermarkEditor::new("export-watermark", &mut settings.watermark).show(ui);
/// if r.event.is_some() { preview.request(); }
/// ```
pub struct WatermarkEditor<'a> {
    id_salt: egui::IdSalt,
    value: &'a mut Watermark,
}

impl<'a> WatermarkEditor<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, value: &'a mut Watermark) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), value }
    }

    /// Draws the rows. Every control's event comes back as the whole watermark, already applied to
    /// `value`; with several in one pass the last wins. Switching Style keeps the text and the
    /// image path apart, so switching back restores them.
    // lightcraft: crates/ui-egui/src/dialogs.rs:1290 export_watermark
    pub fn show(self, ui: &mut Ui) -> EditResponse<Watermark> {
        let labels = WatermarkLabels::shared(ui);
        let numbers = LocaleNumbers::of(ui);
        let host = Host::of(ui);
        let ctx = ui.ctx().clone();
        let salt = self.id_salt;
        let scope = ui.scope_id();
        let ticket_key = scope.with((salt, "ticket"));
        let stash_key = move |graphic: bool| scope.with((salt, "stash", graphic));
        let wm = self.value;
        let graphic = matches!(wm.style, WatermarkStyle::Graphic(_));
        let content = match &wm.style {
            WatermarkStyle::Text(text) | WatermarkStyle::Graphic(text) => text.clone(),
        };
        let mut event = None;

        let inner = FormGrid::new(salt).show(ui, |form| {
            form.row(labels.style.as_str(), |ui, _| {
                let mut choice = graphic;
                let edit = Select::new((salt, "style"), labels.style.as_str(), &mut choice)
                    .option(false, labels.text_style.as_str())
                    .option(true, labels.graphic_style.as_str())
                    .show(ui);
                apply(&mut event, wm, edit.event.clone(), |wm, to_graphic| {
                    let stashed = ctx.data_mut(|d| {
                        d.insert_temp(stash_key(!to_graphic), content.clone());
                        d.get_temp::<String>(stash_key(to_graphic)).unwrap_or_default()
                    });
                    wm.style = if to_graphic { WatermarkStyle::Graphic(stashed) } else { WatermarkStyle::Text(stashed) };
                });
                edit
            });

            if graphic {
                form.row(labels.image.as_str(), |ui, _| {
                    let edit = TextField::new((salt, "image"), labels.image.as_str(), content.as_str()).placeholder("logo.png").show(ui);
                    apply(&mut event, wm, edit.event.clone(), |wm, path| wm.style = WatermarkStyle::Graphic(path));
                    if ui.add(Button::new(labels.choose.as_str()).id_salt((salt, "choose"))).clicked() {
                        let request = PickRequest::open("watermark").filter("Images", &["png", "jpg", "jpeg", "webp", "tif", "tiff"]);
                        let id = host.files().pick(request);
                        ui.data_mut(|d| d.insert_temp(ticket_key, id));
                    }
                    if let Some(ticket) = ui.data(|d| d.get_temp::<PickId>(ticket_key))
                        && let Some(answer) = host.files().take_pick()
                    {
                        if answer.id == ticket {
                            ui.data_mut(|d| d.remove::<PickId>(ticket_key));
                            let path = match answer.picked {
                                Picked::Paths(paths) => paths.first().map(|p| p.to_string_lossy().into_owned()),
                                Picked::Contents(files) => files.first().map(|(name, _)| name.clone()),
                                Picked::Cancelled => None,
                            };
                            if let Some(path) = path {
                                apply(&mut event, wm, Some(EditEvent::set(path, Modifiers::NONE)), |wm, path| {
                                    wm.style = WatermarkStyle::Graphic(path);
                                });
                            }
                        } else {
                            host.files().return_pick(answer);
                        }
                    }
                    edit
                });
            } else {
                form.row(labels.text.as_str(), |ui, _| {
                    let edit = TextField::new((salt, "text"), labels.text.as_str(), content.as_str()).multiline(2).show(ui);
                    apply(&mut event, wm, edit.event.clone(), |wm, text| wm.style = WatermarkStyle::Text(text));
                    edit
                });
                form.row(labels.orientation.as_str(), |ui, _| {
                    let mut vertical = wm.vertical;
                    let edit = Select::new((salt, "orientation"), labels.orientation.as_str(), &mut vertical)
                        .option(false, labels.horizontal.as_str())
                        .option(true, labels.vertical.as_str())
                        .show(ui);
                    apply(&mut event, wm, edit.event.clone(), |wm, vertical| wm.vertical = vertical);
                    edit
                });
            }

            form.row(labels.position.as_str(), |ui, _| {
                let mut position = wm.position;
                let response = AnchorGrid::new(&mut position, (0.0, 0.0)).arrows(false).show(ui);
                if response.changed() {
                    apply(&mut event, wm, Some(EditEvent::set(position, Modifiers::NONE)), |wm, position| wm.position = position);
                }
                response.own_name()
            });

            form.row(labels.rotation.as_str(), |ui, _| {
                let mut rotation = wm.rotation;
                let mut select = Select::new((salt, "rotation"), labels.rotation.as_str(), &mut rotation);
                for degrees in ROTATIONS {
                    select = select.option(degrees, format!("{degrees}\u{b0}"));
                }
                let edit = select.show(ui);
                apply(&mut event, wm, edit.event.clone(), |wm, rotation| wm.rotation = rotation);
                edit
            });

            let size = scalar_row(form, &numbers, (salt, "size"), &labels.size, wm.size, SIZE_RANGE, 0.5, true);
            apply(&mut event, wm, size, |wm, v| wm.size = v as f32);
            let opacity = scalar_row(form, &numbers, (salt, "opacity"), &labels.opacity, wm.opacity, OPACITY_RANGE, 1.0, true);
            apply(&mut event, wm, opacity, |wm, v| wm.opacity = v as f32);
            // Along an axis the position centres on, the inset has nothing to offset.
            let across = !matches!(wm.position, Anchor::Top | Anchor::Center | Anchor::Bottom);
            let down = !matches!(wm.position, Anchor::Left | Anchor::Center | Anchor::Right);
            let inset_x = scalar_row(form, &numbers, (salt, "inset-x"), &labels.inset_x, wm.inset[0], INSET_RANGE, 0.5, across);
            apply(&mut event, wm, inset_x, |wm, v| wm.inset[0] = v as f32);
            let inset_y = scalar_row(form, &numbers, (salt, "inset-y"), &labels.inset_y, wm.inset[1], INSET_RANGE, 0.5, down);
            apply(&mut event, wm, inset_y, |wm, v| wm.inset[1] = v as f32);

            if !graphic {
                form.spanning(|ui| {
                    let mut shadow = wm.shadow;
                    let edit = Checkbox::bool(labels.shadow.as_str(), &mut shadow).id_salt((salt, "shadow")).show(ui);
                    apply(&mut event, wm, edit.event.clone(), |wm, shadow| wm.shadow = shadow);
                    edit
                });
            }
        });
        EditResponse::new(inner.response, event)
    }
}

const ROTATIONS: [u16; 4] = [0, 90, 180, 270];
const SIZE_RANGE: (f64, f64) = (1.0, 15.0);
const OPACITY_RANGE: (f64, f64) = (5.0, 100.0);
const INSET_RANGE: (f64, f64) = (0.0, 40.0);

/// Applies a control's event to `wm` through `set` and keeps it as this pass's event, with the
/// whole watermark as its value.
fn apply<T>(slot: &mut Option<EditEvent<Watermark>>, wm: &mut Watermark, event: Option<EditEvent<T>>, set: impl FnOnce(&mut Watermark, T)) {
    if let Some(event) = event {
        *slot = Some(event.map(|value| {
            set(wm, value);
            wm.clone()
        }));
    }
}

/// A percent row: a number field and a slider on one range. Returns the event of whichever
/// changed (the field wins when both did).
#[allow(clippy::too_many_arguments)]
fn scalar_row(
    form: &mut crate::composites::Form<'_>,
    numbers: &LocaleNumbers,
    salt: (egui::IdSalt, &'static str),
    label: &str,
    value: f32,
    range: (f64, f64),
    step: f64,
    enabled: bool,
) -> Option<EditEvent<f64>> {
    let value = f64::from(value);
    form.row(label, |ui, _| {
        let field = NumberField::new((salt, "field"), label, value, numbers)
            .range(range.0..=range.1)
            .step(step)
            .decimals(0..=1)
            .unit(Unit::new("{n}%"))
            .enabled(enabled)
            .show(ui);
        let slider = Slider::new((salt, "slider"), label, value, range.0..=range.1).step(step).enabled(enabled).show(ui);
        let event = field.event.clone().or(slider.event);
        (field.response.clone(), event)
    })
    .inner
    .1
}
