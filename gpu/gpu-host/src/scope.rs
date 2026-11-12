//! Error scopes: GPU work whose errors come back to the caller instead of the device's handler.
//!
//! wgpu reports an error outside any scope to the device's one uncaptured-error handler
//! ([`crate::DeviceHealth::watch`]), which marks the device faulted. Work whose failure the caller
//! can handle (creating a pipeline, allocating a large texture, building a compositor) runs inside
//! [`scoped`] or [`checked`] instead and gets a [`ScopeError`]. The caller then reports it to the
//! device's health, either as an observation that leaves the device usable
//! ([`crate::DeviceHealth::observe`]) or as a fault that retires it ([`crate::DeviceHealth::caught`]).
//!
//! Scopes are per thread with wgpu's `std` feature, so a scope never catches another thread's
//! errors, and it never replaces the device owner's handlers.

use std::fmt;

/// What kind of error a scope caught.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// wgpu's out-of-memory error: freeing memory and trying again can succeed.
    OutOfMemory,
    /// An internal error of wgpu or the driver (a shader the driver's compiler rejected).
    Internal,
    /// A validation error.
    Validation,
    /// The work panicked inside wgpu or the driver instead of reporting an error (native only).
    Panicked,
}

/// An error a scope caught: its kind, and what failed with wgpu's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeError {
    /// The kind of error.
    pub kind: ScopeKind,
    /// `<what>: <wgpu's message>`.
    pub message: String,
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ScopeError {}

/// What [`checked`] knows right after the call.
#[derive(Debug)]
pub enum Checked<T> {
    /// wgpu answered at once (native backends, WebGL): the value, or what went wrong.
    Done(Result<T, ScopeError>),
    /// The browser's WebGPU answers later: the value, possibly invalid, and where the answer
    /// arrives (`None` for success). Don't use the value until it has arrived. Native backends
    /// never answer late.
    Later(T, std::sync::mpsc::Receiver<Option<ScopeError>>),
}

/// The three scopes, pushed in this order and popped in reverse.
struct Scopes {
    oom: wgpu::ErrorScopeGuard,
    internal: wgpu::ErrorScopeGuard,
    validation: wgpu::ErrorScopeGuard,
}

impl Scopes {
    fn push(device: &wgpu::Device) -> Self {
        let oom = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        Self { oom, internal, validation }
    }

    /// Pop every scope (even after a failure, so none stays on the thread's stack) and the error
    /// they caught. An allocation that ran out of memory also makes later calls on it invalid, so
    /// the out-of-memory error is the cause and comes first, then internal, then validation.
    fn pop(self, what: &str) -> impl Future<Output = Option<ScopeError>> + 'static {
        let validation = self.validation.pop();
        let internal = self.internal.pop();
        let oom = self.oom.pop();
        let what = what.to_string();
        async move {
            let validation = validation.await;
            let internal = internal.await;
            let oom = oom.await;
            let (kind, e) = match (oom, internal, validation) {
                (Some(e), _, _) => (ScopeKind::OutOfMemory, e),
                (None, Some(e), _) => (ScopeKind::Internal, e),
                (None, None, Some(e)) => (ScopeKind::Validation, e),
                (None, None, None) => return None,
            };
            Some(ScopeError { kind, message: format!("{what}: {e}") })
        }
    }
}

/// Run `f` with out-of-memory, internal and validation errors, and panics out of wgpu or the
/// driver, turned into an `Err` instead of reaching the device's handler. Native only: the
/// browser answers a scope later ([`checked`]).
///
/// Native wgpu-core resolves a popped scope at once, so the wait for the answer never waits for
/// other submitted GPU work.
#[cfg(not(target_arch = "wasm32"))]
pub fn scoped<T>(device: &wgpu::Device, what: &str, f: impl FnOnce() -> T) -> Result<T, ScopeError> {
    let scopes = Scopes::push(device);
    // Some native drivers unwind instead of reporting an error. The guard is inside the scopes, so
    // every scope is still popped after a panic.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    if let Some(error) = pollster::block_on(scopes.pop(what)) {
        return Err(error);
    }
    result.map_err(|payload| {
        let reason = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("the native backend panicked");
        ScopeError { kind: ScopeKind::Panicked, message: format!("{what}: {reason}") }
    })
}

/// [`scoped`] for work whose errors may arrive later: never takes an unanswered scope as success.
/// Native backends always answer at once ([`Checked::Done`]).
#[cfg(not(target_arch = "wasm32"))]
pub fn checked<T>(device: &wgpu::Device, what: &str, f: impl FnOnce() -> T) -> Checked<T> {
    Checked::Done(scoped(device, what, f))
}

/// [`scoped`] for work whose errors may arrive later: never takes an unanswered scope as success.
/// The browser's WebGPU may answer after this returns ([`Checked::Later`]).
#[cfg(target_arch = "wasm32")]
pub fn checked<T>(device: &wgpu::Device, what: &str, f: impl FnOnce() -> T) -> Checked<T> {
    use std::task::{Context, Poll, Waker};
    let scopes = Scopes::push(device);
    let out = f();
    let mut answer = Box::pin(scopes.pop(what));
    match answer.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(Some(e)) => Checked::Done(Err(e)),
        Poll::Ready(None) => Checked::Done(Ok(out)),
        Poll::Pending => {
            let (tx, rx) = std::sync::mpsc::channel();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = tx.send(answer.await);
            });
            Checked::Later(out, rx)
        }
    }
}
