//! [`PictureFillEditor`].

use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use craft_host::{Host, PickId, PickRequest, Picked};
use craft_ui::composites::{Form, FormGrid, SliderRow};
use craft_ui::controls::{Button, Checkbox, NumberField, Select};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use egui::{AsIdSalt, Context, Id, IdSalt, Modifiers, Ui};

use super::{PictureFill, PictureFillEvent, PictureFillLabels, PictureMode, TileAlignment, TileMirror};

/// What [`PictureFillEditor::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies the edits and stores a picked picture"]
pub struct PictureFillOutput {
    /// What changed this frame, in order.
    pub events: Vec<PictureFillEvent>,
}

impl PictureFillOutput {
    /// Whether anything changed this pass (an edit or a picked picture): a paint chip's Picture
    /// tab redraws its preview.
    pub fn changed(&self) -> bool {
        !self.events.is_empty()
    }
}

/// PowerPoint's Picture or texture fill pane, in its order:
///
/// 1. "Picture source" with the source's name and "Insert…": picks an image through the
///    installed `Host`'s `Files` (purpose `"picture"`, image types) and reads it as a host job, as
///    `LutPicker` does; reports [`PictureFillEvent::Source`] on the pass it is read, or
///    `SourceFailed`. A cancelled pick reports nothing; without a `Host`, Insert… is disabled.
/// 2. "Tile picture as texture" check box (switches [`PictureMode`](super::PictureMode), keeping
///    each mode's last values for this editor).
/// 3. Tile: Offset X and Offset Y (points), Scale X and Scale Y (percent), Alignment (9 values),
///    Mirror type (None, Horizontal, Vertical, Both). Stretch: Offset left, top, right, bottom
///    (percent).
/// 4. "Rotate with shape" check box.
/// 5. "Transparency" slider and field, 0–100 %.
///
/// Every field reports [`PictureFillEvent::Edit`]. Keyboard: Tab through the fields in this order;
/// the grid mirrors right to left.
pub struct PictureFillEditor<'a> {
    id_salt: IdSalt,
    fill: &'a PictureFill,
    labels: Option<PictureFillLabels>,
}

impl<'a> PictureFillEditor<'a> {
    /// The editor of `fill`.
    pub fn new(id_salt: impl AsIdSalt, fill: &'a PictureFill) -> Self {
        PictureFillEditor { id_salt: IdSalt::new(id_salt), fill, labels: None }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PictureFillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the editor and reports the events.
    pub fn show(self, ui: &mut Ui) -> PictureFillOutput {
        let Self { id_salt, fill, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let labels = labels.unwrap_or_else(|| PictureFillLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let ctx = ui.ctx().clone();
        let host = Host::try_of(ui);
        let load_id = id.with("load");
        let mut events = poll_load(&ctx, host.as_ref(), load_id, &labels.unreadable);

        // Each mode keeps its last values while the other is on.
        let modes_id = id.with("modes");
        let mut modes = ctx.data(|d| d.get_temp::<Modes>(modes_id)).unwrap_or_default();
        match fill.mode {
            PictureMode::Stretch { offsets } => modes.stretch = offsets,
            PictureMode::Tile { offset, scale, alignment, mirror } => modes.tile = TileValues { offset, scale, alignment, mirror },
        }
        ctx.data_mut(|d| d.insert_temp(modes_id, modes));

        let mut out = Reported::default();
        let mut insert = false;
        FormGrid::new(id).show(ui, |form| {
            form.value(labels.source.clone(), fill.source.clone().unwrap_or_default());
            insert =
                form.unlabelled(|ui| ui.add(Button::new(labels.insert.clone()).enabled(host.is_some()).id_salt(id.with("insert")))).inner.clicked();

            let mut tile = matches!(fill.mode, PictureMode::Tile { .. });
            let check = form.unlabelled(|ui| Checkbox::bool(labels.tile.clone(), &mut tile).id_salt(id.with("tile")).show(ui)).inner;
            out.take(check, |on| {
                edited(fill, |f| {
                    f.mode = if on {
                        let t = modes.tile;
                        PictureMode::Tile { offset: t.offset, scale: t.scale, alignment: t.alignment, mirror: t.mirror }
                    } else {
                        PictureMode::Stretch { offsets: modes.stretch }
                    };
                })
            });

            match fill.mode {
                PictureMode::Tile { offset, scale, alignment, mirror } => {
                    let mut number = |form: &mut Form<'_>,
                                      key: &str,
                                      label: &str,
                                      value: f64,
                                      unit: &'static str,
                                      range: RangeInclusive<f64>,
                                      set: fn(&mut PictureMode, f64)| {
                        let edit = form
                            .row(label.to_owned(), |ui, _| {
                                NumberField::new(id.with(key), label.to_owned(), value, &numbers)
                                    .unit(Unit::new(unit))
                                    .range(range)
                                    .decimals(0..=1)
                                    .show(ui)
                            })
                            .inner;
                        out.take(edit, |v| edited(fill, |f| set(&mut f.mode, v)));
                    };
                    number(form, "offset-x", &labels.offset_x, offset[0], "{n} pt", OFFSET, |m, v| tile_set(m, |o, _| o[0] = v));
                    number(form, "offset-y", &labels.offset_y, offset[1], "{n} pt", OFFSET, |m, v| tile_set(m, |o, _| o[1] = v));
                    number(form, "scale-x", &labels.scale_x, scale[0], "{n} %", PictureMode::SCALE, |m, v| tile_set(m, |_, s| s[0] = v));
                    number(form, "scale-y", &labels.scale_y, scale[1], "{n} %", PictureMode::SCALE, |m, v| tile_set(m, |_, s| s[1] = v));

                    let mut value = alignment;
                    let edit = form
                        .row(labels.alignment.clone(), |ui, _| {
                            let mut select = Select::new(id.with("alignment"), labels.alignment.clone(), &mut value).wheel_steps(false);
                            for (option, text) in [
                                (TileAlignment::TopLeft, &labels.top_left),
                                (TileAlignment::Top, &labels.top),
                                (TileAlignment::TopRight, &labels.top_right),
                                (TileAlignment::Left, &labels.left),
                                (TileAlignment::Center, &labels.center),
                                (TileAlignment::Right, &labels.right),
                                (TileAlignment::BottomLeft, &labels.bottom_left),
                                (TileAlignment::Bottom, &labels.bottom),
                                (TileAlignment::BottomRight, &labels.bottom_right),
                            ] {
                                select = select.option(option, text.clone());
                            }
                            select.show(ui)
                        })
                        .inner;
                    if let Some(picked) = chosen(edit.event) {
                        out.set(edited(fill, |f| {
                            if let PictureMode::Tile { alignment, .. } = &mut f.mode {
                                *alignment = picked;
                            }
                        }));
                    }

                    let mut value = mirror;
                    let edit = form
                        .row(labels.mirror.clone(), |ui, _| {
                            let mut select = Select::new(id.with("mirror"), labels.mirror.clone(), &mut value).wheel_steps(false);
                            for (option, text) in [
                                (TileMirror::None, &labels.mirror_none),
                                (TileMirror::Horizontal, &labels.horizontal),
                                (TileMirror::Vertical, &labels.vertical),
                                (TileMirror::Both, &labels.both),
                            ] {
                                select = select.option(option, text.clone());
                            }
                            select.show(ui)
                        })
                        .inner;
                    if let Some(picked) = chosen(edit.event) {
                        out.set(edited(fill, |f| {
                            if let PictureMode::Tile { mirror, .. } = &mut f.mode {
                                *mirror = picked;
                            }
                        }));
                    }
                }
                PictureMode::Stretch { offsets } => {
                    // The order of the model and of DeckCraft's pane: left, top, right, bottom.
                    let rows = [&labels.offset_left, &labels.offset_top, &labels.offset_right, &labels.offset_bottom];
                    for (at, label) in rows.into_iter().enumerate() {
                        let edit = form
                            .row(label.clone(), |ui, _| {
                                NumberField::new(id.with(("offset", at)), label.clone(), offsets[at], &numbers)
                                    .unit(Unit::new("{n} %"))
                                    .decimals(0..=0)
                                    .show(ui)
                            })
                            .inner;
                        out.take(edit, |v| {
                            edited(fill, |f| {
                                if let PictureMode::Stretch { offsets } = &mut f.mode {
                                    offsets[at] = v;
                                }
                            })
                        });
                    }
                }
            }

            let mut rotate = fill.rotate_with_shape;
            let check = form.unlabelled(|ui| Checkbox::bool(labels.rotate.clone(), &mut rotate).id_salt(id.with("rotate")).show(ui)).inner;
            out.take(check, |v| edited(fill, |f| f.rotate_with_shape = v));

            let mut slider_edit = None;
            form.spanning(|ui| {
                let row = SliderRow::new(id.with("transparency"), labels.transparency.clone(), fill.transparency * 100.0, 0.0..=100.0, &numbers)
                    .unit(Unit::new("{n} %"))
                    .decimals(0)
                    .show(ui);
                let response = row.response.clone();
                slider_edit = row.edits().next();
                response
            });
            if let Some(event) = slider_edit {
                out.push(event.map(|v| edited(fill, |f| f.transparency = (v / 100.0).clamp(0.0, 1.0))));
            }
        });

        if insert && let Some(host) = &host {
            start_load(&ctx, host, load_id, labels.insert.trim_end_matches('…'));
        }
        events.extend(out.event.map(PictureFillEvent::Edit));
        PictureFillOutput { events }
    }
}

/// The values of each mode that is off, kept between passes.
#[derive(Clone, Copy)]
struct Modes {
    stretch: [f64; 4],
    tile: TileValues,
}

#[derive(Clone, Copy)]
struct TileValues {
    offset: [f64; 2],
    scale: [f64; 2],
    alignment: TileAlignment,
    mirror: TileMirror,
}

impl Default for Modes {
    fn default() -> Self {
        Modes {
            stretch: [0.0; 4],
            tile: TileValues { offset: [0.0; 2], scale: [100.0; 2], alignment: TileAlignment::default(), mirror: TileMirror::default() },
        }
    }
}

/// The first edit of a pass.
#[derive(Default)]
struct Reported {
    event: Option<EditEvent<PictureFill>>,
}

impl Reported {
    /// Keeps the first event of the pass, as the field's event turned into a fill.
    fn take<T>(&mut self, edit: craft_ui::edit::EditResponse<T>, fill: impl FnOnce(T) -> PictureFill) {
        if self.event.is_none() {
            self.event = edit.event.map(|event| event.map(fill));
        }
    }

    fn push(&mut self, event: EditEvent<PictureFill>) {
        if self.event.is_none() {
            self.event = Some(event);
        }
    }

    /// Keeps a one-shot change of a menu pick unless a field already reported.
    fn set(&mut self, value: PictureFill) {
        self.push(EditEvent::set(value, Modifiers::NONE));
    }
}

/// A copy of `fill` with `change` applied.
fn edited(fill: &PictureFill, change: impl FnOnce(&mut PictureFill)) -> PictureFill {
    let mut copy = fill.clone();
    change(&mut copy);
    copy
}

/// Applies `change` to a tile mode's offset and scale.
fn tile_set(mode: &mut PictureMode, change: impl FnOnce(&mut [f64; 2], &mut [f64; 2])) {
    if let PictureMode::Tile { offset, scale, .. } = mode {
        change(offset, scale);
    }
}

/// The value a menu pick chose: a pick is a `Set`, or the commit of the open list.
fn chosen<T>(event: Option<EditEvent<T>>) -> Option<T> {
    match event? {
        EditEvent::Set { value, .. } | EditEvent::Commit { value, changed: true, .. } => Some(value),
        _ => None,
    }
}

/// A picture read: its file name and bytes, or the message to show.
type Picture = Result<(String, Vec<u8>), String>;

/// Where Insert… is: the picker open, or the chosen file being read.
#[derive(Clone)]
enum Load {
    Picking(PickId),
    Reading(Arc<Mutex<Option<Picture>>>),
}

/// The purpose of the picture picker: its folder is remembered under it.
/// Offsets have no limit.
const OFFSET: RangeInclusive<f64> = f64::NEG_INFINITY..=f64::INFINITY;

const PICK_PURPOSE: &str = "picture";

/// Image types the picker offers.
const PICTURE_TYPES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff"];

/// Asks for the picture; a pick still open is dropped.
fn start_load(ctx: &Context, host: &Host, key: Id, title: &str) {
    if let Some(Load::Picking(old)) = ctx.data(|d| d.get_temp::<Load>(key)) {
        host.files().forget(old);
    }
    let request = PickRequest::open(PICK_PURPOSE).title(title).filter(title, PICTURE_TYPES);
    let id = host.files().pick(request);
    ctx.data_mut(|d| d.insert_temp(key, Load::Picking(id)));
}

/// Moves Insert… along: takes the picker's answer, reads the file as a job, and reports the
/// picture of a finished read (or why it failed). `unreadable` is the failure message with
/// `{name}`.
fn poll_load(ctx: &Context, host: Option<&Arc<Host>>, key: Id, unreadable: &str) -> Vec<PictureFillEvent> {
    let Some(host) = host else { return Vec::new() };
    let Some(load) = ctx.data(|d| d.get_temp::<Load>(key)) else { return Vec::new() };
    let finished = |result: Picture| match result {
        Ok((name, bytes)) => PictureFillEvent::Source { name, bytes },
        Err(message) => PictureFillEvent::SourceFailed(message),
    };
    match load {
        Load::Picking(id) => {
            let Some(picked) = host.files().take_answer(id) else { return Vec::new() };
            ctx.data_mut(|d| d.remove::<Load>(key));
            match picked {
                Picked::Cancelled => Vec::new(),
                Picked::Contents(contents) => contents.into_iter().next().map(|file| vec![finished(Ok(file))]).unwrap_or_default(),
                Picked::Paths(paths) => {
                    let Some(path) = paths.into_iter().next() else { return Vec::new() };
                    let slot = Arc::new(Mutex::new(None));
                    let (shared, wake, reader, template) = (Arc::clone(&slot), ctx.clone(), Arc::clone(host), unreadable.to_owned());
                    host.jobs().spawn(
                        "picture-load",
                        Box::new(move || {
                            let name = file_name(&path);
                            let result = reader
                                .files()
                                .read(&path)
                                .map(|bytes| (name.clone(), bytes))
                                .map_err(|_| craft_i18n::fmt(&template, &[("name", &name)]));
                            *shared.lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
                            wake.request_repaint();
                        }),
                    );
                    ctx.data_mut(|d| d.insert_temp(key, Load::Reading(slot)));
                    Vec::new()
                }
            }
        }
        Load::Reading(slot) => {
            let Some(done) = slot.lock().unwrap_or_else(PoisonError::into_inner).take() else { return Vec::new() };
            ctx.data_mut(|d| d.remove::<Load>(key));
            vec![finished(done)]
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}
