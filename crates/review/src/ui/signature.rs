//! The signature pad dialog and saved signatures (N5.04).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::{Context, Painter, Rect};

use crate::signature::{SavedSignature, SignatureDraft};

craft_ui::words! {
    /// The pad's words, English by default (PdfCraft's, Acrobat's).
    #[non_exhaustive]
    pub struct SignatureLabels {
        create: "signature.create" = "Create {what}";
        change: "signature.change" = "Change {what}";
        signature: "signature.signature" = "signature";
        initials: "signature.initials" = "initials";
        type_tab: "signature.typeTab" = "Type";
        draw_tab: "signature.drawTab" = "Draw";
        image_tab: "signature.imageTab" = "Image";
        scan_tab: "signature.scanTab" = "Scan";
        type_hint: "signature.typeHint" = "Type text";
        image_hint: "signature.imageHint" = "Choose a PNG or JPEG image (up to 4 MiB). Transparency is preserved.";
        browse: "signature.browse" = "Browse…";
        undo: "signature.undo" = "Undo";
        clear: "signature.clear" = "Clear";
        apply: "signature.apply" = "Apply";
        cancel: "signature.cancel" = "Cancel";
    }
}

/// The saved signature and initials, one settings section shared by every document.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SignatureSettings {
    pub signature: Option<SavedSignature>,
    pub initials: Option<SavedSignature>,
}

impl craft_host::Section for SignatureSettings {
    const KEY: &'static str = "signatures";
}

/// Draws a saved signature into `rect` in `color`: strokes scaled from pad units, typed text in
/// the script face fitted to the rect, an image fitted keeping its aspect. Textures for typed
/// and image signatures are cached by the painter's context under the signature's hash.
// pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:406 `paint_saved`, `:309` `script_preview`.
pub fn paint_signature(painter: &Painter, rect: Rect, signature: &SavedSignature, color: egui::Color32) {
    todo!()
}

/// The Scan tab's content, the app's.
pub type ScanTab = Box<dyn FnMut(&mut egui::Ui, &mut SignatureDraft) + Send>;

/// Acrobat's signature pad on the dialog host: "Create signature" / "Change initials", the tabs
/// Type (a field, previewed in the script face), Draw (a pad: strokes with the pointer or pen,
/// Undo), Image (Browse… a PNG or JPEG up to [`MAX_IMAGE_BYTES`](crate::signature::MAX_IMAGE_BYTES)
/// through `host.files()`) and Scan (shown when the app gives a scan hook); Clear; Apply (enabled
/// when [`SignatureDraft::ready`]) and Cancel. Tabs are a segmented control (arrow keys move);
/// Tab walks the controls; Enter is Apply; Esc is Cancel.
pub struct SignaturePadDialog {
    draft: SignatureDraft,
    scan: Option<ScanTab>,
    labels: Option<SignatureLabels>,
}

impl SignaturePadDialog {
    pub fn new(draft: SignatureDraft) -> Self {
        Self { draft, scan: None, labels: None }
    }

    /// The Scan tab's content (PdfCraft's camera and clean-up): it fills `draft.image`.
    pub fn scan(mut self, tab: impl FnMut(&mut egui::Ui, &mut SignatureDraft) + Send + 'static) -> Self {
        self.scan = Some(Box::new(tab));
        self
    }

    pub fn labels(mut self, labels: SignatureLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl StackDialog for SignaturePadDialog {
    /// The saved signature, and whether it is initials.
    type Result = (SavedSignature, bool);

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("signature-pad")
    }

    // pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:598 `signature_pad`, `:732` `pad_buttons`,
    // `fill_sign/scan.rs:204` `scan_tab`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<Self::Result>> {
        todo!()
    }
}
