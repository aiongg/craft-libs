//! [`SliceOptionsDialog`] and [`DivideSlicesDialog`] (task SH-T9). PhotoCraft and VectorCraft
//! both have them with the same fields; what the fields mean at export is the app's.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::Context;

use super::SliceLabels;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, StackDialog, dialog_id};

/// What a slice holds at export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliceKind {
    #[default]
    Image,
    NoImage,
    /// Illustrator's HTML Text (Photoshop's Table).
    HtmlText,
}

/// A slice's options.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SliceOptions {
    pub kind: SliceKind,
    pub name: String,
    pub url: String,
    pub target: String,
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
pub struct SliceOptionsDialog {
    id: DialogId,
    options: SliceOptions,
    kinds: Vec<SliceKind>,
    labels: Option<SliceLabels>,
    state: DialogState,
}

impl SliceOptionsDialog {
    /// The dialog for `options`, offering Image and No Image (PhotoCraft's choices).
    pub fn new(options: SliceOptions) -> Self {
        SliceOptionsDialog {
            id: dialog_id("slice.options"),
            options,
            kinds: vec![SliceKind::Image, SliceKind::NoImage],
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
        todo!()
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
    pub rows: Option<DivideBy>,
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
    divide: DivideSlices,
    labels: Option<SliceLabels>,
    state: DialogState,
}

impl DivideSlicesDialog {
    /// The dialog starting from `divide`.
    pub fn new(divide: DivideSlices) -> Self {
        DivideSlicesDialog { id: dialog_id("slice.divide"), divide, labels: None, state: DialogState::new() }
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
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
        todo!()
    }
}
