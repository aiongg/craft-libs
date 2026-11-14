//! [`PatternFillDialog`].

use super::editor::SwatchPainter;
use super::{PatternFillEditor, PatternFillOptions, PatternSwatch, PatternWords};
use craft_ui::contract::Bind;
use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog, dialog_id};
use craft_ui::patterns::form_dialogs::FormDialog;
use craft_ui::primitives::ContentPainterCx;
use craft_ui::runtime::env::Env;
use egui::{Rect, Ui};

/// Photoshop's Pattern Fill dialog (Layer, New Fill Layer, Pattern...; a Pattern Fill layer's thumbnail; Layer Content
/// Options): [`PatternFillEditor`] in bitmap mode with OK and Cancel. No Preview check box: the app previews live
/// from [`current`](Self::current) while [`changed`](Self::changed) says the values moved this pass (PhotoCraft's
/// proxy preview).
///
/// OK answers the options; OK with the values it opened with answers them too. When the dialog edits an existing fill
/// the app makes no step for an unchanged edit ([`unchanged`](Self::unchanged)); a new fill layer is made whatever
/// the values (PhotoCraft). Cancel answers nothing. Enter is OK, Esc Cancel.
pub struct PatternFillDialog {
    id: DialogId,
    initial: PatternFillOptions,
    current: PatternFillOptions,
    swatches: Vec<PatternSwatch>,
    thumbnails: Option<SwatchPainter>,
    changed: bool,
    canvas: Option<egui::Id>,
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
            changed: false,
            canvas: None,
        }
    }

    /// Paints a swatch's tile (see [`PatternFillEditor::thumbnails`]).
    #[must_use]
    pub fn thumbnails(mut self, paint: impl Fn(&ContentPainterCx<'_>, Rect, &PatternSwatch) + Send + Sync + 'static) -> Self {
        self.thumbnails = Some(SwatchPainter::new(paint));
        self
    }

    /// The canvas whose primary drag moves the pattern while Link with Layer is on: the body passes
    /// that gesture to it each pass.
    #[must_use]
    pub fn canvas(mut self, canvas: egui::Id) -> Self {
        self.canvas = Some(canvas);
        self
    }

    /// Sets the pattern's phase, as the app's canvas drag does (M-12).
    pub fn set_phase(&mut self, phase: [f64; 2]) {
        let _ = phase;
        todo!("dl.K38")
    }

    /// The values the fields show now, for the app's live preview.
    pub fn current(&self) -> &PatternFillOptions {
        &self.current
    }

    /// Whether the values changed in the last pass the dialog drew (the app refreshes its preview).
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Whether the current values equal the ones it opened with. Only the edit of an existing fill skips its history
    /// step on this; creating a new fill layer ignores it.
    pub fn unchanged(&self) -> bool {
        self.current == self.initial
    }
}

impl StackDialog for PatternFillDialog {
    type Result = PatternFillOptions;

    fn json_fields(&self) -> Option<serde_json::Value> {
        todo!("dl.K36")
    }

    fn set_json_field(&mut self, ctx: &egui::Context, field: &str, value: serde_json::Value) -> Result<(), String> {
        let _ = (ctx, field, value);
        todo!("dl.K36")
    }

    fn confirm(&mut self, ctx: &egui::Context) -> Option<Self::Result> {
        let _ = ctx;
        todo!("dl.K36")
    }

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<PatternFillOptions>> {
        let words = Env::of(ui).words::<PatternWords>();
        let PatternFillDialog { current, swatches, thumbnails, changed, canvas, .. } = self;
        let _ = canvas; // P3(dialogs-w13): dl.K38 pass_canvas_gesture while Link with Layer is on.
        let before = current.clone();
        let shown = FormDialog::new(words.title.clone(), Bind::bound(&mut *current)).show(ui, |ui, _, value| {
            let mut editor = PatternFillEditor::new(words.pattern.clone(), Bind::bound(value), swatches);
            if let Some(painter) = thumbnails.clone() {
                editor = editor.thumbnails(move |cx, rect, swatch| painter.paint(cx, rect, swatch));
            }
            editor.show(ui);
            true
        });
        *changed = *current != before;
        match (shown.accepted, shown.closed) {
            (Some(options), _) => Some(DialogResult::Ok(options)),
            (None, Some(_)) => Some(DialogResult::Cancel),
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some("patternFill")
    }
}
