//! [`SliceOptionsDialog`] and [`DivideSlicesDialog`]. PhotoCraft and VectorCraft
//! both have them with the same fields; what the fields mean at export is the app's.

use std::cell::Cell;
use std::sync::Arc;

use craft_theme::{Rgba, Srgba};
use egui::{Context, Ui};

use super::SliceLabels;
use craft_ui::composites::{Form, FormGrid};
use craft_ui::controls::{Button, Checkbox, ControlSize, NumberField, Select, Swatch, TextField};
use craft_ui::dialog::{
    ColorRequest, Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width, dialog_id,
};
use craft_ui::localized::{LocaleNumbers, SharedLabels as _};
use craft_ui::primitives::Text;

/// What a slice holds at export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliceKind {
    /// The slice exports as an image.
    #[default]
    Image,
    /// The slice exports without an image.
    NoImage,
    /// Illustrator's HTML Text (Photoshop's Table).
    HtmlText,
}

/// A slice's options.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SliceOptions {
    /// What the slice holds at export.
    pub kind: SliceKind,
    /// The slice's name.
    pub name: String,
    /// The URL the slice links to.
    pub url: String,
    /// The link's target frame.
    pub target: String,
    /// The message shown for the slice.
    pub message: String,
    /// Image slices only.
    pub alt: String,
    /// No Image slices only.
    pub cell_text: String,
    /// Whether the Background row shows (PhotoCraft's; VectorCraft has none).
    pub background_row: bool,
    /// The cell background; `None` for none.
    pub background: Option<Srgba>,
}

/// Slice Options for one slice: Slice Type (the [`kinds`](Self::kinds) offered), Name, URL,
/// Target, Message Text, then Alt Tag (Image) or Text (No Image, HTML Text), and the optional
/// Background row (None or a colour, asked for with `ColorRequest`). OK answers the options; Cancel
/// nothing. Fields in a `FormGrid`; Enter is OK, Esc Cancel.
///
/// The Background swatch (and the colour button of a row without one) asks for a colour: the app
/// takes [`pick_color`](Self::pick_color) after each pass, opens its picker and answers with
/// [`set_color`](Self::set_color). "None" clears the colour.
pub struct SliceOptionsDialog {
    id: DialogId,
    options: SliceOptions,
    kinds: Vec<SliceKind>,
    color_request: Option<ColorRequest>,
    labels: Option<SliceLabels>,
    state: DialogState,
}

impl SliceOptionsDialog {
    /// The key of the [`ColorRequest`] the Background row reports.
    pub const BACKGROUND_KEY: &'static str = "slice.options.background";

    /// The dialog for `options`, offering Image and No Image (PhotoCraft's choices).
    pub fn new(options: SliceOptions) -> Self {
        SliceOptionsDialog {
            id: dialog_id("slice.options"),
            options,
            kinds: vec![SliceKind::Image, SliceKind::NoImage],
            color_request: None,
            labels: None,
            state: DialogState::new(),
        }
    }

    /// The slice types offered (VectorCraft adds HTML Text).
    pub fn kinds(mut self, kinds: &[SliceKind]) -> Self {
        self.kinds = kinds.to_vec();
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The options as the fields show them now.
    pub fn options(&self) -> &SliceOptions {
        &self.options
    }

    /// The Background swatch was clicked in the pass since the last call: the colour to ask the
    /// app's picker for, keyed [`BACKGROUND_KEY`](Self::BACKGROUND_KEY). Returned once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.color_request.take()
    }

    /// The colour the app's picker returned for `key`; other keys are ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == Self::BACKGROUND_KEY {
            self.options.background = Some(color);
        }
    }

    /// The words: the given ones, else the installed ones.
    fn words(&self, ctx: &Context) -> Arc<SliceLabels> {
        self.labels.clone().map_or_else(|| SliceLabels::shared_ctx(ctx), Arc::new)
    }

    /// One text row.
    fn text_row(form: &mut Form<'_>, salt: &str, caption: &str, value: &mut String, focus: Option<egui::Id>) {
        form.row(caption, |ui, _| {
            let mut field = TextField::new(salt, caption, value.as_str()).size(ControlSize::Sm);
            if let Some(id) = focus {
                field = field.id(id);
            }
            let edit = field.show(ui);
            if let Some(text) = edit.event.as_ref().and_then(|event| event.value()) {
                value.clone_from(text);
            }
            edit
        });
    }

    /// The fields.
    fn body(&mut self, ui: &mut Ui, labels: &SliceLabels) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:370-387 `open_options` (the form's keys: kind, name,
        // url, target, message, alt, cellText, background).
        let name_id = self.id.with("name");
        FormGrid::new("slice-options").show(ui, |form| {
            form.row(labels.kind.as_str(), |ui, _| {
                let mut kind = self.options.kind;
                let mut select = Select::new("kind", labels.kind.as_str(), &mut kind).size(ControlSize::Sm);
                for &offered in &self.kinds {
                    let text = match offered {
                        SliceKind::Image => &labels.image,
                        SliceKind::NoImage => &labels.no_image,
                        SliceKind::HtmlText => &labels.html_text,
                    };
                    select = select.option(offered, text.as_str());
                }
                let edit = select.show(ui);
                self.options.kind = kind;
                edit
            });
            Self::text_row(form, "name", &labels.name, &mut self.options.name, Some(name_id));
            Self::text_row(form, "url", &labels.url, &mut self.options.url, None);
            Self::text_row(form, "target", &labels.target, &mut self.options.target, None);
            Self::text_row(form, "message", &labels.message, &mut self.options.message, None);
            if self.options.kind == SliceKind::Image {
                Self::text_row(form, "alt", &labels.alt, &mut self.options.alt, None);
            } else {
                Self::text_row(form, "cell-text", &labels.cell_text, &mut self.options.cell_text, None);
            }
            if self.options.background_row {
                form.row(labels.background.as_str(), |ui, _| {
                    ui.horizontal(|ui| {
                        let color = self.options.background.map_or(Rgba::TRANSPARENT, Rgba::from);
                        if ui.add(Swatch::new("background", labels.background.as_str(), color)).clicked() {
                            let start = self.options.background.unwrap_or(Srgba::new(1.0, 1.0, 1.0, 1.0));
                            self.color_request = Some(ColorRequest::new(Self::BACKGROUND_KEY, start).title(labels.background.as_str()));
                        }
                        match self.options.background {
                            Some(_) => {
                                ui.add(Text::new(Swatch::hex(color)));
                                if ui.add(Button::new(labels.background_none.as_str()).size(ControlSize::Sm)).clicked() {
                                    self.options.background = None;
                                }
                            }
                            None => {
                                ui.add(Text::new(labels.background_none.as_str()));
                            }
                        }
                    })
                    .response
                });
            }
        });
    }
}

impl StackDialog for SliceOptionsDialog {
    type Result = SliceOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<SliceOptions>> {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:370-387 `open_options` (kind image or noImage,
        // name, url, target, alt, message, cellText, background none or #rrggbb).
        // inv-vectorcraft: crates/ui-egui/src/dialogs/slices.rs:15-80 (Image, No Image, HTML Text).
        let labels = self.words(ctx);
        let name_id = self.id.with("name");
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id, labels.options_title.as_str())
            .stack(stack)
            .width(Width::Compact)
            .initial_focus(InitialFocus::Field(name_id))
            .footer(Footer::standard())
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels));
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.options.clone())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<SliceOptions> {
        Some(self.options.clone())
    }
}

/// How one axis of a slice is divided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DivideBy {
    /// Into this many slices, evenly spaced.
    Count(u32),
    /// Into slices this many pixels long (the last takes what is left).
    Size(u32),
}

/// Divide Slice's answer: rows (divided horizontally) and columns (vertically); `None` leaves
/// that axis.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DivideSlices {
    /// How to divide horizontally into rows, or `None` to leave rows.
    pub rows: Option<DivideBy>,
    /// How to divide vertically into columns, or `None` to leave columns.
    pub columns: Option<DivideBy>,
}

impl Default for DivideSlices {
    /// Two rows, columns off (PhotoCraft's `horizontal` 2, `vertical` 1).
    fn default() -> Self {
        DivideSlices { rows: Some(DivideBy::Count(2)), columns: None }
    }
}

/// Divide Slice: "Divide Horizontally Into" and "Divide Vertically Into", each a check box with
/// a choice of a count ("slices, evenly spaced") or a size ("pixels per slice"), counts and sizes
/// at least 1. OK answers [`DivideSlices`] (disabled when neither axis is on); Cancel nothing.
pub struct DivideSlicesDialog {
    id: DialogId,
    /// Rows, then columns.
    axes: [Axis; 2],
    labels: Option<SliceLabels>,
    state: DialogState,
}

/// One axis of the dialog: its check box, whether the value is a size, and the value. An axis that
/// is off keeps its value for when it is switched on.
#[derive(Clone, Copy, Debug)]
struct Axis {
    on: bool,
    by_size: bool,
    value: u32,
}

impl Axis {
    /// The most slices one axis divides into (PhotoCraft's `slice.divide` clamp).
    const MAX_COUNT: u32 = 1000;
    /// The largest slice size in pixels.
    const MAX_SIZE: u32 = 300_000;

    fn of(divide: Option<DivideBy>, off_value: u32) -> Self {
        match divide {
            Some(DivideBy::Count(n)) => Axis { on: true, by_size: false, value: n.clamp(1, Self::MAX_COUNT) },
            Some(DivideBy::Size(n)) => Axis { on: true, by_size: true, value: n.clamp(1, Self::MAX_SIZE) },
            None => Axis { on: false, by_size: false, value: off_value },
        }
    }

    fn answer(self) -> Option<DivideBy> {
        self.on.then_some(if self.by_size { DivideBy::Size(self.value) } else { DivideBy::Count(self.value) })
    }

    fn max(self) -> u32 {
        if self.by_size { Self::MAX_SIZE } else { Self::MAX_COUNT }
    }
}

impl DivideSlicesDialog {
    /// The dialog starting from `divide`.
    pub fn new(divide: DivideSlices) -> Self {
        DivideSlicesDialog {
            id: dialog_id("slice.divide"),
            axes: [Axis::of(divide.rows, 2), Axis::of(divide.columns, 1)],
            labels: None,
            state: DialogState::new(),
        }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The answer the fields give now; `None` while both axes are off.
    fn answer(&self) -> Option<DivideSlices> {
        let [rows, columns] = self.axes.map(Axis::answer);
        (rows.is_some() || columns.is_some()).then_some(DivideSlices { rows, columns })
    }

    /// The words: the given ones, else the installed ones.
    fn words(&self, ctx: &Context) -> Arc<SliceLabels> {
        self.labels.clone().map_or_else(|| SliceLabels::shared_ctx(ctx), Arc::new)
    }

    /// One axis: its check box, then (while it is on) the value and what the value counts.
    fn axis(&mut self, ui: &mut Ui, index: usize, caption: &str, labels: &SliceLabels, numbers: &LocaleNumbers) {
        let salt = if index == 0 { "rows" } else { "columns" };
        let mut axis = self.axes[index];
        let _on = Checkbox::bool(caption, &mut axis.on).size(ControlSize::Sm).id_salt((salt, "on")).show(ui);
        let unit = if axis.by_size { labels.size.as_str() } else { labels.count.as_str() };
        ui.add_enabled_ui(axis.on, |ui| {
            ui.horizontal(|ui| {
                let name = format!("{caption}, {unit}");
                let edit = NumberField::new((salt, "value"), name, f64::from(axis.value), numbers)
                    .range(1.0..=f64::from(axis.max()))
                    .decimals(0..=0)
                    .size(ControlSize::Sm)
                    .show(ui);
                if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                    axis.value = value.round().clamp(1.0, f64::from(axis.max())) as u32;
                }
                let both = format!("{caption}, {} / {}", labels.count, labels.size);
                let mut by_size = axis.by_size;
                let _by = Select::new((salt, "by"), both, &mut by_size)
                    .option(false, labels.count.as_str())
                    .option(true, labels.size.as_str())
                    .size(ControlSize::Sm)
                    .show(ui);
                axis.by_size = by_size;
                axis.value = axis.value.min(axis.max());
            });
        });
        self.axes[index] = axis;
    }
}

impl StackDialog for DivideSlicesDialog {
    type Result = DivideSlices;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<DivideSlices>> {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:389-402 `open_divide` (horizontal 2, vertical 1).
        // inv-vectorcraft: crates/ui-egui/src/dialogs/slices.rs:15-80 (rows and columns by count or
        // by size).
        let labels = self.words(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let valid = Cell::new(self.answer().is_some());
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id, labels.divide_title.as_str())
            .stack(stack)
            .width(Width::Compact)
            .footer(Footer::standard().ok_enabled_with(|| valid.get()))
            .show(ctx, &mut state, &mut (), |ui, ()| {
                self.axis(ui, 0, &labels.horizontally, &labels, &numbers);
                self.axis(ui, 1, &labels.vertically, &labels, &numbers);
                valid.set(self.answer().is_some());
            });
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.answer()?)),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<DivideSlices> {
        self.answer()
    }
}
