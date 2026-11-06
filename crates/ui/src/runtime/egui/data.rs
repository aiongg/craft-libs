//! Runtime services stored in context data, one lock per service.

use egui::{Context, Id};
use std::sync::{Arc, Mutex, PoisonError};

/// A service's state in context data. Cloning shares the state.
///
/// The context's data lock is held only to fetch the handle. The service's own lock is held while `with`
/// runs its closure, so the closure runs no app code and makes no call that reaches the context (7.2, the
/// lock rule).
#[derive(Default)]
pub(crate) struct Service<S>(Arc<Mutex<S>>);

impl<S> Clone for Service<S> {
    fn clone(&self) -> Self {
        Service(Arc::clone(&self.0))
    }
}

impl<S: Default + Send + 'static> Service<S> {
    /// The context's service of type `S`, created on first use.
    pub(crate) fn of(ctx: &Context) -> Service<S> {
        ctx.data_mut(|data| data.get_temp_mut_or_default::<Service<S>>(Id::unique("craft_ui.service")).clone())
    }

    /// Runs `f` on the state under the service's lock.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Runs `f` on the context's service of type `S`.
pub(crate) fn with<S: Default + Send + 'static, R>(ctx: &Context, f: impl FnOnce(&mut S) -> R) -> R {
    Service::<S>::of(ctx).with(f)
}
