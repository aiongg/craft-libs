//! [`PatternFillDialog`] (task SH-T12).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use egui::{Context, Painter, Rect};

use super::{PatternFillLabels, PatternFillOptions, PatternSwatch};
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, StackDialog, dialog_id};

/// The app's painter of a swatch's tile, shared with the dialog on the host.
type Thumbnails = dyn Fn(&Painter, Rect, &PatternSwatch) + Send + Sync;

/// Photoshop's Pattern Fill dialog (Layer › New Fill Layer › Pattern…, a Pattern Fill layer's
/// thumbnail, Layer Content Options): [`PatternFillEditor`](super::PatternFillEditor) in bitmap
/// mode with OK and Cancel. No Preview check box: the app previews live from
/// [`current`](Self::current) while [`changed`](Self::changed) says the values moved this pass
/// (PhotoCraft's proxy preview), or with `Preview` and an outer gesture.
///
/// OK answers the options; OK with the values it opened with answers them too, and the app makes
/// no step for an unchanged edit (PhotoCraft's `unchanged`). Cancel answers nothing. Enter is OK,
/// Esc Cancel.
pub struct PatternFillDialog {
    id: DialogId,
    initial: PatternFillOptions,
    current: PatternFillOptions,
    swatches: Vec<PatternSwatch>,
    thumbnails: Option<Arc<Thumbnails>>,
    /// The pass the values last changed in.
    changed: Option<u64>,
    labels: Option<PatternFillLabels>,
    state: DialogState,
}

impl PatternFillDialog {
    /// The dialog editing `options`, offering `swatches`.
    pub fn new(options: PatternFillOptions, swatches: Vec<PatternSwatch>) -> Self {
        PatternFillDialog {
            id: dialog_id("patternFill"),
            initial: options.clone(),
            current: options,
            swatches,
            thumbnails: None,
            changed: None,
            labels: None,
            state: DialogState::new(),
        }
    }

    /// Paints a swatch's tile (see [`PatternFillEditor::thumbnails`](super::PatternFillEditor::thumbnails)).
    pub fn thumbnails(mut self, paint: impl Fn(&Painter, Rect, &PatternSwatch) + Send + Sync + 'static) -> Self {
        self.thumbnails = Some(Arc::new(paint));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PatternFillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The values the fields show now, for the app's live preview.
    pub fn current(&self) -> &PatternFillOptions {
        &self.current
    }

    /// Whether the values changed in the last pass the dialog drew (the app refreshes its preview).
    pub fn changed(&self) -> bool {
        // new: compares `changed` with the context's current pass.
        todo!()
    }

    /// Whether the current values equal the ones it opened with.
    pub fn unchanged(&self) -> bool {
        self.current == self.initial
    }
}

impl StackDialog for PatternFillDialog {
    type Result = PatternFillOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PatternFillOptions>> {
        // pc: crates/ui-egui/src/dialogs/pattern_fill_ui.rs:52-72 `open`, `open_new` (100 %, 0°, linked),
        // :100-157 (the live preview, Cancel changes nothing, an unchanged OK is a no-op), :159 `body`.
        todo!()
    }
}
