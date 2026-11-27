//! `AppPaint`: a picture the app paints itself inside a framework control, which places it and draws its own parts
//! around it.

use crate::contract::{MemoKey, MemoProps};
use crate::primitives::ContentPainterCx;

type AppPaintFn<'a, A> = Box<dyn FnMut(&ContentPainterCx<'_>, A) + 'a>;

/// A closure that paints the app's picture where a control places it: the control calls it from its paint with its
/// content painter and the placement `A` (the [`Navigator`](super::Navigator)'s fitted rect, the
/// [`RasterDisplay`](super::RasterDisplay)'s canvas view). The app adds what it likes to the painter, a GPU paint
/// callback included; the control's own parts paint before and after it.
///
/// The closure is not data the component's key can read. A painter with a [`revision`](Self::revision) keeps what it
/// painted until the revision moves; one without runs at every build of its scope.
pub struct AppPaint<'a, A> {
    paint: AppPaintFn<'a, A>,
    revision: Option<u64>,
}

impl<'a, A> AppPaint<'a, A> {
    /// A painter that draws the picture with `paint`, given the content painter and the placement.
    pub fn new(paint: impl FnMut(&ContentPainterCx<'_>, A) + 'a) -> Self {
        AppPaint { paint: Box::new(paint), revision: None }
    }

    /// The painter marked as painting the same picture while `revision` stays: the app moves it when the picture changes.
    #[must_use]
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// The same painter lent for one run of a body that may run more than once per pass.
    pub fn reborrowed(&mut self) -> AppPaint<'_, A> {
        let (paint, revision) = (&mut self.paint, self.revision);
        AppPaint { paint: Box::new(move |painter, at| paint(painter, at)), revision }
    }

    /// Paints the picture with `painter` at `at`.
    pub fn paint(&mut self, painter: &ContentPainterCx<'_>, at: A) {
        (self.paint)(painter, at);
    }
}

impl<A> MemoProps for AppPaint<'_, A> {
    fn key(&self, key: &mut MemoKey) {
        static UNVERSIONED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        match self.revision {
            Some(revision) => (1u8, revision).key(key),
            None => (0u8, UNVERSIONED.fetch_add(1, std::sync::atomic::Ordering::Relaxed)).key(key),
        }
    }
}
