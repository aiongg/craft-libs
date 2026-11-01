//! [`PatternFillEditor`] (task SH-T12).

use craft_color::ui::ColorPopover;
use craft_theme::TypeRole;
use craft_ui::composites::FormGrid;
use craft_ui::controls::{AngleDial, Button, Checkbox, NumberField};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::presets::picker::{PickerField, PickerThumb, PresetPicker, PresetPickerLabels};
use craft_ui::presets::{CellShape, PresetBrowser, PresetEvent, PresetRow, PresetState, PresetStyle, PresetView, ThumbInfo};
use craft_ui::primitives::Content;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{Align2, AsIdSalt, IdSalt, Label, Modifiers, Painter, Rect, Ui};

use super::{PatternFillLabels, PatternFillOptions, PatternSwatch};

/// The app's painter of a swatch's tile.
type Thumbnails<'a> = dyn FnMut(&Painter, Rect, &PatternSwatch) + 'a;

/// One pattern fill editor body.
///
/// Bitmap mode ([`PatternFillOptions::colors`] `None`, Photoshop's Pattern Fill):
/// - The pattern field: S6a-presets' `PresetPicker` (a 40 × 40 thumbnail field opening a grid of
///   every swatch, names in tooltips; "No patterns" when the list is empty), with "Snap to Origin"
///   beside it, which sets the phase to (0, 0).
/// - "Angle": `AngleDial` row, −180 to 180. "Scale": a percent `NumberField`,
///   [`PatternFillOptions::SCALE`]. "Link with Layer" check box.
///
/// Preset mode (`colors` set, PowerPoint's Pattern fill): a grid of the swatches (each painted by
/// the host in the two colours; the selected one and the hovered or focused one ringed in
/// `accent.solid`), then Foreground and Background colour chips.
///
/// Every field reports through one [`EditResponse<PatternFillOptions>`]: a pick, a check box,
/// Snap to Origin and a typed value are `Set`; a scrub or a dial drag is a gesture. Keyboard: Tab
/// through the fields; the grid takes arrows. Right to left the rows mirror.
pub struct PatternFillEditor<'a> {
    id_salt: IdSalt,
    options: &'a PatternFillOptions,
    swatches: &'a [PatternSwatch],
    thumbnails: Option<Box<Thumbnails<'a>>>,
    labels: Option<PatternFillLabels>,
}

impl<'a> PatternFillEditor<'a> {
    /// The editor of `options`, offering `swatches`.
    pub fn new(id_salt: impl AsIdSalt, options: &'a PatternFillOptions, swatches: &'a [PatternSwatch]) -> Self {
        PatternFillEditor { id_salt: IdSalt::new(id_salt), options, swatches, thumbnails: None, labels: None }
    }

    /// Paints a swatch's tile into a rect (the picker's cells and field, the preset grid). Without
    /// it the cells show the swatch's name.
    pub fn thumbnails(mut self, paint: impl FnMut(&Painter, Rect, &PatternSwatch) + 'a) -> Self {
        self.thumbnails = Some(Box::new(paint));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PatternFillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<PatternFillOptions> {
        let Self { id_salt, options, swatches, mut thumbnails, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let labels = labels.unwrap_or_else(|| PatternFillLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let name_color = Content::Normal.color(&tokens).to_egui();
        let name_format = role_format(&tokens, TypeRole::Label, name_color);
        let mut paint = |painter: &Painter, rect: Rect, swatch: &PatternSwatch| match thumbnails.as_mut() {
            Some(paint) => paint(painter, rect, swatch),
            None => {
                let galley = painter.layout_job(LayoutJob::single_section(swatch.name.clone(), name_format.clone()));
                let at = Align2::CENTER_CENTER.align_size_within_rect(galley.size(), rect).min;
                painter.with_clip_rect(rect).galley(at, galley, name_color);
            }
        };
        let rows: Vec<PresetRow<String>> = swatches.iter().map(|s| PresetRow::item(s.id.clone(), s.name.clone(), 0)).collect();
        let mut out = Reported::default();
        let grid = FormGrid::new(id).show(ui, |form| {
            if options.colors.is_none() {
                let mut picked = None;
                let mut snapped = false;
                form.unlabelled(|ui| {
                    if swatches.is_empty() {
                        let muted = role_format(&Tokens::of(ui), TypeRole::Body, Content::Muted.color(&Tokens::of(ui)).to_egui());
                        let response = ui.add(Label::new(LayoutJob::single_section(labels.no_patterns.clone(), muted)));
                        return (response.clone(), response);
                    }
                    let mut words = PresetPickerLabels::of(ui);
                    words.field = labels.pattern_named.clone();
                    let picker = PresetPicker::new(id.with("picker"), labels.pattern.clone(), &rows)
                        .selected(Some(&options.pattern))
                        .field(PickerField::Thumbnail(PickerThumb::Square))
                        .labels(words)
                        .show(ui, |painter, rect, info: ThumbInfo<'_, String>| {
                            if let Some(swatch) = swatches.iter().find(|s| s.id == *info.key) {
                                paint(painter, rect, swatch);
                            }
                        });
                    picked = picker.picked;
                    let snap = ui.add(Button::new(labels.snap_to_origin.clone()).id_salt(id.with("snap")));
                    snapped = snap.clicked();
                    (picker.response, snap)
                });
                if let Some(pattern) = picked.filter(|p| *p != options.pattern) {
                    out.set(edited(options, |o| o.pattern = pattern));
                }
                if snapped {
                    out.set(edited(options, |o| o.phase = [0.0, 0.0]));
                }
                let angle = form
                    .row(labels.angle.clone(), |ui, _| {
                        AngleDial::new(id.with("angle"), labels.angle.clone(), options.angle, &numbers).range(-180.0..=180.0).show(ui)
                    })
                    .inner;
                out.take(angle, |v| edited(options, |o| o.angle = v.clamp(-180.0, 180.0)));
                let scale = form
                    .row(labels.scale.clone(), |ui, _| {
                        NumberField::new(id.with("scale"), labels.scale.clone(), options.scale, &numbers)
                            .range(PatternFillOptions::SCALE)
                            .unit(Unit::new("{n} %"))
                            .decimals(0..=2)
                            .show(ui)
                    })
                    .inner;
                out.take(scale, |v| edited(options, |o| o.scale = v));
                let mut link = options.link;
                let check = form.unlabelled(|ui| Checkbox::bool(labels.link.clone(), &mut link).id_salt(id.with("link")).show(ui)).inner;
                out.take(check, |v| edited(options, |o| o.link = v));
            } else {
                let memory = id.with("grid-state");
                let mut state = form.ctx().data(|d| d.get_temp::<PresetState<String>>(memory)).unwrap_or_default();
                let mut picked = None;
                form.spanning(|ui| {
                    let style = PresetStyle::from_tokens(&Tokens::of(ui));
                    state.thumb = style.picker_cell;
                    let browser = PresetBrowser::new(id.with("grid"), labels.pattern.clone(), &mut state, &rows)
                        .selected(Some(&options.pattern))
                        .cell_shape(CellShape::Square)
                        .thumb_range(style.picker_cell..=style.picker_cell)
                        .view_lock(PresetView::Grid)
                        .context_menu(false)
                        .shrink_to_content(true)
                        .max_height(style.popup_max_height)
                        .show(ui, |painter, rect, info: ThumbInfo<'_, String>| {
                            if let Some(swatch) = swatches.iter().find(|s| s.id == *info.key) {
                                paint(painter, rect, swatch);
                            }
                        });
                    for event in browser.events {
                        if let PresetEvent::Select { key, .. } | PresetEvent::Activate { key, .. } = event {
                            picked = Some(key);
                        }
                    }
                    browser.response
                });
                form.ctx().data_mut(|d| d.insert_temp(memory, state));
                if let Some(pattern) = picked.filter(|p| *p != options.pattern) {
                    out.set(edited(options, |o| o.pattern = pattern));
                }
                if let Some(colors) = options.colors {
                    let foreground = form
                        .row(labels.foreground.clone(), |ui, _| {
                            ColorPopover::new(id.with("foreground"), labels.foreground.clone(), colors.foreground, &numbers).show(ui).edit
                        })
                        .inner;
                    out.take(foreground, |v| edited(options, |o| set_colors(o, |c| c.foreground = v.color)));
                    let background = form
                        .row(labels.background.clone(), |ui, _| {
                            ColorPopover::new(id.with("background"), labels.background.clone(), colors.background, &numbers).show(ui).edit
                        })
                        .inner;
                    out.take(background, |v| edited(options, |o| set_colors(o, |c| c.background = v.color)));
                }
            }
        });
        EditResponse::new(grid.response, out.event).submitted(out.submitted).escaped(out.escaped)
    }
}

/// The first event of a pass and the submit and Esc flags of every field.
#[derive(Default)]
struct Reported {
    event: Option<EditEvent<PatternFillOptions>>,
    submitted: bool,
    escaped: bool,
}

impl Reported {
    /// Keeps the first event of the pass, as `value` of the field's edit turned into options.
    fn take<T>(&mut self, edit: EditResponse<T>, options: impl FnOnce(T) -> PatternFillOptions) {
        self.submitted |= edit.submitted;
        self.escaped |= edit.escaped;
        if self.event.is_none() {
            self.event = edit.event.map(|event| event.map(options));
        }
    }

    /// Keeps a one-shot change of a pick or a button unless a field already reported.
    fn set(&mut self, value: PatternFillOptions) {
        if self.event.is_none() {
            self.event = Some(EditEvent::set(value, Modifiers::NONE));
        }
    }
}

/// A copy of `options` with `change` applied.
fn edited(options: &PatternFillOptions, change: impl FnOnce(&mut PatternFillOptions)) -> PatternFillOptions {
    let mut copy = options.clone();
    change(&mut copy);
    copy
}

fn set_colors(options: &mut PatternFillOptions, change: impl FnOnce(&mut super::PatternColors)) {
    if let Some(colors) = options.colors.as_mut() {
        change(colors);
    }
}
