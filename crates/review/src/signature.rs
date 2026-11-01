//! Saved signatures and initials, and the signature pad's draft (N5.04).

/// The largest image file the Image tab takes: 4 MiB (PdfCraft's "Choose a PNG or JPEG image
/// (up to 4 MiB)").
pub const MAX_IMAGE_BYTES: usize = 4 << 20;

/// A signature image: straight RGBA, transparency kept.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignatureImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A signature or initials kept for reuse.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SavedSignature {
    /// Strokes in pad units (0…1 on both axes of the pad's box).
    Drawn(Vec<Vec<[f32; 2]>>),
    /// Text drawn in the script face.
    Typed(String),
    Image(SignatureImage),
}

/// The pad's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SignatureTab {
    #[default]
    Type,
    Draw,
    Image,
    /// A photo of a signature on paper, cleaned into an image by the app (PdfCraft's scan
    /// worker).
    Scan,
}

/// What the pad holds while open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SignatureDraft {
    pub tab: SignatureTab,
    pub strokes: Vec<Vec<[f32; 2]>>,
    pub text: String,
    /// The Image or Scan tab's result.
    pub image: Option<SignatureImage>,
    /// Creating initials (else the signature).
    pub initials: bool,
    /// Replacing a saved one (the title says Change).
    pub editing: bool,
}

impl SignatureDraft {
    /// A draft for `name`: the Type tab with the name, or for initials the first letter of each
    /// word of it.
    // pdfcraft: crates/ui-egui/src/tools/fill_sign.rs:97 `SigDraft::new`.
    pub fn new(initials: bool, name: &str) -> Self {
        let text = if initials {
            name.split_whitespace().filter_map(|word| word.chars().next()).collect()
        } else {
            name.trim().to_string()
        };
        Self { text, initials, ..Self::default() }
    }

    /// Whether Apply is enabled: the current tab has something (text that isn't blank, a stroke,
    /// an image).
    pub fn ready(&self) -> bool {
        match self.tab {
            SignatureTab::Type => !self.text.trim().is_empty(),
            SignatureTab::Draw => self.strokes.iter().any(|stroke| stroke.len() > 1),
            SignatureTab::Image | SignatureTab::Scan => self.image.is_some(),
        }
    }

    /// Undo on the Draw tab: drop the last stroke.
    pub fn undo_stroke(&mut self) {
        self.strokes.pop();
    }

    /// Clear: strokes, text and image (and the scan).
    pub fn clear(&mut self) {
        self.strokes.clear();
        self.text.clear();
        self.image = None;
    }

    /// What Apply saves: the current tab's content; `None` when not [`ready`](Self::ready).
    pub fn saved(&self) -> Option<SavedSignature> {
        if !self.ready() {
            return None;
        }
        Some(match self.tab {
            SignatureTab::Type => SavedSignature::Typed(self.text.trim().to_string()),
            SignatureTab::Draw => SavedSignature::Drawn(self.strokes.clone()),
            SignatureTab::Image | SignatureTab::Scan => SavedSignature::Image(self.image.clone()?),
        })
    }
}
