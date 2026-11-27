//! Document faces on the contexts a component sees, and on a build's `Ui` (text it draws with `Ui::draw`).

use super::{Component, Cx, MeasureCx, Ui};
use crate::fonts::document;
use crate::fonts::{FaceBytes, FaceState};
use crate::primitives::FacesCx;

impl<C: Component> FacesCx for Cx<'_, C> {
    fn document_face<L>(&self, key: &str, job: impl FnOnce() -> L) -> FaceState
    where
        L: FnOnce() -> Option<FaceBytes> + Send + 'static,
    {
        document::face(self.core.context(), key, job)
    }

    fn document_face_bytes(&self, key: &str) -> Option<FaceBytes> {
        document::bytes(self.core.context(), key)
    }
}

impl<C: Component> FacesCx for MeasureCx<'_, C> {
    fn document_face<L>(&self, key: &str, job: impl FnOnce() -> L) -> FaceState
    where
        L: FnOnce() -> Option<FaceBytes> + Send + 'static,
    {
        document::face(&self.with_ui(|ui| ui.ctx().clone()), key, job)
    }

    fn document_face_bytes(&self, key: &str) -> Option<FaceBytes> {
        document::bytes(&self.with_ui(|ui| ui.ctx().clone()), key)
    }
}

impl FacesCx for Ui<'_, '_> {
    fn document_face<L>(&self, key: &str, job: impl FnOnce() -> L) -> FaceState
    where
        L: FnOnce() -> Option<FaceBytes> + Send + 'static,
    {
        document::face(self.ctx(), key, job)
    }

    fn document_face_bytes(&self, key: &str) -> Option<FaceBytes> {
        document::bytes(self.ctx(), key)
    }
}
