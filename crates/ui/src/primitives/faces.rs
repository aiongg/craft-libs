//! Document faces for content painters.

use super::content::ContentPainterCx;
use crate::contract::FacesCx;
use crate::fonts::document::DocumentFaces;
use crate::fonts::{FaceBytes, FaceState};

impl FacesCx for ContentPainterCx<'_> {
    fn document_face(&self, key: &str, load: impl FnOnce() -> Option<FaceBytes>) -> FaceState {
        DocumentFaces::face(self.painter.ctx(), key, load)
    }
}
