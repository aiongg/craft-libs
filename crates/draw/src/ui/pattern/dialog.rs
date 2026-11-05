//! [`PatternFillDialog`].

use std::sync::Arc;

use egui::{Context, Painter, Rect};

use super::{PatternFillEditor, PatternFillLabels, PatternFillOptions, PatternSwatch};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width, dialog_id};
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;

/// The app's painter of a swatch's tile, shared with the dialog on the host.
type Thumbnails = dyn Fn(&Painter, Rect, &PatternSwatch) + Send + Sync;

/// Photoshop's Pattern Fill dialog (Layer › New Fill Layer › Pattern…, a Pattern Fill layer's
/// thumbnail, Layer Content Options): [`PatternFillEditor`](super::PatternFillEditor) in bitmap
/// mode with OK and Cancel. No Preview check box: the app previews live from
/// [`current`](Self::current) while [`changed`](Self::changed) says the values moved this pass
/// (PhotoCraft's proxy preview), or with `Preview` and an outer gesture.
///
/// OK answers the options; OK with the values it opened with answers them too. When the dialog
/// edits an existing fill the app makes no step for an unchanged edit ([`unchanged`](Self::unchanged));
/// a new fill layer is made whatever the values (PhotoCraft). Cancel answers nothing. Enter is OK,
/// Esc Cancel.
pub struct PatternFillDialog {
    id: DialogId,
    initial: PatternFillOptions,
    current: PatternFillOptions,
    swatches: Vec<PatternSwatch>,
    thumbnails: Option<Arc<Thumbnails>>,
    /// The pass the values last changed in.
    changed: Option<u64>,
    /// The pass the dialog last drew in.
    pass: u64,
    /// The values before the running gesture, which a cancelled gesture restores.
    gesture_start: Option<PatternFillOptions>,
    labels: Option<PatternFillLabels>,
    state: DialogState<PatternFillOptions>,
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
            pass: 0,
            gesture_start: None,
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
        self.changed == Some(self.pass)
    }

    /// Whether the current values equal the ones it opened with. Only the edit of an existing fill
    /// (PhotoCraft's SET) skips its history step on this; creating a new fill layer ignores it.
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
        // pc: crates/ui-egui/src/dialogs/pattern_fill_ui.rs:52-73 `open`, `open_new` (100 %, 0°, linked),
        // :98-151 (`unchanged`, `confirm`, `display_doc`: the live preview, an unchanged OK is a no-op), :159 `body`.
        let Self { id, current, swatches, thumbnails, changed, pass, gesture_start, labels, state, .. } = self;
        let words = labels.clone().unwrap_or_else(|| PatternFillLabels::of_ctx(ctx));
        *pass = ctx.cumulative_pass_nr();
        let pass = *pass;
        let shown = Dialog::with_id(*id, words.title.as_str()).stack(stack).width(Width::Compact).footer(Footer::standard()).show(
            ctx,
            state,
            current,
            |ui, fields| {
                let mut editor = PatternFillEditor::new("pattern", fields, swatches).labels(words.clone());
                if let Some(paint) = thumbnails.clone() {
                    editor = editor.thumbnails(move |painter, rect, swatch| paint(painter, rect, swatch));
                }
                let Some(event) = editor.show(ui).event else { return };
                match &event {
                    EditEvent::Begin { .. } => {
                        gesture_start.get_or_insert_with(|| fields.clone());
                    }
                    EditEvent::Cancel { .. } => {
                        if let Some(start) = gesture_start.take() {
                            *fields = start;
                            *changed = Some(pass);
                        }
                        return;
                    }
                    EditEvent::Commit { .. } | EditEvent::Set { .. } => *gesture_start = None,
                    _ => {}
                }
                if let Some(value) = event.value().filter(|value| *value != fields) {
                    *fields = value.clone();
                    *changed = Some(pass);
                }
            },
        );
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(current.clone())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}
